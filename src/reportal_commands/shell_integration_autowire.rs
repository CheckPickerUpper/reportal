//! Auto-wire that appends the `eval "$(rep init <shell>)"` block to
//! the user's shell rc file on first `rep` invocation, so users who
//! installed via `cargo install reportal` (or any route that bypasses
//! the shell installer in `installer/`) don't have to edit their rc
//! file by hand.
//!
//! This module also owns the load-check that distinguishes between
//! "the integration script ran in this exact shell" and "the
//! `REPORTAL_LOADED` env var was inherited from a parent shell that
//! had it loaded but I'm a child shell that never sourced an rc
//! file." Without that distinction, child shells (non-interactive
//! `zsh -c`, certain terminal-host setups, scripts) inherit the
//! marker, the binary thinks the user is fine, the auto-wire skips,
//! and the user types `rj` only to get an unexplained `command not
//! found`. The marker is now stamped with the shell's own PID; the
//! binary compares it against its parent process id and only treats
//! the integration as "loaded" when they match.

use std::env as environment;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use dirs as directories;

use crate::cli_args::InitializeShell;

/// Fenced marker opening the auto-managed integration block. Byte-
/// for-byte identical to the markers used by the shell installer
/// scripts in `installer/install.sh` and `installer/install.ps1`,
/// which makes both code paths interoperable: whichever runs first
/// claims the block, and the other becomes a no-op.
const MARKER_START: &str = "# >>> reportal shell integration (do not edit) >>>";

/// Fenced marker closing the auto-managed integration block.
const MARKER_END: &str = "# <<< reportal shell integration <<<";

/// One auto-wire attempt against a specific shell + rc-file pair.
/// Holds the shell variant and resolved rc file path so the helper
/// methods don't have to re-derive them on every call.
struct AutoWireOperation {
    /// Target shell whose `eval` line we'd append.
    shell: InitializeShell,
    /// Resolved rc file path the eval line should land in.
    rc_file: PathBuf,
}

/// Outcome of a single `AutoWireOperation::install_if_missing` call.
/// The caller branches on this to decide whether to print an
/// activation hint to stderr — only `Appended` earns the hint.
enum AutoWireOperationOutcome {
    /// Marker already present — no change made.
    AlreadyPresent,
    /// Fenced block appended to the rc file.
    Appended,
}

/// Whether the fenced block should be preceded by a separator
/// newline. Replaces a `bool` parameter so each call site reads
/// the policy at the call instead of decoding which polarity of
/// `true` / `false` corresponds to "needs a newline."
#[derive(Clone, Copy)]
enum AutoWireOperationLeadingNewlinePolicy {
    /// File already ends mid-line — emit a separator newline so
    /// the marker starts on its own line.
    InsertSeparator,
    /// File is empty or already ends with a newline — no separator
    /// needed.
    NoSeparator,
}

/// Which rc-file I/O step failed. Lifted out of the error enum's
/// variants into a discriminant so `AutoWireOperationRcFileError`
/// is a single struct that names its three shared fields once.
#[derive(Debug)]
enum AutoWireOperationRcFileOperationKind {
    /// `read_to_string` of an existing rc file.
    Read,
    /// `create_dir_all` of the rc file's parent directory.
    CreateParentDirectory,
    /// `OpenOptions::open` in append mode on the rc file.
    Open,
    /// `write_all` of the fenced block bytes.
    Write,
}

/// Typed I/O failure produced by `AutoWireOperation` while reading
/// or writing the user's rc file. Carries the failed step plus the
/// path it was operating on so the caller's stderr hint can quote
/// the exact file the auto-wire stumbled on.
#[derive(Debug, thiserror::Error)]
#[error("auto-wire rc-file {operation_kind:?} on {rc_file_path}: {source}")]
struct AutoWireOperationRcFileError {
    /// Which rc-file I/O step failed.
    operation_kind: AutoWireOperationRcFileOperationKind,
    /// Display path of the file or directory we were operating on.
    rc_file_path: String,
    /// Underlying I/O error from the standard library.
    #[source]
    source: std::io::Error,
}

/// Result of comparing the `REPORTAL_LOADED` marker against the
/// actual parent process id. Replaces a `bool` return so call sites
/// at `ensure_shell_integration_installed` and `rep doctor` carry
/// the meaning of each branch in the type, not in a hand-decoded
/// `true` / `false`.
pub enum AutoWireOperationLoadedState {
    /// The integration script ran in the shell that launched this
    /// `rep` process — `rj` and friends are defined.
    LoadedInCurrentShell,
    /// The marker is unset, set to a stale value, or inherited from
    /// a grandparent — the user's current shell almost certainly
    /// does NOT have `rj` defined.
    NotLoadedInCurrentShell,
}

/// PID-marker comparison helpers on `AutoWireOperationLoadedState`,
/// kept as associated functions so the platform-gated branching
/// lives on the type that names the answer instead of as free
/// helpers crowding the module.
impl AutoWireOperationLoadedState {
    /// Compares a PID-stamped `REPORTAL_LOADED` value against the
    /// running process's actual parent PID on Unix; returns
    /// `NotLoadedInCurrentShell` on mismatch or when the parent
    /// PID can't be read.
    #[cfg(unix)]
    fn classify_against_actual_parent_pid(stamped_marker_value: &str) -> Self {
        let Some(parent_process_id) = rustix::process::getppid() else {
            return Self::NotLoadedInCurrentShell;
        };
        let parent_pid_string =
            parent_process_id.as_raw_nonzero().get().to_string();
        if parent_pid_string == stamped_marker_value {
            Self::LoadedInCurrentShell
        } else {
            Self::NotLoadedInCurrentShell
        }
    }

    /// Windows-side parent-PID lookup via `sysinfo`. Mirrors the unix
    /// branch's semantics: returns `NotLoadedInCurrentShell` on
    /// mismatch, when the running process can't be located in the
    /// snapshot, or when the process has no parent. `sysinfo` is the
    /// only safe-API parent-PID source under the project's
    /// `forbid(unsafe_code)` constraint; `winsafe` would push raw
    /// `ToolHelp32` boilerplate into our tree and `windows-sys` is
    /// straight FFI.
    #[cfg(windows)]
    fn classify_against_actual_parent_pid(stamped_marker_value: &str) -> Self {
        let mut system_snapshot = sysinfo::System::new();
        let self_process_id = sysinfo::Pid::from_u32(std::process::id());
        system_snapshot.refresh_processes(
            sysinfo::ProcessesToUpdate::Some(&[self_process_id]),
            true,
        );
        let Some(self_process) = system_snapshot.process(self_process_id) else {
            return Self::NotLoadedInCurrentShell;
        };
        let Some(parent_process_id) = self_process.parent() else {
            return Self::NotLoadedInCurrentShell;
        };
        let parent_pid_string = parent_process_id.as_u32().to_string();
        if parent_pid_string == stamped_marker_value {
            Self::LoadedInCurrentShell
        } else {
            Self::NotLoadedInCurrentShell
        }
    }
}

#[cfg(not(any(unix, windows)))]
compile_error!(
    "RePortal needs platform-specific parent-PID lookup for the \
     REPORTAL_LOADED-inheritance check; only unix and windows are \
     wired up. Add a branch in classify_against_actual_parent_pid for \
     this target before building."
);

/// Auto-wire entry point + private helpers. Grouped here so the
/// shell-detection / rc-file-resolution / fenced-block-write logic
/// lives next to the data it operates on, instead of leaking out
/// as a pile of free helper functions sharing implicit invariants.
impl AutoWireOperation {
    /// Resolves the user's shell + rc file from the environment.
    /// Returns `None` for fish, tcsh, csh, or any shell we don't
    /// generate integration code for — those users can run
    /// `rep init <shell>` themselves and will get a clear rejection
    /// from the CLI parser.
    fn detect_from_environment() -> Option<Self> {
        let shell = Self::shell_from_environment()?;
        let rc_file = Self::rc_file_for(shell)?;
        Some(Self { shell, rc_file })
    }

    /// Performs a single auto-wire attempt: detects the shell + rc
    /// file, installs the fenced block if missing, and emits a
    /// stderr hint for the appended / failed cases. Lives on the
    /// type so `ensure_shell_integration_installed` reads as the
    /// policy ("if not loaded, attempt") and the mechanics travel
    /// with the data they operate on.
    fn run_one_auto_wire_attempt() {
        let Some(operation) = Self::detect_from_environment() else {
            return;
        };
        operation.report_install_outcome();
    }

    /// Runs `install_if_missing` and routes each outcome to the
    /// appropriate stderr hint. Split out from
    /// `run_one_auto_wire_attempt` so the policy branch ("if no
    /// operation could be detected, do nothing") and the I/O
    /// branch ("install + report") read independently.
    fn report_install_outcome(&self) {
        match self.install_if_missing() {
            Ok(AutoWireOperationOutcome::AlreadyPresent) => {}
            Ok(AutoWireOperationOutcome::Appended) => {
                eprintln!(
                    "RePortal: added shell integration to {rc} — run 'source {rc}' or open a new shell to activate.",
                    rc = self.rc_file.display(),
                );
            }
            Err(reason) => {
                eprintln!(
                    "RePortal: note: could not auto-wire shell integration ({reason}); add `{eval_line}` to your rc file manually.",
                    eval_line = self.eval_line(),
                );
            }
        }
    }

    /// Parses `$SHELL`'s basename and returns the matching
    /// `InitializeShell` variant; falls back to `PowerShell` on
    /// Windows when `$SHELL` is unset so the binary still picks
    /// the right rc file when invoked from `cmd.exe`.
    fn shell_from_environment() -> Option<InitializeShell> {
        Self::shell_basename_from_environment_variable().map_or_else(
            Self::default_shell_when_environment_unset,
            |name| Self::interpret_shell_basename(&name),
        )
    }

    /// Reads `$SHELL` and returns its lowercase basename, or `None`
    /// if the env var is unset or has no usable basename.
    fn shell_basename_from_environment_variable() -> Option<String> {
        let shell_executable_path_value = environment::var_os("SHELL")?;
        Path::new(&shell_executable_path_value)
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_ascii_lowercase)
    }

    /// Maps a lowercase shell basename to the `InitializeShell`
    /// variant we generate integration code for. Returns `None`
    /// for shells we don't support yet (fish, tcsh, csh, ...).
    fn interpret_shell_basename(basename: &str) -> Option<InitializeShell> {
        match basename {
            "zsh" => Some(InitializeShell::Zsh),
            "bash" => Some(InitializeShell::Bash),
            "pwsh" | "powershell" => Some(InitializeShell::Powershell),
            _ => None,
        }
    }

    /// Picks a default shell when `$SHELL` is unset — `PowerShell`
    /// on Windows so `cmd.exe`-launched invocations still find a
    /// reasonable rc file, `None` everywhere else.
    fn default_shell_when_environment_unset() -> Option<InitializeShell> {
        if cfg!(windows) {
            Some(InitializeShell::Powershell)
        } else {
            None
        }
    }

    /// Resolves the rc file we should append to for the given
    /// shell.
    fn rc_file_for(shell: InitializeShell) -> Option<PathBuf> {
        match shell {
            InitializeShell::Zsh => {
                directories::home_dir().map(|home| home.join(".zshrc"))
            }
            InitializeShell::Bash => {
                directories::home_dir().map(|home| home.join(".bashrc"))
            }
            InitializeShell::Powershell => Self::powershell_profile_path(),
        }
    }

    /// Resolves the `PowerShell` `$PROFILE` path — prefers the
    /// host-provided env var, then falls back to the conventional
    /// `Documents\PowerShell\…` layout under the user's home.
    fn powershell_profile_path() -> Option<PathBuf> {
        environment::var_os("PROFILE").map_or_else(
            Self::default_powershell_profile_path,
            Self::interpret_powershell_profile_environment_variable,
        )
    }

    /// Wraps a non-empty `$PROFILE` value into a `PathBuf`; treats
    /// the empty string as "unset" so the conventional default
    /// path takes over.
    fn interpret_powershell_profile_environment_variable(
        profile: std::ffi::OsString,
    ) -> Option<PathBuf> {
        let powershell_profile_path = PathBuf::from(profile);
        if powershell_profile_path.as_os_str().is_empty() {
            return Self::default_powershell_profile_path();
        }
        Some(powershell_profile_path)
    }

    /// Conventional `Documents\PowerShell\Microsoft.PowerShell_profile.ps1`
    /// rooted at the user's home directory.
    fn default_powershell_profile_path() -> Option<PathBuf> {
        directories::home_dir().map(|home| {
            home.join("Documents")
                .join("PowerShell")
                .join("Microsoft.PowerShell_profile.ps1")
        })
    }

    /// Returns the single line that goes inside the fenced block
    /// for this shell. Byte-for-byte identical to what the shell
    /// installer scripts write.
    fn eval_line(&self) -> &'static str {
        match self.shell {
            InitializeShell::Zsh => "eval \"$(rep init zsh)\"",
            InitializeShell::Bash => "eval \"$(rep init bash)\"",
            InitializeShell::Powershell => {
                "Invoke-Expression (& rep init powershell | Out-String)"
            }
        }
    }

    /// Appends the fenced integration block to the rc file iff the
    /// marker is absent. Branches on whether the file already
    /// exists so each branch handles one focused concern.
    fn install_if_missing(
        &self,
    ) -> Result<AutoWireOperationOutcome, AutoWireOperationRcFileError> {
        if self.rc_file.exists() {
            return self.install_into_existing_rc_file();
        }
        self.install_into_fresh_rc_file()
    }

    /// Reads the existing rc file, decides whether the marker is
    /// already present, and appends the block with the correct
    /// leading-newline policy for whatever the file's final byte
    /// is.
    fn install_into_existing_rc_file(
        &self,
    ) -> Result<AutoWireOperationOutcome, AutoWireOperationRcFileError> {
        let existing_rc_file_content = self.read_rc_file_content()?;
        if existing_rc_file_content.contains(MARKER_START) {
            return Ok(AutoWireOperationOutcome::AlreadyPresent);
        }
        let leading_newline_policy =
            Self::leading_newline_policy_for(&existing_rc_file_content);
        self.write_fenced_block(leading_newline_policy)?;
        Ok(AutoWireOperationOutcome::Appended)
    }

    /// Creates the parent directory if needed, then writes the
    /// fenced block as the file's only content. No leading
    /// separator since the file is empty.
    fn install_into_fresh_rc_file(
        &self,
    ) -> Result<AutoWireOperationOutcome, AutoWireOperationRcFileError> {
        self.create_parent_directory_if_needed()?;
        self.write_fenced_block(AutoWireOperationLeadingNewlinePolicy::NoSeparator)?;
        Ok(AutoWireOperationOutcome::Appended)
    }

    /// Reads the rc file as UTF-8, mapping any I/O failure into the
    /// typed error so callers don't have to handle
    /// `std::io::Error` directly.
    fn read_rc_file_content(&self) -> Result<String, AutoWireOperationRcFileError> {
        std::fs::read_to_string(&self.rc_file).map_err(|source| {
            AutoWireOperationRcFileError {
                operation_kind: AutoWireOperationRcFileOperationKind::Read,
                rc_file_path: self.rc_file.display().to_string(),
                source,
            }
        })
    }

    /// Decides whether the appended block needs a leading newline
    /// separator based on whether the existing content already ends
    /// with one. Empty files don't need a separator either — they
    /// have no preceding content for the marker to bump up against.
    fn leading_newline_policy_for(
        existing_rc_file_content: &str,
    ) -> AutoWireOperationLeadingNewlinePolicy {
        if existing_rc_file_content.is_empty()
            || existing_rc_file_content.ends_with('\n')
        {
            AutoWireOperationLeadingNewlinePolicy::NoSeparator
        } else {
            AutoWireOperationLeadingNewlinePolicy::InsertSeparator
        }
    }

    /// Creates the rc file's parent directory tree if it doesn't
    /// already exist. No-op when the rc file's path has no parent
    /// component or the parent already exists.
    fn create_parent_directory_if_needed(
        &self,
    ) -> Result<(), AutoWireOperationRcFileError> {
        let Some(rc_file_parent_directory) = self.rc_file.parent() else {
            return Ok(());
        };
        if rc_file_parent_directory.as_os_str().is_empty()
            || rc_file_parent_directory.exists()
        {
            return Ok(());
        }
        std::fs::create_dir_all(rc_file_parent_directory).map_err(|source| {
            AutoWireOperationRcFileError {
                operation_kind: AutoWireOperationRcFileOperationKind::CreateParentDirectory,
                rc_file_path: rc_file_parent_directory.display().to_string(),
                source,
            }
        })
    }

    /// Opens the rc file in append mode and writes the fenced
    /// block. Composition of the block bytes lives in
    /// `compose_fenced_block` so this method stays focused on I/O.
    fn write_fenced_block(
        &self,
        leading_newline_policy: AutoWireOperationLeadingNewlinePolicy,
    ) -> Result<(), AutoWireOperationRcFileError> {
        let mut rc_file_handle = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.rc_file)
            .map_err(|source| AutoWireOperationRcFileError {
                operation_kind: AutoWireOperationRcFileOperationKind::Open,
                rc_file_path: self.rc_file.display().to_string(),
                source,
            })?;
        let fenced_block_bytes = self.compose_fenced_block(leading_newline_policy);
        rc_file_handle
            .write_all(fenced_block_bytes.as_bytes())
            .map_err(|source| AutoWireOperationRcFileError {
                operation_kind: AutoWireOperationRcFileOperationKind::Write,
                rc_file_path: self.rc_file.display().to_string(),
                source,
            })
    }

    /// Builds the fenced block as a single string. `PowerShell` on
    /// Windows uses CRLF; everything else uses LF.
    fn compose_fenced_block(
        &self,
        leading_newline_policy: AutoWireOperationLeadingNewlinePolicy,
    ) -> String {
        let eol = self.line_ending();
        let mut fenced_block_buffer = String::new();
        match leading_newline_policy {
            AutoWireOperationLeadingNewlinePolicy::InsertSeparator => {
                fenced_block_buffer.push_str(eol);
            }
            AutoWireOperationLeadingNewlinePolicy::NoSeparator => {}
        }
        fenced_block_buffer.push_str(eol);
        fenced_block_buffer.push_str(MARKER_START);
        fenced_block_buffer.push_str(eol);
        fenced_block_buffer.push_str(self.eval_line());
        fenced_block_buffer.push_str(eol);
        fenced_block_buffer.push_str(MARKER_END);
        fenced_block_buffer.push_str(eol);
        fenced_block_buffer
    }

    /// Picks the line-ending bytes for this shell. CRLF only for
    /// `PowerShell` on Windows; LF for everything else (including
    /// `PowerShell` on macOS / Linux via `pwsh`).
    fn line_ending(&self) -> &'static str {
        match self.shell {
            InitializeShell::Powershell => {
                #[cfg(windows)]
                {
                    "\r\n"
                }
                #[cfg(not(windows))]
                {
                    "\n"
                }
            }
            InitializeShell::Zsh | InitializeShell::Bash => "\n",
        }
    }
}

/// @why Best-effort first-run hook that ensures the user's shell rc
/// file sources the `RePortal` integration, so users who installed
/// via `cargo install reportal` (or any route that skips the shell
/// installer) don't have to edit their rc file by hand. Called from
/// `main()` before subcommand dispatch and skipped for `rep init`
/// itself, since that subcommand's stdout is consumed via
/// `eval "$(rep init zsh)"` where any stderr noise would surface on
/// every shell startup. The function never returns an error to its
/// caller — auto-setup is a convenience and must never break the
/// user's actual command.
pub fn ensure_shell_integration_installed() {
    match current_shell_integration_loaded_state() {
        AutoWireOperationLoadedState::LoadedInCurrentShell => {}
        AutoWireOperationLoadedState::NotLoadedInCurrentShell => {
            AutoWireOperation::run_one_auto_wire_attempt();
        }
    }
}

/// @why Reports whether the integration script actually ran in the
/// shell that launched this `rep` process, so callers
/// (`ensure_shell_integration_installed`, `rep doctor`) can
/// distinguish between a real loaded session and a child process
/// that merely inherited the `REPORTAL_LOADED` env var from a
/// grandparent without sourcing the integration itself. The shell
/// stamps the marker with its own `$$` (or `$PID` on `PowerShell`)
/// at integration time; we read that stamp and compare it against
/// our parent process id. A match means the parent shell IS the
/// one that set the marker; a mismatch means the marker was
/// inherited across one or more process boundaries and the actual
/// functions (`rj`, `ro`, ...) probably aren't defined in the
/// user's prompt.
///
/// The legacy literal value `"1"` (used by versions ≤ 0.18.1) is
/// treated as "loaded" so users with an older shell session still
/// open during an upgrade don't see spurious warnings on every
/// `rep` invocation until they re-source their rc file.
#[must_use]
pub fn current_shell_integration_loaded_state() -> AutoWireOperationLoadedState {
    let Ok(stamped_marker_value) = environment::var("REPORTAL_LOADED") else {
        return AutoWireOperationLoadedState::NotLoadedInCurrentShell;
    };
    if stamped_marker_value == "1" {
        return AutoWireOperationLoadedState::LoadedInCurrentShell;
    }
    AutoWireOperationLoadedState::classify_against_actual_parent_pid(
        &stamped_marker_value,
    )
}

//! Looks up whether a candidate name resolves to an executable
//! file somewhere on the user's `PATH`.
//!
//! Used by the repo and workspace registration builders to reject
//! aliases that would shadow an existing system command once the
//! user opts a config entry into shell-alias export. Without this
//! check, opting a repo aliased `mc` into shell-alias mode would
//! silently clobber Midnight Commander on the user's prompt.
//!
//! The lookup is deliberately lightweight: no spawning of
//! `which`, no shell invocation, just a `PATH` walk plus an
//! executable-bit check. That matches what an interactive shell
//! does to resolve a typed command and gives the same answer.

use std::path::{Path, PathBuf};

/// Outcome of probing `PATH` for a candidate name.
#[derive(Debug)]
pub enum SystemExecutableLookupOutcome {
    /// No directory on `PATH` contains an executable file with
    /// the candidate name.
    NotFound,
    /// At least one directory on `PATH` contains an executable
    /// file with the candidate name; the first match's full path
    /// is reported back so error messages can point the user at
    /// the binary they would otherwise shadow.
    ShadowsExisting {
        /// Absolute path to the existing executable that the
        /// candidate alias would shadow.
        existing_executable: PathBuf,
    },
}

/// Construction and helpers for the lookup outcome.
impl SystemExecutableLookupOutcome {
    /// @why Probes the user's `PATH` for a candidate alias name
    /// so the repo and workspace registration builders can reject
    /// aliases that would clobber an existing system executable
    /// once shell-alias export is opted in.
    #[must_use]
    pub fn for_candidate_name(candidate_name: &str) -> Self {
        let Some(path_environment_variable) = std::env::var_os("PATH") else {
            return Self::NotFound;
        };
        for path_directory in std::env::split_paths(&path_environment_variable) {
            let bare_probe_path = path_directory.join(candidate_name);
            if Self::is_executable_file(&bare_probe_path) {
                return Self::ShadowsExisting {
                    existing_executable: bare_probe_path,
                };
            }
            #[cfg(windows)]
            {
                let extension_probe_path =
                    path_directory.join(format!("{candidate_name}.exe"));
                if extension_probe_path.is_file() {
                    return Self::ShadowsExisting {
                        existing_executable: extension_probe_path,
                    };
                }
            }
        }
        Self::NotFound
    }

    /// Whether the given path is a regular file with at least
    /// one executable bit set on Unix, or a regular file on
    /// Windows (where executability is encoded by extension and
    /// the caller probes the `.exe` form separately).
    fn is_executable_file(probe_path: &Path) -> bool {
        let Ok(file_metadata) = std::fs::metadata(probe_path) else {
            return false;
        };
        if !file_metadata.is_file() {
            return false;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let unix_permission_bits = file_metadata.permissions().mode();
            return (unix_permission_bits & 0o111) != 0;
        }
        #[cfg(not(unix))]
        {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_garbage_name_resolves_to_not_found() {
        let outcome = SystemExecutableLookupOutcome::for_candidate_name(
            "this-name-should-never-exist-on-any-PATH-zzz-9182734",
        );
        assert!(matches!(outcome, SystemExecutableLookupOutcome::NotFound));
    }

    #[cfg(unix)]
    #[test]
    fn cat_resolves_to_shadows_existing_on_unix() {
        let outcome = SystemExecutableLookupOutcome::for_candidate_name("cat");
        assert!(
            matches!(outcome, SystemExecutableLookupOutcome::ShadowsExisting { .. }),
            "expected `cat` to resolve to /bin/cat or /usr/bin/cat on any POSIX system, got {outcome:?}",
        );
    }
}

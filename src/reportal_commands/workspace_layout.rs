//! Materializes a workspace as a real directory on disk, with
//! symlinks (Unix) or directory junctions (Windows) pointing at
//! each member repo, plus a `<name>.code-workspace` file inside.
//!
//! The workspace directory replaces the pre-v0.15.2 layout where
//! reportal only wrote a loose `<name>.code-workspace` JSON file
//! under `~/.reportal/workspaces/`. Materializing a real directory
//! means `rjw` can cd into it and every member repo appears as a
//! subdirectory — so `ls`, `claude`, `cargo`, and an IDE all see
//! the same view of the workspace.
//!
//! Member repos are NEVER moved on disk; only a symlink / junction
//! is created per member. A single repo can therefore belong to any
//! number of workspaces without conflict.

use crate::code_workspace::CodeWorkspaceFile;
use crate::error::ReportalError;
use std::io;
use std::path::{Path, PathBuf};

/// Describes one member link that the workspace layout builder
/// materializes on disk.
///
/// The link name is the short label the user sees in the workspace
/// directory (and in the editor sidebar) — typically the repo alias.
/// The target is the absolute path of the real repo on disk, which
/// the symlink / junction points at.
pub struct WorkspaceLinkSpec {
    /// Short name for the link inside the workspace directory.
    pub link_name: String,
    /// Absolute path the link should resolve to.
    pub target_absolute_path: PathBuf,
}

/// Outcome of materializing a single member link, for post-op
/// diagnostics and logging.
///
/// Named variants rather than `Option<bool>` because `Created`,
/// `AlreadyCorrect`, and `Replaced` carry different operational
/// meanings the caller surfaces in user output.
pub enum WorkspaceLinkOutcome {
    /// The link did not exist; a fresh symlink / junction was created.
    Created,
    /// A link already existed pointing at the expected target; no
    /// change on disk.
    AlreadyCorrect,
    /// A link already existed pointing elsewhere; the stale link was
    /// removed and recreated against the new target.
    Replaced,
}

/// Creates a symlink (Unix) or directory junction (Windows) at
/// `link_path` pointing to `target`.
///
/// The platform split exists because Windows `std::os::windows::fs::
/// symlink_dir` requires developer mode or elevated privileges,
/// while a directory junction works for any user. Junctions resolve
/// identically to symlinks for our purposes: `cd`, `ls`, and editor
/// file operations all follow them transparently.
///
/// # Errors
///
/// Returns the underlying `io::Error` from the OS call if the link
/// cannot be created (common causes: parent directory missing,
/// target does not exist on Windows, or a file already exists at
/// `link_path`).
pub fn create_workspace_link(target: &Path, link_path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link_path)
    }
    #[cfg(windows)]
    {
        junction::create(target, link_path)
    }
    #[cfg(not(any(unix, windows)))]
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "workspace symlink creation is not supported on this platform",
        ))
    }
}

/// Parameters for materializing a workspace's on-disk layout.
///
/// Bundled so the public entry point takes a single struct rather
/// than four positional arguments, in line with the project's
/// param-count rules for public functions.
pub struct WorkspaceLayoutParameters<'caller> {
    /// Absolute path of the directory that should hold the
    /// `.code-workspace` file and every member link.
    pub workspace_directory: &'caller Path,
    /// Workspace name (also the base filename of the
    /// `.code-workspace` file written inside the directory).
    pub workspace_name: &'caller str,
    /// Ordered list of member links to materialize. The order
    /// determines the `folders[]` order in the `.code-workspace`
    /// file and therefore the editor sidebar.
    pub member_links: &'caller [WorkspaceLinkSpec],
    /// Workspace identity label stamped into the editor's
    /// `window.title` setting. `None` leaves the setting
    /// untouched; `Some(label)` replaces the `.code-workspace`
    /// file's `settings` object with a reportal-managed block.
    pub window_title_override: Option<&'caller str>,
    /// Workspace accent color as a raw `#RRGGBB` string, used
    /// for the editor title bar when `window_title_override` is
    /// also set. `None` means no title-bar color override.
    pub title_bar_color_hex: Option<&'caller str>,
}

/// Materializes the workspace directory, one link per member, and
/// the `.code-workspace` file inside the directory.
///
/// Idempotent: running against an existing workspace directory
/// updates links whose targets have changed, leaves already-correct
/// links untouched, and refuses to clobber non-link entries at the
/// link paths so a user who drops a real file into the workspace
/// directory cannot lose it to a silent overwrite.
///
/// The `.code-workspace` file is written with relative `./<link>`
/// folder entries so opening it in Cursor / `VSCode` surfaces the
/// symlinked paths exactly the way a terminal `cd` into the
/// workspace directory would.
///
/// # Errors
///
/// Returns [`ReportalError::CodeWorkspaceIoFailure`] if the
/// directory cannot be created, a stale link cannot be replaced, or
/// the `.code-workspace` file cannot be written; or
/// [`ReportalError::ValidationFailure`] if any link path is already
/// occupied by a real file / directory that is not a symlink
/// pointing at the expected target.
pub fn materialize_workspace_layout(
    layout_params: &WorkspaceLayoutParameters<'_>,
) -> Result<PathBuf, ReportalError> {
    ensure_directory_exists(layout_params.workspace_directory)?;
    for link_spec in layout_params.member_links {
        ensure_member_link(layout_params.workspace_directory, link_spec)?;
    }
    let workspace_file_path = workspace_file_path_inside_dir(
        layout_params.workspace_directory,
        layout_params.workspace_name,
    );
    let relative_folder_paths: Vec<PathBuf> = layout_params
        .member_links
        .iter()
        .map(|link_spec| PathBuf::from(format!("./{}", link_spec.link_name)))
        .collect();
    let mut code_workspace_document =
        CodeWorkspaceFile::load_or_empty(&workspace_file_path)?;
    code_workspace_document.set_folder_paths(&relative_folder_paths);
    if let Some(window_title) = layout_params.window_title_override {
        code_workspace_document.set_workspace_identity(
            window_title.to_owned(),
            layout_params.title_bar_color_hex.map(str::to_owned),
        );
    }
    code_workspace_document.write_to_disk(&workspace_file_path)?;
    Ok(workspace_file_path)
}

/// Computes the canonical `.code-workspace` file path inside a
/// workspace directory, for callers that only need the file path
/// without running the full materialization.
#[must_use]
pub fn workspace_file_path_inside_dir(
    workspace_directory: &Path,
    workspace_name: &str,
) -> PathBuf {
    workspace_directory.join(format!("{workspace_name}.code-workspace"))
}

/// Removes the workspace directory and every symlink / file it
/// contains, following the `--purge` contract on
/// `rep workspace delete`.
///
/// Only the workspace directory is removed. Member repo contents
/// are never touched: symlinks are unlinked, the real repo
/// directories they point at remain on disk.
///
/// # Errors
///
/// Returns [`ReportalError::CodeWorkspaceIoFailure`] if the
/// directory cannot be removed (permission denied, device busy,
/// etc.). A missing directory is not an error — purge is a no-op
/// in that case because the user is clearly asking for "make sure
/// this doesn't exist."
pub fn purge_workspace_directory(workspace_directory: &Path) -> Result<(), ReportalError> {
    if !workspace_directory.exists() {
        return Ok(());
    }
    std::fs::remove_dir_all(workspace_directory).map_err(|remove_error| {
        ReportalError::CodeWorkspaceIoFailure {
            file_path: workspace_directory.display().to_string(),
            reason: remove_error.to_string(),
        }
    })?;
    Ok(())
}

/// Creates `target_directory` (and every missing parent) if it
/// does not already exist.
fn ensure_directory_exists(target_directory: &Path) -> Result<(), ReportalError> {
    if target_directory.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(target_directory).map_err(|create_error| {
        ReportalError::CodeWorkspaceIoFailure {
            file_path: target_directory.display().to_string(),
            reason: create_error.to_string(),
        }
    })?;
    Ok(())
}

/// Ensures one member link exists at
/// `<workspace_directory>/<link_name>` pointing at
/// `link_spec.target_absolute_path`.
///
/// Handles three cases:
/// - The path is free: create the link.
/// - The path is a link to the expected target: no-op.
/// - The path is a link to a different target: unlink + recreate.
/// - The path is a real file / directory: refuse — the user would
///   lose data on an unconditional overwrite.
fn ensure_member_link(
    workspace_directory: &Path,
    link_spec: &WorkspaceLinkSpec,
) -> Result<WorkspaceLinkOutcome, ReportalError> {
    let link_path = workspace_directory.join(&link_spec.link_name);
    let link_metadata = std::fs::symlink_metadata(&link_path);
    match link_metadata {
        Err(metadata_error) if metadata_error.kind() == io::ErrorKind::NotFound => {
            create_workspace_link(&link_spec.target_absolute_path, &link_path)
                .map_err(|link_error| ReportalError::CodeWorkspaceIoFailure {
                    file_path: link_path.display().to_string(),
                    reason: link_error.to_string(),
                })?;
            Ok(WorkspaceLinkOutcome::Created)
        }
        Err(metadata_error) => Err(ReportalError::CodeWorkspaceIoFailure {
            file_path: link_path.display().to_string(),
            reason: metadata_error.to_string(),
        }),
        Ok(existing_metadata) => handle_existing_link_path(
            &link_path,
            &existing_metadata,
            &link_spec.target_absolute_path,
        ),
    }
}

/// Decides what to do with an existing entry at the link path:
/// leave it if it already points at the right target, replace it
/// if it's a stale link, or refuse if it's real user data.
fn handle_existing_link_path(
    link_path: &Path,
    existing_metadata: &std::fs::Metadata,
    expected_target: &Path,
) -> Result<WorkspaceLinkOutcome, ReportalError> {
    if existing_metadata.file_type().is_symlink() {
        return replace_or_keep_symlink(link_path, expected_target);
    }
    // On Windows a directory junction reports as a directory, not a
    // symlink. Treat any directory whose canonical path matches the
    // expected target as a correct link so re-runs are idempotent.
    if existing_metadata.is_dir() && paths_point_to_same_target(link_path, expected_target) {
        return Ok(WorkspaceLinkOutcome::AlreadyCorrect);
    }
    Err(ReportalError::ValidationFailure {
        field: "workspace member link".to_owned(),
        reason: format!(
            "'{}' already exists and is not a reportal-managed symlink; refusing to clobber — move the file out of the workspace directory and retry",
            link_path.display(),
        ),
    })
}

/// Reads a symlink, compares against the expected target, and
/// either leaves it alone or replaces it with a correct link.
fn replace_or_keep_symlink(
    link_path: &Path,
    expected_target: &Path,
) -> Result<WorkspaceLinkOutcome, ReportalError> {
    let current_target = std::fs::read_link(link_path).map_err(|read_error| {
        ReportalError::CodeWorkspaceIoFailure {
            file_path: link_path.display().to_string(),
            reason: read_error.to_string(),
        }
    })?;
    if current_target == expected_target {
        return Ok(WorkspaceLinkOutcome::AlreadyCorrect);
    }
    std::fs::remove_file(link_path).map_err(|remove_error| {
        ReportalError::CodeWorkspaceIoFailure {
            file_path: link_path.display().to_string(),
            reason: remove_error.to_string(),
        }
    })?;
    create_workspace_link(expected_target, link_path).map_err(|link_error| {
        ReportalError::CodeWorkspaceIoFailure {
            file_path: link_path.display().to_string(),
            reason: link_error.to_string(),
        }
    })?;
    Ok(WorkspaceLinkOutcome::Replaced)
}

/// Best-effort comparison of two paths by canonical form, used on
/// Windows where a junction shows up as a plain directory in the
/// metadata but still resolves to the intended target through
/// `canonicalize`.
fn paths_point_to_same_target(candidate_path: &Path, expected_path: &Path) -> bool {
    match (candidate_path.canonicalize(), expected_path.canonicalize()) {
        (Ok(candidate_canonical), Ok(expected_canonical)) => {
            candidate_canonical == expected_canonical
        }
        _ => false,
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "test code: `.expect` is the project-wide convention for fixture setup in tests (matches the pattern in code_workspace_file.rs and workspace_registration_builder.rs test modules)"
)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unique_temp_dir(label: &str) -> PathBuf {
        let counter_value = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let process_id = std::process::id();
        let path =
            std::env::temp_dir().join(format!("reportal-wslayout-{label}-{process_id}-{counter_value}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("temp dir must be creatable");
        path
    }

    #[test]
    #[cfg(unix)]
    fn create_workspace_link_on_unix_points_at_target() {
        let scratch_dir = unique_temp_dir("create-link");
        let target_dir = scratch_dir.join("target");
        std::fs::create_dir_all(&target_dir).expect("target must be creatable");
        let link_path = scratch_dir.join("link");

        create_workspace_link(&target_dir, &link_path).expect("link creation must succeed");

        let link_metadata =
            std::fs::symlink_metadata(&link_path).expect("link metadata must be readable");
        assert!(
            link_metadata.file_type().is_symlink(),
            "created path must be a symlink",
        );
        let resolved_target =
            std::fs::read_link(&link_path).expect("symlink must be readable");
        assert_eq!(resolved_target, target_dir);

        let _ = std::fs::remove_dir_all(&scratch_dir);
    }

    #[test]
    fn materialize_layout_produces_expected_links_and_file() {
        let scratch_dir = unique_temp_dir("materialize");
        let repo_alpha = scratch_dir.join("repos").join("alpha");
        let repo_bravo = scratch_dir.join("repos").join("bravo");
        let repo_charlie = scratch_dir.join("repos").join("charlie");
        std::fs::create_dir_all(&repo_alpha).expect("repo alpha must be creatable");
        std::fs::create_dir_all(&repo_bravo).expect("repo bravo must be creatable");
        std::fs::create_dir_all(&repo_charlie).expect("repo charlie must be creatable");

        let workspace_dir = scratch_dir.join("workspaces").join("nro");
        let member_links = vec![
            WorkspaceLinkSpec {
                link_name: "alpha".to_owned(),
                target_absolute_path: repo_alpha.clone(),
            },
            WorkspaceLinkSpec {
                link_name: "bravo".to_owned(),
                target_absolute_path: repo_bravo.clone(),
            },
            WorkspaceLinkSpec {
                link_name: "charlie".to_owned(),
                target_absolute_path: repo_charlie.clone(),
            },
        ];

        let workspace_file_path = materialize_workspace_layout(&WorkspaceLayoutParameters {
            workspace_directory: &workspace_dir,
            workspace_name: "nro",
            member_links: &member_links,
            window_title_override: None,
            title_bar_color_hex: None,
        })
        .expect("materialize must succeed");

        assert!(workspace_dir.exists(), "workspace directory must be created");
        assert_eq!(workspace_file_path, workspace_dir.join("nro.code-workspace"));
        assert!(
            workspace_file_path.exists(),
            ".code-workspace file must be written inside workspace dir",
        );

        for (link_name, expected_target) in [
            ("alpha", &repo_alpha),
            ("bravo", &repo_bravo),
            ("charlie", &repo_charlie),
        ] {
            let link_path = workspace_dir.join(link_name);
            let metadata =
                std::fs::symlink_metadata(&link_path).expect("link metadata must exist");
            #[cfg(unix)]
            assert!(
                metadata.file_type().is_symlink(),
                "member '{link_name}' must be a symlink",
            );
            #[cfg(windows)]
            assert!(
                metadata.is_dir(),
                "member '{link_name}' must be a junction (reports as dir on Windows)",
            );
            let _ = metadata;
            #[cfg(unix)]
            {
                let resolved =
                    std::fs::read_link(&link_path).expect("link must be readable");
                assert_eq!(&resolved, expected_target);
            }
            let _ = expected_target;
        }

        let workspace_file_contents =
            std::fs::read_to_string(&workspace_file_path).expect("workspace file read");
        assert!(
            workspace_file_contents.contains("./alpha"),
            "folders[] should use relative path ./alpha",
        );
        assert!(
            workspace_file_contents.contains("./bravo"),
            "folders[] should use relative path ./bravo",
        );
        assert!(
            workspace_file_contents.contains("./charlie"),
            "folders[] should use relative path ./charlie",
        );

        let _ = std::fs::remove_dir_all(&scratch_dir);
    }

    #[test]
    #[cfg(unix)]
    fn materialize_layout_is_idempotent_and_replaces_stale_links() {
        let scratch_dir = unique_temp_dir("idempotent");
        let real_target = scratch_dir.join("repos").join("real");
        let stale_target = scratch_dir.join("repos").join("stale");
        std::fs::create_dir_all(&real_target).expect("real target must be creatable");
        std::fs::create_dir_all(&stale_target).expect("stale target must be creatable");

        let workspace_dir = scratch_dir.join("workspaces").join("nro");
        std::fs::create_dir_all(&workspace_dir).expect("workspace dir must be creatable");

        // Pre-populate a stale link to prove it gets replaced.
        let stale_link_path = workspace_dir.join("alpha");
        std::os::unix::fs::symlink(&stale_target, &stale_link_path)
            .expect("stale link must be creatable");

        let member_links = vec![WorkspaceLinkSpec {
            link_name: "alpha".to_owned(),
            target_absolute_path: real_target.clone(),
        }];

        materialize_workspace_layout(&WorkspaceLayoutParameters {
            workspace_directory: &workspace_dir,
            workspace_name: "nro",
            member_links: &member_links,
            window_title_override: None,
            title_bar_color_hex: None,
        })
        .expect("first run must succeed");

        let resolved_after_first =
            std::fs::read_link(&stale_link_path).expect("link must resolve");
        assert_eq!(
            resolved_after_first, real_target,
            "stale link must have been replaced with the real target",
        );

        // Second run is a no-op — must still succeed.
        materialize_workspace_layout(&WorkspaceLayoutParameters {
            workspace_directory: &workspace_dir,
            workspace_name: "nro",
            member_links: &member_links,
            window_title_override: None,
            title_bar_color_hex: None,
        })
        .expect("second run must be idempotent");

        let resolved_after_second =
            std::fs::read_link(&stale_link_path).expect("link must still resolve");
        assert_eq!(resolved_after_second, real_target);

        let _ = std::fs::remove_dir_all(&scratch_dir);
    }

    #[test]
    #[cfg(unix)]
    fn materialize_layout_refuses_to_clobber_real_file_at_link_path() {
        let scratch_dir = unique_temp_dir("clobber-guard");
        let real_target = scratch_dir.join("repos").join("real");
        std::fs::create_dir_all(&real_target).expect("real target must be creatable");

        let workspace_dir = scratch_dir.join("workspaces").join("nro");
        std::fs::create_dir_all(&workspace_dir).expect("workspace dir must be creatable");

        // Pre-populate a real file at the link path.
        let occupied_link_path = workspace_dir.join("alpha");
        std::fs::write(&occupied_link_path, b"user data the tool must not destroy\n")
            .expect("scratch file must be writable");

        let member_links = vec![WorkspaceLinkSpec {
            link_name: "alpha".to_owned(),
            target_absolute_path: real_target.clone(),
        }];

        let outcome = materialize_workspace_layout(&WorkspaceLayoutParameters {
            workspace_directory: &workspace_dir,
            workspace_name: "nro",
            member_links: &member_links,
            window_title_override: None,
            title_bar_color_hex: None,
        });

        assert!(
            matches!(outcome, Err(ReportalError::ValidationFailure { .. })),
            "must refuse to clobber a real file at a link path, got: {outcome:?}",
        );
        let file_contents =
            std::fs::read(&occupied_link_path).expect("file must still exist after refusal");
        assert_eq!(file_contents, b"user data the tool must not destroy\n");

        let _ = std::fs::remove_dir_all(&scratch_dir);
    }

    #[test]
    fn purge_workspace_directory_is_noop_on_missing_dir() {
        let scratch_dir = unique_temp_dir("purge-missing");
        let nonexistent = scratch_dir.join("never-created");
        purge_workspace_directory(&nonexistent).expect("purge of missing dir must succeed");
        let _ = std::fs::remove_dir_all(&scratch_dir);
    }

    #[test]
    fn workspace_file_path_inside_dir_composes_expected_name() {
        let workspace_dir = PathBuf::from("/workspaces/nro");
        let file_path = workspace_file_path_inside_dir(&workspace_dir, "nro");
        assert_eq!(file_path, PathBuf::from("/workspaces/nro/nro.code-workspace"));
    }
}

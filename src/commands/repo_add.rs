/// Interactively registers a new repo in the RePortal config.

use crate::error::ReportalError;
use crate::reportal_config::{RepoRegistrationBuilder, ReportalConfig};
use dialoguer::Input;

/// Prompts the user for repo details, validates them, and saves to config.
///
/// Takes a filesystem path as the starting point, then asks for
/// alias, description, tags, and remote URL interactively.
pub fn run_add(filesystem_path: &str) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_from_disk()?;

    let repo_alias: String = Input::new()
        .with_prompt("Alias")
        .interact_text()
        .map_err(|prompt_error| ReportalError::ConfigIoFailure {
            reason: prompt_error.to_string(),
        })?;

    let repo_description: String = Input::new()
        .with_prompt("Description")
        .default(String::new())
        .interact_text()
        .map_err(|prompt_error| ReportalError::ConfigIoFailure {
            reason: prompt_error.to_string(),
        })?;

    let tags_input: String = Input::new()
        .with_prompt("Tags (comma-separated)")
        .default(String::new())
        .interact_text()
        .map_err(|prompt_error| ReportalError::ConfigIoFailure {
            reason: prompt_error.to_string(),
        })?;

    let parsed_tags: Vec<String> = tags_input
        .split(',')
        .map(|tag_segment| tag_segment.trim().to_string())
        .filter(|trimmed_tag| !trimmed_tag.is_empty())
        .collect();

    let repo_remote: String = Input::new()
        .with_prompt("Remote URL")
        .default(String::new())
        .interact_text()
        .map_err(|prompt_error| ReportalError::ConfigIoFailure {
            reason: prompt_error.to_string(),
        })?;

    let validated_registration = RepoRegistrationBuilder::start(repo_alias)
        .repo_path(filesystem_path.to_string())
        .repo_description(repo_description)
        .repo_tags(parsed_tags)
        .repo_remote(repo_remote)
        .build()?;

    let (ref registered_alias, _) = validated_registration;
    println!("Registered '{}' at {}", registered_alias, filesystem_path);

    loaded_config.add_repo(validated_registration)?;
    loaded_config.save_to_disk()?;

    Ok(())
}

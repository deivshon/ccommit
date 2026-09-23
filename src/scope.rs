use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::jsonc;

pub const NO_SCOPE: &str = "No scope";
pub const NEW_SCOPE: &str = "New scope";
pub const NEW_SCOPE_ONCE: &str = "New scope (only use once)";

const SCOPES_KEY: &str = "conventionalCommits.scopes";
const SETTINGS_PATH: &str = ".vscode/settings.json";

#[derive(Debug, Default, Deserialize)]
struct Settings {
    #[serde(rename = "conventionalCommits.scopes", default)]
    scopes: Vec<String>,
}

/// The path is relative to the repository root, not to the process directory.
pub fn settings_path(repository_root: &Path) -> PathBuf {
    repository_root.join(SETTINGS_PATH)
}

/// A missing or unreadable file gives no scopes, because the file is optional metadata.
pub fn read_scopes(repository_root: &Path) -> Vec<String> {
    let path = settings_path(repository_root);
    jsonc::read::<Settings>(&path)
        .unwrap_or_default()
        .unwrap_or_default()
        .scopes
}

/// Adds one scope to the settings file and keeps every comment.
pub fn add_scope(repository_root: &Path, scope: &str) -> Result<(), String> {
    let path = settings_path(repository_root);
    jsonc::edit_file(&path, |text| {
        jsonc::append_to_string_array(text, SCOPES_KEY, scope)
    })
}

/// The scopes from the settings file come first, then the three fixed rows.
pub fn menu_entries(repository_root: &Path) -> Vec<String> {
    let mut entries = read_scopes(repository_root);
    entries.push(NO_SCOPE.to_string());
    entries.push(NEW_SCOPE.to_string());
    entries.push(NEW_SCOPE_ONCE.to_string());
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_directory(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("ccommit-scope-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&directory).ok();
        std::fs::create_dir_all(directory.join(".vscode")).expect("the directory is created");
        directory
    }

    #[test]
    fn a_missing_file_gives_only_the_three_fixed_rows() {
        let directory = temporary_directory("missing");
        assert_eq!(
            menu_entries(&directory),
            vec![NO_SCOPE, NEW_SCOPE, NEW_SCOPE_ONCE]
        );
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_file_with_comments_still_gives_its_scopes() {
        let directory = temporary_directory("comments");
        std::fs::write(
            settings_path(&directory),
            "{\n  // the scopes we use\n  \"conventionalCommits.scopes\": [\n    \"ui\",\n    \"api\",\n  ],\n  \"editor.tabSize\": 2\n}\n",
        )
        .expect("the write works");

        assert_eq!(
            menu_entries(&directory),
            vec!["ui", "api", NO_SCOPE, NEW_SCOPE, NEW_SCOPE_ONCE]
        );
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn adding_a_scope_keeps_the_comments() {
        let directory = temporary_directory("add");
        let path = settings_path(&directory);
        std::fs::write(
            &path,
            "{\n  // the scopes we use\n  \"conventionalCommits.scopes\": [\n    \"ui\"\n  ]\n}\n",
        )
        .expect("the write works");

        add_scope(&directory, "cli").expect("the scope is added");

        let text = std::fs::read_to_string(&path).expect("the file reads");
        assert!(text.contains("// the scopes we use"), "{text}");
        assert_eq!(read_scopes(&directory), vec!["ui", "cli"]);
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn adding_a_scope_builds_an_absent_file() {
        let directory = temporary_directory("build");
        add_scope(&directory, "cli").expect("the scope is added");
        assert_eq!(read_scopes(&directory), vec!["cli"]);
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn adding_a_scope_creates_an_absent_key() {
        let directory = temporary_directory("key");
        let path = settings_path(&directory);
        std::fs::write(&path, "{\n  \"editor.tabSize\": 2\n}\n").expect("the write works");

        add_scope(&directory, "cli").expect("the scope is added");

        let text = std::fs::read_to_string(&path).expect("the file reads");
        assert!(text.contains("editor.tabSize"), "{text}");
        assert_eq!(read_scopes(&directory), vec!["cli"]);
        std::fs::remove_dir_all(&directory).ok();
    }
}

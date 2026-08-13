//! Reads, edits, and writes JSON files that hold comments.
//!
//! Every edit goes through the lossless syntax tree of `jsonc_parser`, so comments and
//! formatting outside the edited value survive.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use jsonc_parser::ParseOptions;
use jsonc_parser::cst::{CstInputValue, CstRootNode};

/// Sets one top-level key to a raw JSON literal.
pub fn set_object_key(text: &str, key: &str, literal: &str) -> Result<String, String> {
    let value = input_value(literal)?;

    edit(text, |root| {
        let object = root_object(root)?;
        match object.get(key) {
            Some(property) => property.set_value(value),
            None => {
                object.append(key, value);
            }
        }
        Ok(())
    })
}

/// Appends one string to the array under a top-level key.
pub fn append_to_string_array(text: &str, key: &str, item: &str) -> Result<String, String> {
    edit(text, |root| {
        let array = root_object(root)?
            .array_value_or_create(key)
            .ok_or_else(|| format!("`{key}` is not an array"))?;
        array.append(CstInputValue::String(item.to_string()));
        Ok(())
    })
}

/// Parses the text, applies the edit to the tree, and gives the text back.
fn edit(
    text: &str,
    apply: impl FnOnce(&CstRootNode) -> Result<(), String>,
) -> Result<String, String> {
    let root =
        CstRootNode::parse(text, &ParseOptions::default()).map_err(|error| error.to_string())?;
    apply(&root)?;
    Ok(root.to_string())
}

/// The root object, created when the file is empty.
fn root_object(root: &CstRootNode) -> Result<jsonc_parser::cst::CstObject, String> {
    root.object_value_or_create()
        .ok_or_else(|| "the root value is not an object".to_string())
}

fn input_value(literal: &str) -> Result<CstInputValue, String> {
    let value: serde_json::Value = serde_json::from_str(literal)
        .map_err(|error| format!("`{literal}` is not valid JSON: {error}"))?;
    Ok(convert(value))
}

fn convert(value: serde_json::Value) -> CstInputValue {
    match value {
        serde_json::Value::Null => CstInputValue::Null,
        serde_json::Value::Bool(value) => CstInputValue::Bool(value),
        serde_json::Value::Number(value) => CstInputValue::Number(value.to_string()),
        serde_json::Value::String(value) => CstInputValue::String(value),
        serde_json::Value::Array(values) => {
            CstInputValue::Array(values.into_iter().map(convert).collect())
        }
        serde_json::Value::Object(map) => CstInputValue::Object(
            map.into_iter()
                .map(|(key, value)| (key, convert(value)))
                .collect(),
        ),
    }
}

/// Reads the file, applies the edit to its text, and writes the result back in one atomic step.
///
/// A missing file edits the empty string, so the edit can build the file from nothing.
pub fn edit_file(
    path: &Path,
    edit: impl FnOnce(&str) -> Result<String, String>,
) -> Result<(), String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(format!("Could not read `{}`: {error}", path.display())),
    };

    let updated =
        edit(&text).map_err(|error| format!("Could not update `{}`: {error}", path.display()))?;
    write_atomic(path, &updated)
}

/// Writes through a temporary file in the same directory, so a crash cannot destroy the target.
pub fn write_atomic(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create `{}`: {error}", parent.display()))?;
    }

    let temporary = temporary_path(path);
    let write = || -> std::io::Result<()> {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()
    };

    if let Err(error) = write() {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "Could not write `{}`: {error}",
            temporary.display()
        ));
    }

    fs::rename(&temporary, path)
        .map_err(|error| format!("Could not replace `{}`: {error}", path.display()))
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".ccommit-{}.tmp", std::process::id()));
    path.with_file_name(name)
}

/// Reads a JSON file that can hold comments. A missing file gives `None`.
pub fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Could not read `{}`: {error}", path.display())),
    };

    if text.trim().is_empty() {
        return Ok(None);
    }

    jsonc_parser::parse_to_serde_value(&text, &ParseOptions::default())
        .map(Some)
        .map_err(|error| format!("Could not parse `{}`: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_replaces_an_existing_value_and_keeps_the_comments() {
        let text = "{\n  // keep me\n  \"real_emoji\": true,\n  \"gitmojis\": null\n}\n";
        let result = set_object_key(text, "real_emoji", "false").expect("the edit works");
        assert_eq!(
            result,
            "{\n  // keep me\n  \"real_emoji\": false,\n  \"gitmojis\": null\n}\n"
        );
    }

    #[test]
    fn set_adds_an_absent_key_after_the_last_property() {
        let text = "{\n    \"a\": 1 // trailing\n}\n";
        let result = set_object_key(text, "real_emoji", "true").expect("the edit works");
        assert_eq!(
            result,
            "{\n    \"a\": 1, // trailing\n    \"real_emoji\": true\n}\n"
        );
    }

    #[test]
    fn set_keeps_a_comment_after_a_trailing_comma_on_its_line() {
        let text = "{\n  \"a\": 1, // trailing\n}\n";
        let result = set_object_key(text, "b", "2").expect("the edit works");
        assert_eq!(result, "{\n  \"a\": 1, // trailing\n  \"b\": 2,\n}\n");
    }

    #[test]
    fn set_fills_an_empty_object() {
        let result = set_object_key("{}", "real_emoji", "true").expect("the edit works");
        assert_eq!(result, "{\n  \"real_emoji\": true\n}");
    }

    #[test]
    fn set_builds_a_file_from_nothing() {
        let result = set_object_key("", "real_emoji", "true").expect("the edit works");
        assert_eq!(result, "{\n  \"real_emoji\": true\n}\n");
    }

    #[test]
    fn set_keeps_a_trailing_comma_valid() {
        let text = "{\n  \"a\": 1,\n}";
        let result = set_object_key(text, "b", "2").expect("the edit works");
        assert_eq!(result, "{\n  \"a\": 1,\n  \"b\": 2,\n}");
    }

    #[test]
    fn set_refuses_a_root_that_is_not_an_object() {
        let error = set_object_key("[1, 2]", "a", "1").expect_err("the edit fails");
        assert!(error.contains("not an object"), "{error}");
    }

    #[test]
    fn append_extends_a_multi_line_array() {
        let text = "{\n  // scopes\n  \"conventionalCommits.scopes\": [\n    \"ui\",\n    \"api\"\n  ]\n}\n";
        let result =
            append_to_string_array(text, "conventionalCommits.scopes", "cli").expect("it appends");
        assert_eq!(
            result,
            "{\n  // scopes\n  \"conventionalCommits.scopes\": [\n    \"ui\",\n    \"api\",\n    \"cli\"\n  ]\n}\n"
        );
    }

    #[test]
    fn append_extends_a_single_line_array() {
        let text = "{ \"conventionalCommits.scopes\": [\"ui\"] }";
        let result =
            append_to_string_array(text, "conventionalCommits.scopes", "cli").expect("it appends");
        assert_eq!(
            result,
            "{ \"conventionalCommits.scopes\": [\"ui\", \"cli\"] }"
        );
    }

    #[test]
    fn append_keeps_a_comment_on_the_last_element_line() {
        let text = "{\n  \"conventionalCommits.scopes\": [\n    \"ui\" // the main one\n  ]\n}\n";
        let result =
            append_to_string_array(text, "conventionalCommits.scopes", "cli").expect("it appends");
        assert_eq!(
            result,
            "{\n  \"conventionalCommits.scopes\": [\n    \"ui\", // the main one\n    \"cli\"\n  ]\n}\n"
        );
    }

    #[test]
    fn append_fills_an_empty_array() {
        let text = "{\n  \"conventionalCommits.scopes\": []\n}\n";
        let result =
            append_to_string_array(text, "conventionalCommits.scopes", "cli").expect("it appends");
        assert_eq!(
            result,
            "{\n  \"conventionalCommits.scopes\": [\"cli\"]\n}\n"
        );
    }

    #[test]
    fn append_creates_an_absent_key() {
        let text = "{\n  // editor\n  \"editor.tabSize\": 2\n}\n";
        let result =
            append_to_string_array(text, "conventionalCommits.scopes", "cli").expect("it appends");
        assert_eq!(
            result,
            "{\n  // editor\n  \"editor.tabSize\": 2,\n  \"conventionalCommits.scopes\": [\"cli\"]\n}\n"
        );
    }

    #[test]
    fn append_escapes_the_new_item() {
        let text = "{\n  \"conventionalCommits.scopes\": []\n}\n";
        let result =
            append_to_string_array(text, "conventionalCommits.scopes", "a\"b").expect("it appends");
        assert!(result.contains("[\"a\\\"b\"]"), "{result}");
    }

    #[test]
    fn append_refuses_a_value_that_is_not_an_array() {
        let text = "{ \"conventionalCommits.scopes\": 5 }";
        let error = append_to_string_array(text, "conventionalCommits.scopes", "cli")
            .expect_err("it fails");
        assert!(error.contains("not an array"), "{error}");
    }

    #[test]
    fn a_trailing_comma_and_a_comment_do_not_stop_the_read() {
        let directory = std::env::temp_dir().join(format!("ccommit-jsonc-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("the directory is created");
        let path = directory.join("settings.json");
        write_atomic(&path, "{\n  // note\n  \"a\": 1,\n}\n").expect("the write works");

        let value: Option<serde_json::Value> = read(&path).expect("the read works");
        assert_eq!(value.expect("a value")["a"], 1);

        let missing: Option<serde_json::Value> =
            read(&directory.join("absent.json")).expect("a missing file gives none");
        assert!(missing.is_none());

        fs::remove_dir_all(&directory).ok();
    }
}

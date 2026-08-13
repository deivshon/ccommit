use std::fs;
use std::path::Path;

use serde::Deserialize;

const EMBEDDED_COMMIT_TYPES: &str = include_str!("../data/commit-types.json");
const EMBEDDED_GITMOJIS: &str = include_str!("../data/gitmojis.json");

#[derive(Debug, Clone, Deserialize)]
pub struct Entry {
    pub name: String,
    #[serde(default)]
    pub long: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub emoji: Option<String>,
}

impl Entry {
    pub fn menu_entry(&self) -> String {
        if self.long.is_empty() {
            format!("{} ({})", self.name, self.description)
        } else if self.description.is_empty() {
            format!("{} ({})", self.name, self.long)
        } else {
            format!("{} ({}; {})", self.name, self.long, self.description)
        }
    }

    /// The gitmoji menu shows the picture next to the code.
    pub fn menu_entry_with_emoji(&self) -> String {
        match &self.emoji {
            Some(emoji) if !emoji.is_empty() => format!("{emoji}  {}", self.menu_entry()),
            _ => self.menu_entry(),
        }
    }

    pub fn selection(&self, real_emoji: bool) -> String {
        match &self.emoji {
            Some(emoji) if real_emoji && !emoji.is_empty() => emoji.clone(),
            _ => self.name.clone(),
        }
    }
}

pub fn commit_types(path: Option<&Path>) -> Result<Vec<Entry>, String> {
    load(path, EMBEDDED_COMMIT_TYPES)
}

pub fn gitmojis(path: Option<&Path>) -> Result<Vec<Entry>, String> {
    load(path, EMBEDDED_GITMOJIS)
}

fn load(path: Option<&Path>, embedded: &str) -> Result<Vec<Entry>, String> {
    match path {
        None => parse(embedded, "the embedded data"),
        Some(path) => {
            let text = fs::read_to_string(path)
                .map_err(|error| format!("Could not read `{}`: {error}", path.display()))?;
            parse(&text, &path.display().to_string())
        }
    }
}

fn parse(text: &str, source: &str) -> Result<Vec<Entry>, String> {
    serde_json::from_str(text).map_err(|error| format!("Could not parse `{source}`: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_data_parses() {
        let types = commit_types(None).expect("the embedded commit types parse");
        assert!(!types.is_empty());
        assert!(types.iter().any(|entry| entry.name == "feat"));

        let gitmojis = gitmojis(None).expect("the embedded gitmojis parse");
        assert!(!gitmojis.is_empty());
        assert!(gitmojis.iter().all(|entry| entry.emoji.is_some()));
    }

    #[test]
    fn no_gitmoji_description_ends_with_a_period() {
        for entry in gitmojis(None).expect("the embedded gitmojis parse") {
            assert!(
                !entry.description.ends_with('.'),
                "`{}` ends with a period",
                entry.name
            );
        }
    }

    #[test]
    fn menu_entry_drops_the_empty_part() {
        let entry = Entry {
            name: "feat".to_string(),
            long: "Feature".to_string(),
            description: "A new feature".to_string(),
            emoji: None,
        };
        assert_eq!(entry.menu_entry(), "feat (Feature; A new feature)");

        let entry = Entry {
            long: String::new(),
            ..entry
        };
        assert_eq!(entry.menu_entry(), "feat (A new feature)");

        let entry = Entry {
            long: "Feature".to_string(),
            description: String::new(),
            ..entry
        };
        assert_eq!(entry.menu_entry(), "feat (Feature)");
    }

    #[test]
    fn selection_follows_the_real_emoji_setting() {
        let entry = Entry {
            name: ":art:".to_string(),
            long: String::new(),
            description: "Improve structure".to_string(),
            emoji: Some("🎨".to_string()),
        };
        assert_eq!(entry.selection(true), "🎨");
        assert_eq!(entry.selection(false), ":art:");

        let entry = Entry {
            emoji: None,
            ..entry
        };
        assert_eq!(entry.selection(true), ":art:");
    }

    #[test]
    fn a_missing_file_is_an_error() {
        let error = commit_types(Some(Path::new("/nonexistent/types.json")))
            .expect_err("a missing file fails");
        assert!(error.contains("/nonexistent/types.json"), "{error}");
    }
}

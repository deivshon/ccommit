use std::env;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::jsonc;
use crate::message::{MessageLimit, NO_LIMIT};
use crate::theme::{self, Colors, HexColor};

/// The literal that stands for the built-in default, so an absent key means the same as this one.
const NULL: &str = "null";

pub const DEFAULT_COMMIT_ON_FINISH: bool = true;
pub const DEFAULT_PREVIEW_ON_FINISH: bool = true;
pub const DEFAULT_REAL_EMOJI: bool = true;
pub const DEFAULT_SCOPE_STEP: bool = true;
pub const DEFAULT_GITMOJI_STEP: bool = true;
pub const DEFAULT_LONG_DESCRIPTION_STEP: bool = true;
pub const DEFAULT_BREAKING_CHANGE_STEP: bool = true;
pub const DEFAULT_DENY_CLI_LONG_MESSAGE: bool = false;

pub const KEY_COMMIT_TYPES: &str = "commit_types";
pub const KEY_GITMOJIS: &str = "gitmojis";
pub const KEY_COMMIT_ON_FINISH: &str = "commit_on_finish";
pub const KEY_PREVIEW_ON_FINISH: &str = "preview_on_finish";
pub const KEY_REAL_EMOJI: &str = "real_emoji";
pub const KEY_SCOPE_STEP: &str = "scope_step";
pub const KEY_GITMOJI_STEP: &str = "gitmoji_step";
pub const KEY_LONG_DESCRIPTION_STEP: &str = "long_description_step";
pub const KEY_BREAKING_CHANGE_STEP: &str = "breaking_change_step";
pub const KEY_DENY_CLI_LONG_MESSAGE: &str = "deny_cli_long_message";
pub const KEY_ACCENT_COLOR: &str = "accent_color";
pub const KEY_MUTED_COLOR: &str = "muted_color";
pub const KEY_SUCCESS_COLOR: &str = "success_color";
pub const KEY_WARNING_COLOR: &str = "warning_color";
pub const KEY_ERROR_COLOR: &str = "error_color";
pub const KEY_SHORT_MESSAGE_LIMIT: &str = "short_message_limit";

/// The keys as one source gives them. `None` means that source said nothing.
///
/// The command line and the configuration file each give one `Partial`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Partial {
    pub commit_types: Option<PathBuf>,
    pub gitmojis: Option<PathBuf>,
    pub commit_on_finish: Option<bool>,
    pub preview_on_finish: Option<bool>,
    pub real_emoji: Option<bool>,
    pub scope_step: Option<bool>,
    pub gitmoji_step: Option<bool>,
    pub long_description_step: Option<bool>,
    pub breaking_change_step: Option<bool>,
    pub deny_cli_long_message: Option<bool>,
    pub accent_color: Option<HexColor>,
    pub muted_color: Option<HexColor>,
    pub success_color: Option<HexColor>,
    pub warning_color: Option<HexColor>,
    pub error_color: Option<HexColor>,
    pub short_message_limit: Option<MessageLimit>,
}

impl Partial {
    /// Field-wise precedence: `self` wins, and `other` fills the gaps.
    fn or(self, other: Partial) -> Partial {
        Partial {
            commit_types: self.commit_types.or(other.commit_types),
            gitmojis: self.gitmojis.or(other.gitmojis),
            commit_on_finish: self.commit_on_finish.or(other.commit_on_finish),
            preview_on_finish: self.preview_on_finish.or(other.preview_on_finish),
            real_emoji: self.real_emoji.or(other.real_emoji),
            scope_step: self.scope_step.or(other.scope_step),
            gitmoji_step: self.gitmoji_step.or(other.gitmoji_step),
            long_description_step: self.long_description_step.or(other.long_description_step),
            breaking_change_step: self.breaking_change_step.or(other.breaking_change_step),
            deny_cli_long_message: self.deny_cli_long_message.or(other.deny_cli_long_message),
            accent_color: self.accent_color.or(other.accent_color),
            muted_color: self.muted_color.or(other.muted_color),
            success_color: self.success_color.or(other.success_color),
            warning_color: self.warning_color.or(other.warning_color),
            error_color: self.error_color.or(other.error_color),
            short_message_limit: self.short_message_limit.or(other.short_message_limit),
        }
    }
}

/// The values the program runs with, after the merge.
///
/// A color is `None` when the user set nothing, as an embedded data file is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub commit_types: Option<PathBuf>,
    pub gitmojis: Option<PathBuf>,
    pub commit_on_finish: bool,
    pub preview_on_finish: bool,
    pub real_emoji: bool,
    pub scope_step: bool,
    pub gitmoji_step: bool,
    pub long_description_step: bool,
    pub breaking_change_step: bool,
    pub deny_cli_long_message: bool,
    pub accent_color: Option<HexColor>,
    pub muted_color: Option<HexColor>,
    pub success_color: Option<HexColor>,
    pub warning_color: Option<HexColor>,
    pub error_color: Option<HexColor>,
    pub short_message_limit: Option<MessageLimit>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            commit_types: None,
            gitmojis: None,
            commit_on_finish: DEFAULT_COMMIT_ON_FINISH,
            preview_on_finish: DEFAULT_PREVIEW_ON_FINISH,
            real_emoji: DEFAULT_REAL_EMOJI,
            scope_step: DEFAULT_SCOPE_STEP,
            gitmoji_step: DEFAULT_GITMOJI_STEP,
            long_description_step: DEFAULT_LONG_DESCRIPTION_STEP,
            breaking_change_step: DEFAULT_BREAKING_CHANGE_STEP,
            deny_cli_long_message: DEFAULT_DENY_CLI_LONG_MESSAGE,
            accent_color: None,
            muted_color: None,
            success_color: None,
            warning_color: None,
            error_color: None,
            short_message_limit: None,
        }
    }
}

impl Config {
    /// The limit of the first line, with the built-in default where the user set nothing.
    pub fn message_limit(&self) -> MessageLimit {
        self.short_message_limit.unwrap_or_default()
    }

    /// The color of each role, with the ANSI default where the user set nothing.
    pub fn colors(&self) -> Colors {
        Colors {
            accent: theme::or_default(self.accent_color, theme::DEFAULT_ACCENT),
            muted: theme::or_default(self.muted_color, theme::DEFAULT_MUTED),
            success: theme::or_default(self.success_color, theme::DEFAULT_SUCCESS),
            warning: theme::or_default(self.warning_color, theme::DEFAULT_WARNING),
            error: theme::or_default(self.error_color, theme::DEFAULT_ERROR),
        }
    }
}

/// Precedence is the command line, then the file, then the built-in default.
pub fn merge(overrides: Partial, file: Partial) -> Config {
    let merged = overrides.or(file);

    Config {
        commit_types: real_path(merged.commit_types),
        gitmojis: real_path(merged.gitmojis),
        commit_on_finish: merged.commit_on_finish.unwrap_or(DEFAULT_COMMIT_ON_FINISH),
        preview_on_finish: merged
            .preview_on_finish
            .unwrap_or(DEFAULT_PREVIEW_ON_FINISH),
        real_emoji: merged.real_emoji.unwrap_or(DEFAULT_REAL_EMOJI),
        scope_step: merged.scope_step.unwrap_or(DEFAULT_SCOPE_STEP),
        gitmoji_step: merged.gitmoji_step.unwrap_or(DEFAULT_GITMOJI_STEP),
        long_description_step: merged
            .long_description_step
            .unwrap_or(DEFAULT_LONG_DESCRIPTION_STEP),
        breaking_change_step: merged
            .breaking_change_step
            .unwrap_or(DEFAULT_BREAKING_CHANGE_STEP),
        deny_cli_long_message: merged
            .deny_cli_long_message
            .unwrap_or(DEFAULT_DENY_CLI_LONG_MESSAGE),
        accent_color: merged.accent_color,
        muted_color: merged.muted_color,
        success_color: merged.success_color,
        warning_color: merged.warning_color,
        error_color: merged.error_color,
        short_message_limit: merged.short_message_limit,
    }
}

/// The values the configuration menu shows.
///
/// The menu reads and writes the file, so the command line stays out of the merge.
pub fn from_file(file: Partial) -> Config {
    merge(Partial::default(), file)
}

/// An empty path means the embedded data.
fn real_path(path: Option<PathBuf>) -> Option<PathBuf> {
    path.filter(|path| !path.as_os_str().is_empty())
}

pub fn default_path() -> Option<PathBuf> {
    if let Some(xdg) = env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(xdg).join("ccommit/config.json"));
    }

    env::home_dir().map(|home| home.join(".config/ccommit/config.json"))
}

/// Gives the file the program reads and the configuration menu writes.
///
/// A path from the command line must exist. The default path can be absent.
pub fn resolve_path(explicit: Option<&Path>) -> Result<Option<PathBuf>, String> {
    match explicit {
        Some(path) => {
            if path.is_file() {
                Ok(Some(path.to_path_buf()))
            } else {
                Err(format!("No config at `{}`", absolute(path).display()))
            }
        }
        None => Ok(default_path()),
    }
}

/// A missing file gives the defaults. A malformed file is a hard failure.
pub fn load(path: &Path) -> Result<Partial, String> {
    Ok(jsonc::read(path)?.unwrap_or_default())
}

/// Writes every key, so the file the menu leaves behind is complete.
///
/// A key the file already holds keeps its place, and the comments around it survive. The program
/// never reads the file as complete, so a key a person erases by hand gives the default again.
pub fn write(path: &Path, config: &Config) -> Result<(), String> {
    jsonc::edit_file(path, |text| {
        literals(config)
            .iter()
            .try_fold(text.to_string(), |text, (key, literal)| {
                jsonc::set_object_key(&text, key, literal)
            })
    })
}

/// Every key with its JSON literal, in the order the file receives them.
fn literals(config: &Config) -> Vec<(&'static str, String)> {
    vec![
        (KEY_COMMIT_TYPES, path_literal(config.commit_types.as_ref())),
        (KEY_GITMOJIS, path_literal(config.gitmojis.as_ref())),
        (KEY_COMMIT_ON_FINISH, config.commit_on_finish.to_string()),
        (KEY_PREVIEW_ON_FINISH, config.preview_on_finish.to_string()),
        (KEY_REAL_EMOJI, config.real_emoji.to_string()),
        (KEY_SCOPE_STEP, config.scope_step.to_string()),
        (KEY_GITMOJI_STEP, config.gitmoji_step.to_string()),
        (
            KEY_LONG_DESCRIPTION_STEP,
            config.long_description_step.to_string(),
        ),
        (
            KEY_BREAKING_CHANGE_STEP,
            config.breaking_change_step.to_string(),
        ),
        (
            KEY_DENY_CLI_LONG_MESSAGE,
            config.deny_cli_long_message.to_string(),
        ),
        (
            KEY_SHORT_MESSAGE_LIMIT,
            limit_literal(config.short_message_limit),
        ),
        (KEY_ACCENT_COLOR, color_literal(config.accent_color)),
        (KEY_MUTED_COLOR, color_literal(config.muted_color)),
        (KEY_SUCCESS_COLOR, color_literal(config.success_color)),
        (KEY_WARNING_COLOR, color_literal(config.warning_color)),
        (KEY_ERROR_COLOR, color_literal(config.error_color)),
    ]
}

fn path_literal(path: Option<&PathBuf>) -> String {
    path.and_then(|path| serde_json::to_string(path).ok())
        .unwrap_or_else(|| NULL.to_string())
}

fn color_literal(color: Option<HexColor>) -> String {
    color.map_or_else(|| NULL.to_string(), |color| format!("\"{color}\""))
}

fn limit_literal(limit: Option<MessageLimit>) -> String {
    match limit.map(MessageLimit::total) {
        Some(Some(characters)) => characters.to_string(),
        Some(None) => format!("\"{NO_LIMIT}\""),
        None => NULL.to_string(),
    }
}

fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }

    env::current_dir()
        .map(|current| current.join(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::*;
    use crate::message::MessageLimit;

    fn hex(text: &str) -> HexColor {
        text.parse().expect("the color parses")
    }

    #[test]
    fn the_built_in_defaults_win_when_nothing_else_speaks() {
        let config = merge(Partial::default(), Partial::default());
        assert!(config.commit_on_finish);
        assert!(config.preview_on_finish);
        assert!(config.real_emoji);
        assert!(config.scope_step);
        assert!(config.gitmoji_step);
        assert!(config.long_description_step);
        assert!(config.breaking_change_step);
        assert!(!config.deny_cli_long_message);
        assert_eq!(config.commit_types, None);
        assert_eq!(config.gitmojis, None);
        assert_eq!(config.colors(), Colors::default());
    }

    #[test]
    fn the_file_beats_the_built_in_default() {
        let file = Partial {
            real_emoji: Some(false),
            commit_on_finish: Some(false),
            gitmojis: Some(PathBuf::from("/data/gitmojis.json")),
            ..Partial::default()
        };
        let config = merge(Partial::default(), file);
        assert!(!config.real_emoji);
        assert!(!config.commit_on_finish);
        assert!(config.preview_on_finish);
        assert_eq!(config.gitmojis, Some(PathBuf::from("/data/gitmojis.json")));
    }

    #[test]
    fn the_command_line_beats_the_file() {
        let file = Partial {
            real_emoji: Some(false),
            commit_on_finish: Some(false),
            gitmojis: Some(PathBuf::from("/data/from-file.json")),
            ..Partial::default()
        };
        let overrides = Partial {
            real_emoji: Some(true),
            gitmojis: Some(PathBuf::from("/data/from-cli.json")),
            ..Partial::default()
        };
        let config = merge(overrides, file);
        assert!(config.real_emoji);
        assert!(!config.commit_on_finish, "the file still decides this key");
        assert_eq!(config.gitmojis, Some(PathBuf::from("/data/from-cli.json")));
    }

    #[test]
    fn the_menu_values_come_from_the_file_alone() {
        let file = Partial {
            real_emoji: Some(false),
            commit_on_finish: Some(true),
            gitmojis: Some(PathBuf::from("/data/from-file.json")),
            ..Partial::default()
        };
        let config = from_file(file);
        assert!(!config.real_emoji);
        assert!(config.commit_on_finish);
        assert_eq!(config.gitmojis, Some(PathBuf::from("/data/from-file.json")));
    }

    #[test]
    fn a_step_key_travels_from_the_file_and_the_command_line() {
        let file = Partial {
            scope_step: Some(false),
            gitmoji_step: Some(false),
            long_description_step: Some(false),
            breaking_change_step: Some(false),
            ..Partial::default()
        };
        let config = merge(Partial::default(), file.clone());
        assert!(!config.scope_step);
        assert!(!config.gitmoji_step);
        assert!(!config.long_description_step);
        assert!(!config.breaking_change_step);

        let overrides = Partial {
            breaking_change_step: Some(true),
            ..Partial::default()
        };
        let config = merge(overrides, file);
        assert!(config.breaking_change_step, "the flag wins");
        assert!(!config.long_description_step, "the file still decides this");
    }

    #[test]
    fn an_absent_key_gives_the_built_in_default_to_the_menu() {
        let config = from_file(Partial::default());
        assert!(config.real_emoji);
        assert!(config.preview_on_finish);
        assert_eq!(config.commit_types, None);
    }

    #[test]
    fn an_empty_path_means_the_embedded_data() {
        let file = Partial {
            commit_types: Some(PathBuf::from("")),
            ..Partial::default()
        };
        assert_eq!(merge(Partial::default(), file).commit_types, None);
    }

    #[test]
    fn an_absent_limit_holds_the_first_line_at_72_characters() {
        let config = merge(Partial::default(), Partial::default());
        assert_eq!(config.short_message_limit, None);
        assert_eq!(config.message_limit(), MessageLimit::default());
    }

    #[test]
    fn the_command_line_beats_the_file_for_the_limit() {
        let file = Partial {
            short_message_limit: Some(MessageLimit::NONE),
            ..Partial::default()
        };
        let overrides = Partial {
            short_message_limit: Some(MessageLimit::characters(80).expect("80 works")),
            ..Partial::default()
        };

        assert_eq!(
            merge(Partial::default(), file.clone()).message_limit(),
            MessageLimit::NONE
        );
        assert_eq!(
            merge(overrides, file).message_limit().total(),
            Some(80),
            "the flag wins"
        );
    }

    #[test]
    fn a_limit_key_travels_through_the_file() {
        let directory =
            std::env::temp_dir().join(format!("ccommit-config-limit-{}", std::process::id()));
        std::fs::remove_dir_all(&directory).ok();
        let path = directory.join("config.json");

        let mut config = Config {
            short_message_limit: MessageLimit::characters(80).ok(),
            ..Config::default()
        };
        write(&path, &config).expect("the write works");
        let file = load(&path).expect("the load works");
        assert_eq!(file.short_message_limit, MessageLimit::characters(80).ok());

        config.short_message_limit = Some(MessageLimit::NONE);
        write(&path, &config).expect("the second write works");
        let file = load(&path).expect("the load works");
        assert_eq!(file.short_message_limit, Some(MessageLimit::NONE));

        config.short_message_limit = None;
        write(&path, &config).expect("the third write works");
        let file = load(&path).expect("the load works");
        assert_eq!(file.short_message_limit, None, "null gives the default");

        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_limit_under_the_minimum_stops_the_load() {
        let directory =
            std::env::temp_dir().join(format!("ccommit-config-bad-limit-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("the directory is created");
        let path = directory.join("config.json");

        for literal in ["49", "-1", "\"wide\""] {
            std::fs::write(&path, format!("{{ \"short_message_limit\": {literal} }}"))
                .expect("the write works");

            let error = load(&path).expect_err("the load fails");
            assert!(error.contains("config.json"), "{error}");
            assert!(error.contains("50"), "{error}");
        }

        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_color_from_the_file_replaces_one_ansi_default() {
        let file = Partial {
            accent_color: Some(hex("#1e88e5")),
            ..Partial::default()
        };
        let colors = merge(Partial::default(), file).colors();

        assert_eq!(colors.accent, Color::Rgb(0x1e, 0x88, 0xe5));
        assert_eq!(colors.muted, theme::DEFAULT_MUTED, "the others stay ANSI");
        assert_eq!(colors.success, theme::DEFAULT_SUCCESS);
        assert_eq!(colors.warning, theme::DEFAULT_WARNING);
        assert_eq!(colors.error, theme::DEFAULT_ERROR);
    }

    #[test]
    fn the_command_line_beats_the_file_for_a_color() {
        let file = Partial {
            accent_color: Some(hex("#111111")),
            error_color: Some(hex("#222222")),
            ..Partial::default()
        };
        let overrides = Partial {
            accent_color: Some(hex("#333333")),
            ..Partial::default()
        };
        let colors = merge(overrides, file).colors();

        assert_eq!(colors.accent, Color::Rgb(0x33, 0x33, 0x33));
        assert_eq!(colors.error, Color::Rgb(0x22, 0x22, 0x22));
    }

    #[test]
    fn the_menu_color_values_come_from_the_file_alone() {
        let file = Partial {
            warning_color: Some(hex("#f0a")),
            ..Partial::default()
        };
        let config = from_file(file);

        assert_eq!(config.warning_color, Some(hex("#ff00aa")));
        assert_eq!(config.accent_color, None);
    }

    #[test]
    fn a_color_key_travels_through_the_file() {
        let directory =
            std::env::temp_dir().join(format!("ccommit-config-color-{}", std::process::id()));
        std::fs::remove_dir_all(&directory).ok();
        let path = directory.join("config.json");

        let mut config = Config {
            accent_color: Some(hex("#1e88e5")),
            ..Config::default()
        };
        write(&path, &config).expect("the write works");
        let file = load(&path).expect("the load works");
        assert_eq!(file.accent_color, Some(hex("#1e88e5")));

        config.accent_color = None;
        write(&path, &config).expect("the second write works");
        assert_eq!(load(&path).expect("the load works").accent_color, None);

        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_color_that_is_not_hex_stops_the_load() {
        let directory =
            std::env::temp_dir().join(format!("ccommit-config-bad-color-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("the directory is created");
        let path = directory.join("config.json");
        std::fs::write(&path, "{ \"accent_color\": \"blue\" }").expect("the write works");

        let error = load(&path).expect_err("the load fails");
        assert!(error.contains("config.json"), "{error}");
        assert!(error.contains("not a hex color"), "{error}");

        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn an_unknown_key_does_not_stop_the_load() {
        let directory = std::env::temp_dir().join(format!("ccommit-config-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("the directory is created");
        let path = directory.join("config.json");
        jsonc::write_atomic(&path, "{\n  // old key\n  \"use_emojis\": true\n}\n")
            .expect("the write works");

        let file = load(&path).expect("the load works");
        assert_eq!(file.real_emoji, None, "the old key is lost in silence");

        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_malformed_file_is_a_hard_failure() {
        let directory =
            std::env::temp_dir().join(format!("ccommit-config-bad-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("the directory is created");
        let path = directory.join("config.json");
        std::fs::write(&path, "{ this is not json").expect("the write works");

        let error = load(&path).expect_err("the load fails");
        assert!(error.contains("config.json"), "{error}");

        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_missing_file_gives_the_defaults() {
        let path = std::env::temp_dir().join("ccommit-absent-config.json");
        std::fs::remove_file(&path).ok();
        let file = load(&path).expect("the load works");
        assert_eq!(file.real_emoji, None);
    }

    #[test]
    fn write_creates_the_parent_directories() {
        let directory =
            std::env::temp_dir().join(format!("ccommit-config-new-{}", std::process::id()));
        std::fs::remove_dir_all(&directory).ok();
        let path = directory.join("nested/config.json");

        let config = Config {
            real_emoji: false,
            ..Config::default()
        };
        write(&path, &config).expect("the write works");
        assert_eq!(load(&path).expect("the load works").real_emoji, Some(false));

        std::fs::remove_dir_all(&directory).ok();
    }

    /// A key the file does not hold gives the default, so the writer can write `null`.
    #[test]
    fn one_write_puts_every_key_in_the_file() {
        let directory =
            std::env::temp_dir().join(format!("ccommit-config-whole-{}", std::process::id()));
        std::fs::remove_dir_all(&directory).ok();
        let path = directory.join("config.json");

        write(&path, &Config::default()).expect("the write works");

        let text = std::fs::read_to_string(&path).expect("the file exists");
        for (key, _) in literals(&Config::default()) {
            assert!(
                text.contains(&format!("\"{key}\"")),
                "{key} is absent:\n{text}"
            );
        }
        assert_eq!(
            from_file(load(&path).expect("the load works")),
            Config::default(),
            "the complete file of the defaults gives the defaults back"
        );

        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn every_value_survives_the_round_trip_through_the_file() {
        let directory =
            std::env::temp_dir().join(format!("ccommit-config-round-{}", std::process::id()));
        std::fs::remove_dir_all(&directory).ok();
        let path = directory.join("config.json");

        let config = Config {
            commit_types: Some(PathBuf::from("/data/types.json")),
            gitmojis: Some(PathBuf::from("/data/gitmojis.json")),
            commit_on_finish: false,
            preview_on_finish: false,
            real_emoji: false,
            scope_step: false,
            gitmoji_step: false,
            long_description_step: false,
            breaking_change_step: false,
            deny_cli_long_message: true,
            short_message_limit: MessageLimit::characters(80).ok(),
            accent_color: Some(hex("#111111")),
            muted_color: Some(hex("#222222")),
            success_color: Some(hex("#333333")),
            warning_color: Some(hex("#444444")),
            error_color: Some(hex("#555555")),
        };

        write(&path, &config).expect("the write works");
        assert_eq!(from_file(load(&path).expect("the load works")), config);

        std::fs::remove_dir_all(&directory).ok();
    }

    /// The file of a person holds what that person wrote, so the writer completes it in place.
    #[test]
    fn write_completes_a_file_and_keeps_the_comments() {
        let directory =
            std::env::temp_dir().join(format!("ccommit-config-partial-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("the directory is created");
        let path = directory.join("config.json");
        std::fs::write(
            &path,
            "{\n  // I want the gitmoji codes\n  \"real_emoji\": false\n}\n",
        )
        .expect("the write works");

        let config = Config {
            real_emoji: false,
            scope_step: false,
            ..Config::default()
        };
        write(&path, &config).expect("the write works");

        let text = std::fs::read_to_string(&path).expect("the file exists");
        assert!(text.contains("// I want the gitmoji codes"), "{text}");
        assert!(text.contains("\"real_emoji\": false"), "{text}");
        assert!(text.contains("\"scope_step\": false"), "{text}");
        assert!(text.contains("\"gitmoji_step\": true"), "{text}");
        assert!(text.contains("\"accent_color\": null"), "{text}");
        assert_eq!(from_file(load(&path).expect("the load works")), config);

        std::fs::remove_dir_all(&directory).ok();
    }

    /// A person can erase keys by hand, so the program never reads the file as complete.
    #[test]
    fn a_file_a_person_cut_down_still_loads_with_the_defaults() {
        let directory =
            std::env::temp_dir().join(format!("ccommit-config-cut-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("the directory is created");
        let path = directory.join("config.json");
        std::fs::write(&path, "{\n  \"breaking_change_step\": false\n}\n")
            .expect("the write works");

        let expected = Config {
            breaking_change_step: false,
            ..Config::default()
        };
        assert_eq!(from_file(load(&path).expect("the load works")), expected);

        // An empty object says nothing at all, and so does a file of comments alone.
        for text in ["{}", "{\n  // nothing here\n}\n"] {
            std::fs::write(&path, text).expect("the write works");
            assert_eq!(
                from_file(load(&path).expect("the load works")),
                Config::default(),
                "{text}"
            );
        }

        std::fs::remove_dir_all(&directory).ok();
    }
}

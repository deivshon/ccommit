use std::path::PathBuf;

use clap::Parser;
use clap::builder::BoolishValueParser;

use crate::config::Partial;
use crate::message::MessageLimit;
use crate::theme::HexColor;

/// Each boolean flag takes one value. `None` means the user said nothing, so the config file
/// decides.
#[derive(Debug, Parser)]
#[command(
    name = "ccommit",
    version,
    about = "Conventional commits from the terminal"
)]
pub struct Args {
    /// Set the working directory
    #[arg(short = 'C', long, default_value = ".")]
    pub working_directory: PathBuf,

    /// Open the configuration menu and exit
    #[arg(short = 'm', long)]
    pub config_menu: bool,

    /// Path to the configuration file
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Path to a JSON file with the commit types
    #[arg(long)]
    pub commit_types: Option<PathBuf>,

    /// Path to a JSON file with the gitmojis
    #[arg(long)]
    pub gitmojis: Option<PathBuf>,

    /// Commit with this first line and skip the wizard
    #[arg(short = 's', long, value_name = "TEXT", conflicts_with = "config_menu")]
    pub short_message: Option<String>,

    /// Put this text in the body, under one empty line
    #[arg(short = 'l', long, value_name = "TEXT", requires = "short_message")]
    pub long_message: Option<String>,

    /// Validate the short message and exit without a commit
    #[arg(long, requires = "short_message")]
    pub check_message: bool,

    /// Replace the last commit instead of adding one
    #[arg(long, conflicts_with_all = ["config_menu", "check_message"])]
    pub amend: bool,

    /// Commit when the wizard ends
    #[arg(long, value_name = "on|off", value_parser = BoolishValueParser::new(), hide_possible_values = true, overrides_with = "commit_on_finish")]
    commit_on_finish: Option<bool>,

    /// Show the message before the wizard ends
    #[arg(long, value_name = "on|off", value_parser = BoolishValueParser::new(), hide_possible_values = true, overrides_with = "preview_on_finish")]
    preview_on_finish: Option<bool>,

    /// Put the emoji in the message instead of the gitmoji code
    #[arg(long, value_name = "on|off", value_parser = BoolishValueParser::new(), hide_possible_values = true, overrides_with = "real_emoji")]
    real_emoji: Option<bool>,

    /// Show the scope step
    #[arg(long, value_name = "on|off", value_parser = BoolishValueParser::new(), hide_possible_values = true, overrides_with = "scope_step")]
    scope_step: Option<bool>,

    /// Show the gitmoji step
    #[arg(long, value_name = "on|off", value_parser = BoolishValueParser::new(), hide_possible_values = true, overrides_with = "gitmoji_step")]
    gitmoji_step: Option<bool>,

    /// Show the long description step
    #[arg(long, value_name = "on|off", value_parser = BoolishValueParser::new(), hide_possible_values = true, overrides_with = "long_description_step")]
    long_description_step: Option<bool>,

    /// Show the breaking change step
    #[arg(long, value_name = "on|off", value_parser = BoolishValueParser::new(), hide_possible_values = true, overrides_with = "breaking_change_step")]
    breaking_change_step: Option<bool>,

    /// Refuse the long message flag
    #[arg(long, value_name = "on|off", value_parser = BoolishValueParser::new(), hide_possible_values = true, overrides_with = "deny_cli_long_message")]
    deny_cli_long_message: Option<bool>,

    /// Characters allowed on the first line of the message
    #[arg(long, value_name = "50 or more|none")]
    short_message_limit: Option<MessageLimit>,

    /// Color of the titles, the prompts, and the cursor
    #[arg(long, value_name = "#rrggbb")]
    accent_color: Option<HexColor>,

    /// Color of the key help and the file path
    #[arg(long, value_name = "#rrggbb")]
    muted_color: Option<HexColor>,

    /// Color of the menu filter and the short counter
    #[arg(long, value_name = "#rrggbb")]
    success_color: Option<HexColor>,

    /// Color of the counter above half the limit
    #[arg(long, value_name = "#rrggbb")]
    warning_color: Option<HexColor>,

    /// Color of the failure line and the counter near the limit
    #[arg(long, value_name = "#rrggbb")]
    error_color: Option<HexColor>,
}

impl Args {
    /// The flags as one partial configuration, for the merge with the file.
    pub fn overrides(&self) -> Partial {
        Partial {
            commit_types: self.commit_types.clone(),
            gitmojis: self.gitmojis.clone(),
            commit_on_finish: self.commit_on_finish,
            preview_on_finish: self.preview_on_finish,
            real_emoji: self.real_emoji,
            scope_step: self.scope_step,
            gitmoji_step: self.gitmoji_step,
            long_description_step: self.long_description_step,
            breaking_change_step: self.breaking_change_step,
            deny_cli_long_message: self.deny_cli_long_message,
            accent_color: self.accent_color,
            muted_color: self.muted_color,
            success_color: self.success_color,
            warning_color: self.warning_color,
            error_color: self.error_color,
            short_message_limit: self.short_message_limit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(text: &str) -> HexColor {
        text.parse().expect("the color parses")
    }

    fn parse(arguments: &[&str]) -> Args {
        let mut full = vec!["ccommit"];
        full.extend_from_slice(arguments);
        Args::try_parse_from(full).expect("the arguments parse")
    }

    #[test]
    fn a_silent_boolean_stays_none() {
        let overrides = parse(&[]).overrides();
        assert_eq!(overrides.commit_on_finish, None);
        assert_eq!(overrides.preview_on_finish, None);
        assert_eq!(overrides.real_emoji, None);
        assert_eq!(overrides.scope_step, None);
        assert_eq!(overrides.gitmoji_step, None);
        assert_eq!(overrides.long_description_step, None);
        assert_eq!(overrides.breaking_change_step, None);
        assert_eq!(overrides.deny_cli_long_message, None);
    }

    #[test]
    fn every_step_flag_reaches_the_partial() {
        let overrides = parse(&[
            "--scope-step=off",
            "--gitmoji-step",
            "off",
            "--long-description-step=off",
            "--breaking-change-step=false",
        ])
        .overrides();

        assert_eq!(overrides.scope_step, Some(false));
        assert_eq!(overrides.gitmoji_step, Some(false));
        assert_eq!(overrides.long_description_step, Some(false));
        assert_eq!(overrides.breaking_change_step, Some(false));
    }

    #[test]
    fn off_turns_a_boolean_off() {
        assert_eq!(
            parse(&["--commit-on-finish", "off"])
                .overrides()
                .commit_on_finish,
            Some(false)
        );
        assert_eq!(
            parse(&["--preview-on-finish=off"])
                .overrides()
                .preview_on_finish,
            Some(false)
        );
        assert_eq!(
            parse(&["--real-emoji=off"]).overrides().real_emoji,
            Some(false)
        );
        assert_eq!(
            parse(&["--deny-cli-long-message=off"])
                .overrides()
                .deny_cli_long_message,
            Some(false)
        );
    }

    #[test]
    fn on_turns_a_boolean_on() {
        assert_eq!(
            parse(&["--commit-on-finish", "on"])
                .overrides()
                .commit_on_finish,
            Some(true)
        );
        assert_eq!(
            parse(&["--real-emoji=on"]).overrides().real_emoji,
            Some(true)
        );
        assert_eq!(
            parse(&["--deny-cli-long-message=on"])
                .overrides()
                .deny_cli_long_message,
            Some(true)
        );
    }

    /// The config file writes `true` and `false`, so the flags take those words too.
    #[test]
    fn true_and_false_work_like_on_and_off() {
        assert_eq!(
            parse(&["--commit-on-finish=false"])
                .overrides()
                .commit_on_finish,
            Some(false)
        );
        assert_eq!(
            parse(&["--commit-on-finish=true"])
                .overrides()
                .commit_on_finish,
            Some(true)
        );
    }

    #[test]
    fn the_last_value_on_the_command_line_wins() {
        let args = parse(&["--commit-on-finish=on", "--commit-on-finish=off"]);
        assert_eq!(args.overrides().commit_on_finish, Some(false));
    }

    #[test]
    fn a_boolean_flag_needs_a_value() {
        assert!(Args::try_parse_from(["ccommit", "--commit-on-finish"]).is_err());
        assert!(Args::try_parse_from(["ccommit", "--no-commit-on-finish"]).is_err());
        assert!(Args::try_parse_from(["ccommit", "--real-emoji", "maybe"]).is_err());
    }

    #[test]
    fn the_limit_flag_takes_a_number_and_the_word_for_no_limit() {
        assert_eq!(parse(&[]).overrides().short_message_limit, None);
        assert_eq!(
            parse(&["--short-message-limit=none"])
                .overrides()
                .short_message_limit,
            Some(MessageLimit::NONE)
        );
        assert_eq!(
            parse(&["--short-message-limit", "80"])
                .overrides()
                .short_message_limit
                .and_then(MessageLimit::total),
            Some(80)
        );
        assert_eq!(
            parse(&["--short-message-limit=50"])
                .overrides()
                .short_message_limit
                .and_then(MessageLimit::total),
            Some(50)
        );
    }

    #[test]
    fn the_limit_flag_refuses_a_number_under_the_minimum() {
        let error = Args::try_parse_from(["ccommit", "--short-message-limit", "49"])
            .expect_err("the arguments fail");
        assert!(error.to_string().contains("minimum of 50"), "{error}");

        assert!(Args::try_parse_from(["ccommit", "--short-message-limit", "wide"]).is_err());
        assert!(Args::try_parse_from(["ccommit", "--short-message-limit"]).is_err());
    }

    #[test]
    fn a_silent_color_stays_none() {
        let overrides = parse(&[]).overrides();
        assert_eq!(overrides.accent_color, None);
        assert_eq!(overrides.error_color, None);
    }

    #[test]
    fn every_color_flag_reaches_the_partial() {
        let overrides = parse(&[
            "--accent-color=#111111",
            "--muted-color",
            "#222222",
            "--success-color=#333333",
            "--warning-color=#444444",
            "--error-color=#555555",
        ])
        .overrides();

        assert_eq!(overrides.accent_color, Some(hex("#111111")));
        assert_eq!(overrides.muted_color, Some(hex("#222222")));
        assert_eq!(overrides.success_color, Some(hex("#333333")));
        assert_eq!(overrides.warning_color, Some(hex("#444444")));
        assert_eq!(overrides.error_color, Some(hex("#555555")));
    }

    #[test]
    fn a_color_flag_takes_the_short_form_and_drops_the_hash() {
        assert_eq!(
            parse(&["--accent-color=#f0a"]).overrides().accent_color,
            Some(hex("#ff00aa"))
        );
        assert_eq!(
            parse(&["--accent-color=1E88E5"]).overrides().accent_color,
            Some(hex("#1e88e5"))
        );
    }

    #[test]
    fn a_color_flag_refuses_a_value_that_is_not_hex() {
        let error = Args::try_parse_from(["ccommit", "--accent-color", "blue"])
            .expect_err("the arguments fail");
        assert!(error.to_string().contains("not a hex color"), "{error}");

        assert!(Args::try_parse_from(["ccommit", "--accent-color"]).is_err());
    }

    #[test]
    fn the_direct_flags_stay_silent_by_default() {
        let args = parse(&[]);
        assert_eq!(args.short_message, None);
        assert_eq!(args.long_message, None);
        assert!(!args.check_message);
        assert!(!args.amend);
    }

    #[test]
    fn the_amend_flag_needs_no_value_and_no_other_flag() {
        assert!(parse(&["--amend"]).amend);
        assert!(parse(&["--amend", "-s", "feat: add thing"]).amend);
    }

    #[test]
    fn the_amend_flag_refuses_the_menu_and_the_check() {
        assert!(Args::try_parse_from(["ccommit", "-m", "--amend"]).is_err());
        assert!(
            Args::try_parse_from([
                "ccommit",
                "-s",
                "feat: add thing",
                "--check-message",
                "--amend"
            ])
            .is_err()
        );
    }

    #[test]
    fn the_direct_flags_parse_in_both_forms() {
        let args = parse(&[
            "--short-message",
            "feat: add thing",
            "--long-message",
            "the body",
            "--check-message",
        ]);
        assert_eq!(args.short_message.as_deref(), Some("feat: add thing"));
        assert_eq!(args.long_message.as_deref(), Some("the body"));
        assert!(args.check_message);

        let args = parse(&["-s", "feat: add thing", "-l", "first\nsecond"]);
        assert_eq!(args.short_message.as_deref(), Some("feat: add thing"));
        assert_eq!(args.long_message.as_deref(), Some("first\nsecond"));
        assert!(!args.check_message);
    }

    #[test]
    fn the_long_message_and_the_check_need_the_short_message() {
        assert!(Args::try_parse_from(["ccommit", "--long-message", "the body"]).is_err());
        assert!(Args::try_parse_from(["ccommit", "--check-message"]).is_err());
    }

    #[test]
    fn the_short_message_refuses_the_config_menu() {
        assert!(Args::try_parse_from(["ccommit", "-m", "-s", "feat: add thing"]).is_err());
    }

    #[test]
    fn the_working_directory_defaults_to_the_current_one() {
        assert_eq!(parse(&[]).working_directory, PathBuf::from("."));
        assert_eq!(
            parse(&["-C", "/tmp"]).working_directory,
            PathBuf::from("/tmp")
        );
    }
}

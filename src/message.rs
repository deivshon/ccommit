use std::fmt;
use std::str::FromStr;

use serde::de::{Deserializer, Visitor};
use serde::{Deserialize, de};

/// The first line of a commit stays at 72 characters, so the limit starts here.
pub const DEFAULT_SHORT_MESSAGE_LIMIT: usize = 72;

/// A shorter first line than this leaves no room for a description.
pub const MINIMUM_SHORT_MESSAGE_LIMIT: usize = 50;

/// The word the user writes for a first line with no limit.
pub const NO_LIMIT: &str = "none";

/// The limit of the whole first line, in characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageLimit(Option<usize>);

impl MessageLimit {
    /// A first line the program never cuts.
    pub const NONE: Self = Self(None);

    /// A first line of `characters` at most. A number under the minimum is a failure.
    pub fn characters(characters: usize) -> Result<Self, String> {
        if characters < MINIMUM_SHORT_MESSAGE_LIMIT {
            return Err(format!(
                "`{characters}` is under the minimum of {MINIMUM_SHORT_MESSAGE_LIMIT} characters"
            ));
        }

        Ok(Self(Some(characters)))
    }

    /// The length of the whole first line. `None` means no limit.
    pub fn total(self) -> Option<usize> {
        self.0
    }
}

impl Default for MessageLimit {
    fn default() -> Self {
        Self(Some(DEFAULT_SHORT_MESSAGE_LIMIT))
    }
}

impl FromStr for MessageLimit {
    type Err = String;

    /// Reads `none` and a whole number of at least the minimum.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let trimmed = text.trim();

        if trimmed.eq_ignore_ascii_case(NO_LIMIT) {
            return Ok(Self::NONE);
        }

        match trimmed.parse::<usize>() {
            Ok(characters) => Self::characters(characters),
            Err(_) => Err(refusal(trimmed)),
        }
    }
}

impl fmt::Display for MessageLimit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(characters) => write!(formatter, "{characters}"),
            None => write!(formatter, "{NO_LIMIT}"),
        }
    }
}

/// The file writes a number, or the word for no limit.
impl<'de> Deserialize<'de> for MessageLimit {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(LimitVisitor)
    }
}

struct LimitVisitor;

impl Visitor<'_> for LimitVisitor {
    type Value = MessageLimit;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "a whole number of at least {MINIMUM_SHORT_MESSAGE_LIMIT}, or `{NO_LIMIT}`"
        )
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<MessageLimit, E> {
        MessageLimit::characters(value as usize).map_err(E::custom)
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<MessageLimit, E> {
        match usize::try_from(value) {
            Ok(characters) => MessageLimit::characters(characters).map_err(E::custom),
            Err(_) => Err(E::custom(refusal(&value.to_string()))),
        }
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<MessageLimit, E> {
        Err(E::custom(refusal(&value.to_string())))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<MessageLimit, E> {
        value.parse().map_err(E::custom)
    }
}

fn refusal(text: &str) -> String {
    format!(
        "`{text}` is not a whole number of at least {MINIMUM_SHORT_MESSAGE_LIMIT}, and it is not `{NO_LIMIT}`"
    )
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConventionalCommit {
    pub short_message: String,
    pub long_message: String,
}

impl ConventionalCommit {
    /// Gives the message as `git commit -m short -m long` writes it.
    pub fn text(&self) -> String {
        if self.long_message.is_empty() {
            self.short_message.clone()
        } else {
            format!("{}\n\n{}", self.short_message, self.long_message)
        }
    }
}

pub fn build_short_message(
    commit_type: &str,
    scope: &str,
    gitmoji: &str,
    short_description: &str,
) -> String {
    let mut message = String::from(commit_type);

    if !scope.is_empty() {
        message.push('(');
        message.push_str(scope);
        message.push(')');
    }
    message.push(':');

    if !gitmoji.is_empty() {
        message.push(' ');
        message.push_str(gitmoji);
    }

    if !short_description.is_empty() {
        message.push(' ');
        message.push_str(short_description);
    }

    message
}

pub fn build_long_message(long_description: &str, breaking_changes: &str) -> String {
    if long_description.is_empty() || breaking_changes.is_empty() {
        format!("{long_description}{breaking_changes}")
    } else {
        format!("{long_description}\n{breaking_changes}")
    }
}

/// The room the short description has on the first line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DescriptionLimit {
    /// Characters the description can take.
    pub characters: usize,
    /// Characters the type, the scope, the gitmoji, and the space after them already hold.
    pub occupied: usize,
}

/// Counts characters, not bytes, so one emoji costs one character.
///
/// `None` means the description has no limit.
pub fn short_description_limit(
    commit_type: &str,
    scope: &str,
    gitmoji: &str,
    limit: MessageLimit,
) -> Option<DescriptionLimit> {
    let prefix = build_short_message(commit_type, scope, gitmoji, "");
    let occupied = prefix.chars().count() + 1;

    limit.total().map(|total| DescriptionLimit {
        characters: total.saturating_sub(occupied),
        occupied,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limit_of(
        commit_type: &str,
        scope: &str,
        gitmoji: &str,
        limit: MessageLimit,
    ) -> Option<usize> {
        short_description_limit(commit_type, scope, gitmoji, limit).map(|limit| limit.characters)
    }

    fn occupied_by(commit_type: &str, scope: &str, gitmoji: &str) -> Option<usize> {
        short_description_limit(commit_type, scope, gitmoji, MessageLimit::default())
            .map(|limit| limit.occupied)
    }

    #[test]
    fn the_occupied_count_holds_the_prefix_and_the_space_after_it() {
        assert_eq!(occupied_by("feat", "", ""), Some(6), "`feat: `");
        assert_eq!(occupied_by("feat", "ui", ""), Some(10), "`feat(ui): `");
        assert_eq!(occupied_by("feat", "", "✨"), Some(8), "`feat: ✨ `");

        // The two counts always add up to the whole first line.
        for (commit_type, scope, gitmoji) in [("feat", "", ""), ("feat", "ui", "✨")] {
            let limit =
                short_description_limit(commit_type, scope, gitmoji, MessageLimit::default())
                    .expect("a limit");
            assert_eq!(limit.characters + limit.occupied, 72);
        }
    }

    #[test]
    fn short_message_holds_every_part() {
        let cases = [
            (("feat", "", "", ""), "feat:"),
            (("feat", "", "", "add thing"), "feat: add thing"),
            (("feat", "ui", "", "add thing"), "feat(ui): add thing"),
            (
                ("feat", "", ":sparkles:", "add thing"),
                "feat: :sparkles: add thing",
            ),
            (
                ("feat", "ui", ":sparkles:", "add thing"),
                "feat(ui): :sparkles: add thing",
            ),
            (("feat", "ui", "✨", "add thing"), "feat(ui): ✨ add thing"),
            (("feat", "ui", ":sparkles:", ""), "feat(ui): :sparkles:"),
        ];

        for ((commit_type, scope, gitmoji, description), expected) in cases {
            assert_eq!(
                build_short_message(commit_type, scope, gitmoji, description),
                expected
            );
        }
    }

    #[test]
    fn long_message_joins_the_two_parts_with_one_newline() {
        assert_eq!(build_long_message("", ""), "");
        assert_eq!(build_long_message("body", ""), "body");
        assert_eq!(build_long_message("", "BREAKING"), "BREAKING");
        assert_eq!(build_long_message("body", "BREAKING"), "body\nBREAKING");
        assert_eq!(build_long_message("a\nb", "BREAKING"), "a\nb\nBREAKING");
    }

    #[test]
    fn limit_leaves_the_first_line_at_72_characters() {
        let cases = [
            ("feat", "", "", 66),
            ("feat", "ui", "", 62),
            ("feat", "", ":sparkles:", 55),
            ("feat", "ui", ":sparkles:", 51),
            ("refactor", "", "", 62),
            ("performance", "long-scope-name", ":white_check_mark:", 23),
        ];

        for (commit_type, scope, gitmoji, expected) in cases {
            let limit = limit_of(commit_type, scope, gitmoji, MessageLimit::default());
            assert_eq!(limit, Some(expected), "{commit_type}({scope}) {gitmoji}");

            let description = "x".repeat(expected);
            let message = build_short_message(commit_type, scope, gitmoji, &description);
            assert_eq!(message.chars().count(), 72);
        }
    }

    #[test]
    fn a_real_emoji_costs_one_character_of_the_limit() {
        assert_eq!(
            limit_of("feat", "", "✨", MessageLimit::default()),
            Some(64)
        );

        let description = "x".repeat(64);
        let message = build_short_message("feat", "", "✨", &description);
        assert_eq!(message.chars().count(), 72);
        assert!(message.len() > 72, "the emoji takes more than one byte");
    }

    #[test]
    fn an_oversized_prefix_leaves_no_room() {
        let scope = "s".repeat(200);
        assert_eq!(
            limit_of("feat", &scope, "", MessageLimit::default()),
            Some(0)
        );
    }

    #[test]
    fn a_shorter_first_line_shortens_the_description() {
        let limit = MessageLimit::characters(50).expect("50 is the minimum");
        assert_eq!(limit_of("feat", "", "", limit), Some(44));

        let description = "x".repeat(44);
        let message = build_short_message("feat", "", "", &description);
        assert_eq!(message.chars().count(), 50);
    }

    #[test]
    fn a_longer_first_line_lengthens_the_description() {
        let limit = MessageLimit::characters(100).expect("100 is over the minimum");
        assert_eq!(limit_of("feat", "", "", limit), Some(94));
    }

    #[test]
    fn no_limit_leaves_the_description_free() {
        assert_eq!(limit_of("feat", "", "", MessageLimit::NONE), None);
        assert_eq!(limit_of("feat", "ui", "✨", MessageLimit::NONE), None);
    }

    #[test]
    fn the_default_limit_holds_the_first_line_at_72_characters() {
        assert_eq!(MessageLimit::default().total(), Some(72));
        assert_eq!(MessageLimit::NONE.total(), None);
    }

    #[test]
    fn a_number_under_the_minimum_is_refused() {
        for characters in [0, 1, 49] {
            let error = MessageLimit::characters(characters).expect_err("it fails");
            assert!(error.contains("minimum of 50"), "{error}");
        }

        assert_eq!(
            MessageLimit::characters(50).expect("it works").total(),
            Some(50)
        );
    }

    #[test]
    fn the_text_of_a_limit_reads_back_as_the_same_limit() {
        assert_eq!("none".parse(), Ok(MessageLimit::NONE));
        assert_eq!("NONE".parse(), Ok(MessageLimit::NONE));
        assert_eq!("  none  ".parse(), Ok(MessageLimit::NONE));
        assert_eq!("72".parse(), Ok(MessageLimit::default()));
        assert_eq!(" 80 ".parse(), MessageLimit::characters(80));

        assert_eq!(MessageLimit::NONE.to_string(), "none");
        assert_eq!(MessageLimit::default().to_string(), "72");
    }

    #[test]
    fn a_text_that_is_not_a_limit_fails_with_the_text() {
        for text in ["", "wide", "-1", "72.5", "seventy"] {
            let error = text.parse::<MessageLimit>().expect_err("the parse fails");
            assert!(error.contains("50"), "{error}");
        }

        let error = "49".parse::<MessageLimit>().expect_err("the parse fails");
        assert!(error.contains("minimum of 50"), "{error}");
    }

    #[test]
    fn text_separates_the_subject_from_the_body_with_a_blank_line() {
        let commit = ConventionalCommit {
            short_message: "feat: add thing".to_string(),
            long_message: String::new(),
        };
        assert_eq!(commit.text(), "feat: add thing");

        let commit = ConventionalCommit {
            short_message: "feat: add thing".to_string(),
            long_message: "body\nBREAKING".to_string(),
        };
        assert_eq!(commit.text(), "feat: add thing\n\nbody\nBREAKING");
    }
}

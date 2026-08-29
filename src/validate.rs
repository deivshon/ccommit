//! Validates a first line that comes from the command line instead of the wizard.
//!
//! A line passes only when the wizard can produce it with the same lists and the same limit.

use crate::data::Entry;
use crate::message::MessageLimit;

/// The parts of a valid first line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parts<'a> {
    pub commit_type: &'a str,
    pub scope: &'a str,
    pub gitmoji: &'a str,
    pub short_description: &'a str,
}

/// Makes sure that the wizard can produce `short_message`, and gives its parts back.
pub fn validate<'a>(
    short_message: &'a str,
    commit_types: &[Entry],
    gitmojis: &[Entry],
    limit: MessageLimit,
) -> Result<Parts<'a>, String> {
    if short_message.contains('\n') {
        return Err(
            "The short message spans more than one line. Put the extra lines in the long message"
                .to_string(),
        );
    }

    let (commit_type, rest) = split_type(short_message, commit_types)?;
    let (scope, after_colon) = split_scope(rest)?;
    let (gitmoji, short_description) = split_description(after_colon, gitmojis)?;

    if short_description.trim().is_empty() {
        return Err("The short description is empty. Write one after the `:`".to_string());
    }

    if let Some(total) = limit.total() {
        let count = short_message.chars().count();
        if count > total {
            return Err(format!(
                "The first line has {count} characters, over the limit of {total}"
            ));
        }
    }

    Ok(Parts {
        commit_type,
        scope,
        gitmoji,
        short_description,
    })
}

/// The commit type ends where the scope or the `:` begins.
fn split_type<'a>(message: &'a str, commit_types: &[Entry]) -> Result<(&'a str, &'a str), String> {
    let end = message
        .find(['(', ':'])
        .ok_or("The first line has no `:` after the commit type")?;
    let commit_type = &message[..end];

    if commit_type.is_empty() {
        return Err("The first line has no commit type".to_string());
    }
    if !commit_types.iter().any(|entry| entry.name == commit_type) {
        return Err(format!(
            "`{commit_type}` is not one of the commit types: {}",
            names(commit_types)
        ));
    }

    Ok((commit_type, &message[end..]))
}

/// The scope sits between `(` and `)`, and a `:` follows it.
fn split_scope(rest: &str) -> Result<(&str, &str), String> {
    let (scope, rest) = match rest.strip_prefix('(') {
        Some(inner) => {
            let close = inner.find(')').ok_or("The scope has no closing `)`")?;
            if close == 0 {
                return Err("The scope between `(` and `)` is empty".to_string());
            }
            (&inner[..close], &inner[close + 1..])
        }
        None => ("", rest),
    };

    let after_colon = rest.strip_prefix(':').ok_or("No `:` follows the scope")?;
    Ok((scope, after_colon))
}

/// After the `:` come one space, an optional gitmoji, and the short description.
fn split_description<'a>(
    after_colon: &'a str,
    gitmojis: &[Entry],
) -> Result<(&'a str, &'a str), String> {
    if after_colon.is_empty() {
        return Ok(("", ""));
    }

    let text = after_colon
        .strip_prefix(' ')
        .ok_or("No space follows the `:`")?;

    let token = text.split(' ').next().unwrap_or("");
    let is_gitmoji = gitmojis
        .iter()
        .any(|entry| entry.name == token || entry.emoji.as_deref() == Some(token));

    if is_gitmoji {
        let rest = &text[token.len()..];
        Ok((token, rest.strip_prefix(' ').unwrap_or(rest)))
    } else if looks_like_code(token) {
        Err(format!("`{token}` is not one of the gitmojis"))
    } else {
        Ok(("", text))
    }
}

/// A word shaped like `:sparkles:` is a gitmoji code, so a typo in one is an error.
fn looks_like_code(token: &str) -> bool {
    token.len() > 2 && token.starts_with(':') && token.ends_with(':')
}

fn names(entries: &[Entry]) -> String {
    entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, emoji: Option<&str>) -> Entry {
        Entry {
            name: name.to_string(),
            long: String::new(),
            description: String::new(),
            emoji: emoji.map(str::to_string),
        }
    }

    fn types() -> Vec<Entry> {
        ["feat", "fix", "chore"]
            .into_iter()
            .map(|name| entry(name, None))
            .collect()
    }

    fn gitmoji_entries() -> Vec<Entry> {
        vec![entry(":sparkles:", Some("✨")), entry(":bug:", Some("🐛"))]
    }

    fn check(message: &str) -> Result<Parts<'_>, String> {
        validate(
            message,
            &types(),
            &gitmoji_entries(),
            MessageLimit::default(),
        )
    }

    fn parts<'a>(
        commit_type: &'a str,
        scope: &'a str,
        gitmoji: &'a str,
        short_description: &'a str,
    ) -> Parts<'a> {
        Parts {
            commit_type,
            scope,
            gitmoji,
            short_description,
        }
    }

    #[test]
    fn every_form_the_wizard_produces_passes() {
        let cases = [
            ("feat: add thing", parts("feat", "", "", "add thing")),
            ("feat(ui): add thing", parts("feat", "ui", "", "add thing")),
            (
                "feat: :sparkles: add thing",
                parts("feat", "", ":sparkles:", "add thing"),
            ),
            ("feat: ✨ add thing", parts("feat", "", "✨", "add thing")),
            (
                "feat(ui): ✨ add thing",
                parts("feat", "ui", "✨", "add thing"),
            ),
            ("fix: 🐛 squash it", parts("fix", "", "🐛", "squash it")),
        ];

        for (message, expected) in cases {
            assert_eq!(check(message), Ok(expected), "{message}");
        }
    }

    #[test]
    fn an_unknown_type_fails_and_the_error_lists_the_types() {
        let error = check("wip: add thing").expect_err("the type is unknown");
        assert!(error.contains("`wip`"), "{error}");
        assert!(error.contains("feat, fix, chore"), "{error}");
    }

    #[test]
    fn a_first_line_with_no_colon_fails() {
        let error = check("feat add thing").expect_err("no `:` is present");
        assert!(error.contains("no `:`"), "{error}");

        let error = check("").expect_err("an empty line fails");
        assert!(error.contains("no `:`"), "{error}");
    }

    #[test]
    fn a_first_line_with_no_type_fails() {
        for message in [": add thing", "(ui): add thing"] {
            let error = check(message).expect_err("the type is absent");
            assert!(error.contains("no commit type"), "{error}");
        }
    }

    #[test]
    fn a_broken_scope_fails() {
        let error = check("feat(): add thing").expect_err("the scope is empty");
        assert!(error.contains("empty"), "{error}");

        let error = check("feat(ui: add thing").expect_err("the scope never closes");
        assert!(error.contains("closing"), "{error}");

        let error = check("feat(ui) add thing").expect_err("the `:` is absent");
        assert!(error.contains("No `:` follows the scope"), "{error}");
    }

    #[test]
    fn a_missing_space_after_the_colon_fails() {
        let error = check("feat:add thing").expect_err("the space is absent");
        assert!(error.contains("No space follows"), "{error}");
    }

    #[test]
    fn an_empty_short_description_fails() {
        for message in [
            "feat:",
            "feat: ",
            "feat:   ",
            "feat: :sparkles:",
            "feat(ui):",
        ] {
            let error = check(message).expect_err("the description is absent");
            assert!(
                error.contains("short description is empty"),
                "{message}: {error}"
            );
        }
    }

    #[test]
    fn an_unknown_gitmoji_code_fails() {
        let error = check("feat: :wombat: add thing").expect_err("the code is unknown");
        assert!(error.contains("`:wombat:`"), "{error}");
        assert!(error.contains("not one of the gitmojis"), "{error}");
    }

    #[test]
    fn a_word_that_only_starts_with_a_colon_is_description() {
        let result = check("feat: :) smile more").expect("the word is description");
        assert_eq!(result, parts("feat", "", "", ":) smile more"));
    }

    #[test]
    fn an_emoji_outside_the_list_is_description() {
        let result = check("feat: 🦄 add thing").expect("the emoji is description");
        assert_eq!(result, parts("feat", "", "", "🦄 add thing"));
    }

    #[test]
    fn the_limit_holds_the_first_line() {
        // `feat: ` takes 6 characters, so 66 more reach the limit of 72.
        let at_limit = format!("feat: {}", "x".repeat(66));
        assert!(check(&at_limit).is_ok());

        let over = format!("feat: {}", "x".repeat(67));
        let error = check(&over).expect_err("the line is over the limit");
        assert!(
            error.contains("73 characters, over the limit of 72"),
            "{error}"
        );

        let result = validate(&over, &types(), &gitmoji_entries(), MessageLimit::NONE);
        assert!(result.is_ok(), "no limit leaves the line free");
    }

    #[test]
    fn a_newline_in_the_short_message_fails() {
        let error = check("feat: add thing\nmore").expect_err("the line breaks");
        assert!(error.contains("more than one line"), "{error}");
    }
}

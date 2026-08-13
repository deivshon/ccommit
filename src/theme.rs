//! The five colors of the interface.
//!
//! A color the user does not set stays an ANSI color, so the palette of the terminal decides it.

use std::fmt;
use std::str::FromStr;

use ratatui::style::Color;
use serde::{Deserialize, Deserializer};

/// The bright half of the palette, because the dark half is hard to read on a dark background.
pub const DEFAULT_ACCENT: Color = Color::LightBlue;
pub const DEFAULT_MUTED: Color = Color::Gray;
pub const DEFAULT_SUCCESS: Color = Color::LightGreen;
pub const DEFAULT_WARNING: Color = Color::LightYellow;
pub const DEFAULT_ERROR: Color = Color::LightRed;

/// The color of each role, after the merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colors {
    /// The titles, the prompts, and the cursor mark of a menu.
    pub accent: Color,
    /// The key help and the path of the configuration file.
    pub muted: Color,
    /// The filter of a menu, and the counter well under the limit.
    pub success: Color,
    /// The counter above half the limit.
    pub warning: Color,
    /// The failure line, and the counter above three quarters of the limit.
    pub error: Color,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            accent: DEFAULT_ACCENT,
            muted: DEFAULT_MUTED,
            success: DEFAULT_SUCCESS,
            warning: DEFAULT_WARNING,
            error: DEFAULT_ERROR,
        }
    }
}

/// A color the user writes as a hex string. The file and the command line share this parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HexColor {
    red: u8,
    green: u8,
    blue: u8,
}

impl HexColor {
    pub fn color(self) -> Color {
        Color::Rgb(self.red, self.green, self.blue)
    }
}

/// Gives the color of the role, or the ANSI default when the user set nothing.
pub fn or_default(color: Option<HexColor>, default: Color) -> Color {
    color.map_or(default, HexColor::color)
}

/// The name of one ANSI default, because a hex value does not exist for it.
pub fn default_name(default: Color) -> &'static str {
    match default {
        DEFAULT_ACCENT => "bright blue",
        DEFAULT_MUTED => "gray",
        DEFAULT_SUCCESS => "bright green",
        DEFAULT_WARNING => "bright yellow",
        DEFAULT_ERROR => "bright red",
        _ => "the terminal color",
    }
}

impl FromStr for HexColor {
    type Err = String;

    /// Reads `#rgb` and `#rrggbb`. The `#` is optional and the case of the digits is free.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let trimmed = text.trim();
        let body = trimmed.strip_prefix('#').unwrap_or(trimmed);

        let digits: Vec<u8> = body
            .chars()
            .map(|digit| digit.to_digit(16).map(|value| value as u8))
            .collect::<Option<_>>()
            .ok_or_else(|| refusal(text))?;

        // A short form digit stands for a pair, so `f` means `ff`, that is 15 * 17.
        let (red, green, blue) = match digits[..] {
            [red, green, blue] => (red * 17, green * 17, blue * 17),
            [
                red_high,
                red_low,
                green_high,
                green_low,
                blue_high,
                blue_low,
            ] => (
                red_high * 16 + red_low,
                green_high * 16 + green_low,
                blue_high * 16 + blue_low,
            ),
            _ => return Err(refusal(text)),
        };

        Ok(Self { red, green, blue })
    }
}

impl fmt::Display for HexColor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { red, green, blue } = self;
        write!(formatter, "#{red:02x}{green:02x}{blue:02x}")
    }
}

impl<'de> Deserialize<'de> for HexColor {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

fn refusal(text: &str) -> String {
    format!("`{text}` is not a hex color such as `#1e88e5`")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Color {
        text.parse::<HexColor>()
            .unwrap_or_else(|error| panic!("{text} parses: {error}"))
            .color()
    }

    #[test]
    fn the_defaults_stay_ansi_colors() {
        let colors = Colors::default();
        assert_eq!(colors.accent, Color::LightBlue);
        assert_eq!(colors.muted, Color::Gray);
        assert_eq!(colors.success, Color::LightGreen);
        assert_eq!(colors.warning, Color::LightYellow);
        assert_eq!(colors.error, Color::LightRed);

        for color in [
            colors.accent,
            colors.muted,
            colors.success,
            colors.warning,
            colors.error,
        ] {
            assert!(
                !matches!(color, Color::Rgb(..)),
                "a default color leaves the palette to the terminal"
            );
        }
    }

    #[test]
    fn six_digits_give_one_channel_per_pair() {
        assert_eq!(parse("#1e88e5"), Color::Rgb(0x1e, 0x88, 0xe5));
        assert_eq!(parse("#000000"), Color::Rgb(0, 0, 0));
        assert_eq!(parse("#ffffff"), Color::Rgb(255, 255, 255));
    }

    #[test]
    fn three_digits_repeat_each_digit() {
        assert_eq!(parse("#f00"), Color::Rgb(255, 0, 0));
        assert_eq!(parse("#1a3"), Color::Rgb(0x11, 0xaa, 0x33));
    }

    #[test]
    fn the_hash_the_case_and_the_spaces_are_free() {
        let expected = Color::Rgb(0x1e, 0x88, 0xe5);
        assert_eq!(parse("1e88e5"), expected);
        assert_eq!(parse("#1E88E5"), expected);
        assert_eq!(parse("  #1e88E5  "), expected);
    }

    #[test]
    fn a_text_that_is_not_a_hex_color_fails_with_the_text() {
        for text in ["", "#", "blue", "#12", "#12345", "#1234567", "#12345g"] {
            let error = text
                .parse::<HexColor>()
                .expect_err("the parse fails on {text}");
            assert!(error.contains(text), "{error}");
            assert!(error.contains("#1e88e5"), "{error}");
        }
    }

    #[test]
    fn the_text_of_a_color_reads_back_as_the_same_color() {
        for text in ["#1e88e5", "#000000", "#ffffff", "#f00"] {
            let color = text.parse::<HexColor>().expect("the parse works");
            assert_eq!(color.to_string().parse::<HexColor>(), Ok(color));
        }

        assert_eq!(
            "#f00".parse::<HexColor>().expect("it parses").to_string(),
            "#ff0000"
        );
    }

    #[test]
    fn an_absent_color_gives_the_default() {
        assert_eq!(or_default(None, DEFAULT_ACCENT), DEFAULT_ACCENT);
        assert_eq!(
            or_default(Some("#1e88e5".parse().expect("it parses")), DEFAULT_ACCENT),
            Color::Rgb(0x1e, 0x88, 0xe5)
        );
    }
}

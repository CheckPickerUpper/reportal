//! A validated `#RRGGBB` hex color for terminal background theming.
//!
//! Validated at parse time so downstream code can emit OSC sequences
//! without rechecking format. Serializes/deserializes as a plain string.

use crate::error::ReportalError;

/// Stores a validated `#RRGGBB` hex color string.
#[derive(Debug, Clone)]
pub struct HexColor {
    value: String,
}

/// Parsing, validation, and OSC escape sequence generation for hex colors.
impl HexColor {
    /// Parses a hex color string, rejecting anything that isn't exactly `#RRGGBB`.
    ///
    /// Returns `InvalidColor` if the string is the wrong length, missing the
    /// leading `#`, or contains non-hex characters.
    pub fn parse(raw: &str) -> Result<Self, ReportalError> {
        let trimmed = raw.trim();
        if trimmed.len() != 7 || !trimmed.starts_with('#') {
            return Err(ReportalError::InvalidColor {
                value: raw.to_owned(),
            });
        }
        let hex_digits = &trimmed[1..];
        let all_hex = hex_digits.chars().all(|character| character.is_ascii_hexdigit());
        if !all_hex {
            return Err(ReportalError::InvalidColor {
                value: raw.to_owned(),
            });
        }
        Ok(Self {
            value: trimmed.to_owned(),
        })
    }

    /// The raw hex string as stored (e.g. `#1a1a2e`).
    pub fn raw_value(&self) -> &str {
        &self.value
    }

    /// Extracts the red, green, and blue bytes from the validated hex string.
    ///
    /// Safe to call on any `HexColor` because `parse()` already validated
    /// that exactly 6 hex digits follow the `#`.
    pub fn as_rgb_bytes(&self) -> Result<(u8, u8, u8), ReportalError> {
        let hex_digits = &self.value[1..];
        let parse_hex_pair = |slice: &str| -> Result<u8, ReportalError> {
            u8::from_str_radix(slice, 16)
                .map_err(|parse_error| ReportalError::InvalidColor {
                    value: format!("{}: {parse_error}", self.value),
                })
        };
        let red = parse_hex_pair(&hex_digits[0..2])?;
        let green = parse_hex_pair(&hex_digits[2..4])?;
        let blue = parse_hex_pair(&hex_digits[4..6])?;
        Ok((red, green, blue))
    }

    /// Returns the OSC 4;264 escape sequence that sets the Windows Terminal
    /// tab color strip to this color. Index 264 is `FRAME_BACKGROUND` in WT's
    /// color table. Silently ignored by terminals that don't support it.
    /// Format: `\x1b]4;264;rgb:RR/GG/BB\x07`
    pub fn as_osc_tab_color_sequence(&self) -> String {
        let hex_digits = &self.value[1..];
        let red_hex = &hex_digits[0..2];
        let green_hex = &hex_digits[2..4];
        let blue_hex = &hex_digits[4..6];
        format!(
            "\x1b]4;264;rgb:{red_hex}/{green_hex}/{blue_hex}\x07"
        )
    }
}

/// Serializes a `HexColor` as its raw `#RRGGBB` string for TOML output.
impl serde::Serialize for HexColor {
    fn serialize<S: serde::Serializer>(&self, toml_serializer: S) -> Result<S::Ok, S::Error> {
        toml_serializer.serialize_str(&self.value)
    }
}

/// Deserializes a `#RRGGBB` string into a validated `HexColor`.
impl<'de> serde::Deserialize<'de> for HexColor {
    fn deserialize<D: serde::Deserializer<'de>>(toml_deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(toml_deserializer)?;
        HexColor::parse(&raw).map_err(serde::de::Error::custom)
    }
}

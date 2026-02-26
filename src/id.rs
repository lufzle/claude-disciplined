use std::fmt;

use serde::{Deserialize, Serialize};

/// All entity prefixes in the system. Each has its own counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Prefix {
    Req,
    Nfr,
    M,
    E,
    S,
    T,
    D,
    Ai,
    F,
    Ur,
}

impl Prefix {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Req => "REQ",
            Self::Nfr => "NFR",
            Self::M => "M",
            Self::E => "E",
            Self::S => "S",
            Self::T => "T",
            Self::D => "D",
            Self::Ai => "AI",
            Self::F => "F",
            Self::Ur => "UR",
        }
    }
}

impl fmt::Display for Prefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A typed entity ID: prefix + 3-digit hex value (001–FFF).
///
/// # Invariants
/// - `value` is in 1..=0xFFF (1–4095)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Id {
    pub prefix: Prefix,
    value: u16,
}

/// Maximum ID value (0xFFF = 4095).
const MAX_ID: u16 = 0xFFF;

impl Id {
    /// Create an `Id` from a prefix and numeric value.
    ///
    /// # Errors
    /// Returns `Err` if `value` is 0 or exceeds 0xFFF.
    pub fn new(prefix: Prefix, value: u16) -> Result<Self, IdError> {
        if value == 0 || value > MAX_ID {
            return Err(IdError::OutOfRange { prefix, value });
        }
        Ok(Self { prefix, value })
    }

    /// Parse an ID from its string representation (e.g., "REQ-0A1", "T-001").
    ///
    /// # Errors
    /// Returns `Err` if the string doesn't match the expected format.
    pub fn parse(s: &str) -> Result<Self, IdError> {
        let (prefix_str, hex_str) = s
            .rsplit_once('-')
            .ok_or_else(|| IdError::InvalidFormat(s.to_owned()))?;

        let prefix = match prefix_str {
            "REQ" => Prefix::Req,
            "NFR" => Prefix::Nfr,
            "M" => Prefix::M,
            "E" => Prefix::E,
            "S" => Prefix::S,
            "T" => Prefix::T,
            "D" => Prefix::D,
            "AI" => Prefix::Ai,
            "F" => Prefix::F,
            "UR" => Prefix::Ur,
            _ => return Err(IdError::UnknownPrefix(prefix_str.to_owned())),
        };

        if hex_str.len() != 3 {
            return Err(IdError::InvalidFormat(s.to_owned()));
        }

        let value =
            u16::from_str_radix(hex_str, 16).map_err(|_| IdError::InvalidFormat(s.to_owned()))?;

        Self::new(prefix, value)
    }

    pub fn value(&self) -> u16 {
        self.value
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{:03X}", self.prefix, self.value)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum IdError {
    OutOfRange { prefix: Prefix, value: u16 },
    InvalidFormat(String),
    UnknownPrefix(String),
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfRange { prefix, value } => {
                write!(f, "{prefix} id {value} out of range (must be 1..=4095)")
            }
            Self::InvalidFormat(s) => write!(f, "invalid id format: {s}"),
            Self::UnknownPrefix(p) => write!(f, "unknown id prefix: {p}"),
        }
    }
}

impl std::error::Error for IdError {}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Id::new --

    #[test]
    fn new_with_valid_value() {
        let id = Id::new(Prefix::Req, 1).unwrap();
        assert_eq!(id.prefix, Prefix::Req);
        assert_eq!(id.value, 1);
    }

    #[test]
    fn new_with_max_value() {
        let id = Id::new(Prefix::T, 0xFFF).unwrap();
        assert_eq!(id.value, 0xFFF);
    }

    #[test]
    fn new_rejects_zero() {
        let err = Id::new(Prefix::E, 0).unwrap_err();
        assert_eq!(
            err,
            IdError::OutOfRange {
                prefix: Prefix::E,
                value: 0
            }
        );
    }

    #[test]
    fn new_rejects_above_max() {
        let err = Id::new(Prefix::S, 0x1000).unwrap_err();
        assert_eq!(
            err,
            IdError::OutOfRange {
                prefix: Prefix::S,
                value: 0x1000
            }
        );
    }

    // -- Display --

    #[test]
    fn display_pads_to_three_hex_digits() {
        let id = Id::new(Prefix::Req, 1).unwrap();
        assert_eq!(id.to_string(), "REQ-001");
    }

    #[test]
    fn display_hex_uppercase() {
        let id = Id::new(Prefix::D, 0xABC).unwrap();
        assert_eq!(id.to_string(), "D-ABC");
    }

    #[test]
    fn display_max() {
        let id = Id::new(Prefix::Ur, 0xFFF).unwrap();
        assert_eq!(id.to_string(), "UR-FFF");
    }

    // -- Parse --

    #[test]
    fn parse_valid_ids() {
        let cases = [
            ("REQ-001", Prefix::Req, 1),
            ("NFR-0A1", Prefix::Nfr, 0x0A1),
            ("M-010", Prefix::M, 0x010),
            ("E-FFF", Prefix::E, 0xFFF),
            ("S-00F", Prefix::S, 0x00F),
            ("T-100", Prefix::T, 0x100),
            ("D-ABC", Prefix::D, 0xABC),
            ("AI-001", Prefix::Ai, 1),
            ("F-0B2", Prefix::F, 0x0B2),
            ("UR-00A", Prefix::Ur, 0x00A),
        ];
        for (input, expected_prefix, expected_value) in cases {
            let id = Id::parse(input).unwrap();
            assert_eq!(id.prefix, expected_prefix, "prefix mismatch for {input}");
            assert_eq!(id.value, expected_value, "value mismatch for {input}");
        }
    }

    #[test]
    fn parse_roundtrips_through_display() {
        let id = Id::new(Prefix::Req, 0x0A1).unwrap();
        let parsed = Id::parse(&id.to_string()).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn parse_rejects_no_dash() {
        assert!(Id::parse("REQ001").is_err());
    }

    #[test]
    fn parse_rejects_unknown_prefix() {
        let err = Id::parse("FOO-001").unwrap_err();
        assert_eq!(err, IdError::UnknownPrefix("FOO".to_owned()));
    }

    #[test]
    fn parse_rejects_wrong_hex_length() {
        assert!(Id::parse("T-01").is_err());
        assert!(Id::parse("T-0001").is_err());
    }

    #[test]
    fn parse_rejects_non_hex() {
        assert!(Id::parse("T-XYZ").is_err());
    }

    #[test]
    fn parse_rejects_zero_value() {
        let err = Id::parse("T-000").unwrap_err();
        assert_eq!(
            err,
            IdError::OutOfRange {
                prefix: Prefix::T,
                value: 0
            }
        );
    }

    // -- Prefix::as_str --

    #[test]
    fn all_prefixes_roundtrip() {
        let all = [
            Prefix::Req,
            Prefix::Nfr,
            Prefix::M,
            Prefix::E,
            Prefix::S,
            Prefix::T,
            Prefix::D,
            Prefix::Ai,
            Prefix::F,
            Prefix::Ur,
        ];
        for p in all {
            let s = format!("{p}-001");
            let id = Id::parse(&s).unwrap();
            assert_eq!(id.prefix, p);
        }
    }
}

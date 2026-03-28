use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Supported locales for the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    En,
    Pl,
}

impl Locale {
    pub const ALL: &'static [Locale] = &[Locale::En, Locale::Pl];

    pub fn as_str(&self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::Pl => "pl",
        }
    }
}

impl FromStr for Locale {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "en" => Ok(Locale::En),
            "pl" => Ok(Locale::Pl),
            other => Err(format!("unsupported locale: {other}")),
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Default for Locale {
    fn default() -> Self {
        Locale::En
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_roundtrip() {
        for locale in Locale::ALL {
            let s = locale.as_str();
            let parsed: Locale = s.parse().unwrap();
            assert_eq!(*locale, parsed);
            assert_eq!(s, locale.to_string());
        }
    }

    #[test]
    fn locale_serde_roundtrip() {
        let json = serde_json::to_string(&Locale::Pl).unwrap();
        assert_eq!(json, "\"pl\"");
        let parsed: Locale = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, Locale::Pl);
    }

    #[test]
    fn locale_invalid() {
        assert!("xx".parse::<Locale>().is_err());
        assert!("".parse::<Locale>().is_err());
    }
}

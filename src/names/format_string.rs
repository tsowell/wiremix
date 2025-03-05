//! A type for validating name format strings.

use regex::{self, Regex};
use serde_with::DeserializeFromStr;

use crate::names::tag::Tag;

#[derive(Debug, DeserializeFromStr)]
pub struct FormatString(String);

impl FormatString {
    pub fn from_raw(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl std::str::FromStr for FormatString {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tags_regex = Regex::new(r"\{([a-z.-:]*)\}")?;

        for cap in tags_regex.captures_iter(s) {
            let tag = cap
                .get(1)
                .ok_or(Self::Err::msg("Failed to parse tag"))?
                .as_str();
            if tag.parse::<Tag>().is_err() {
                return Err(Self::Err::msg(format!(
                    "\"{}\" is not implemented",
                    tag
                )));
            }
        }

        Ok(FormatString(s.to_string()))
    }
}

#[allow(clippy::to_string_trait_impl)] // This is not for display.
impl ToString for FormatString {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

use std::collections::HashMap;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::config::property_key::{PropertyKey, PropertyResolver};
use crate::wirehose::state;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct MatchCondition(pub HashMap<PropertyKey, MatchValue>);

impl MatchCondition {
    pub fn matches(
        &self,
        state: &state::State,
        resolver: &(impl PropertyResolver + ?Sized),
    ) -> bool {
        self.0
            .iter()
            .all(|(key, value)| value.matches(resolver.resolve_key(state, key)))
    }

    /// Builds a matcher against a node's exact `node.name` - used to
    /// durably identify a node for hide-items persistence. `node.name` is
    /// the same property `Filter::defaults()` already relies on to
    /// identify wiremix's own capture streams by name, the established
    /// "stable enough" identifier for a node in this codebase. Not
    /// perfectly stable for every kind of stream (an app that varies its
    /// own `node.name` between launches won't re-match), the same
    /// limitation any hand-written `[[filters]]` entry already has.
    pub fn from_node_name(name: &str) -> Self {
        Self(HashMap::from([(
            PropertyKey::Bare(String::from("node.name")),
            MatchValue::Literal(String::from(name)),
        )]))
    }
}

#[derive(Debug, Clone, DeserializeFromStr, SerializeDisplay)]
pub enum MatchValue {
    Literal(String),
    NegatedLiteral(String),
    Regex(Regex),
    NegatedRegex(Regex),
    Null,
    NotNull,
}

impl std::fmt::Display for MatchValue {
    /// Inverse of `FromStr` - only round-trips values `FromStr` can
    /// actually parse back: the literal string `"null"` is escaped as
    /// `"null"` (quoted) so it isn't read back as the `Null` variant on
    /// reload. Literal strings that themselves start with `!` or `~` are
    /// not escaped, since `FromStr` has no quoting mechanism for that -
    /// same pre-existing limitation as any hand-written `[[filters]]`
    /// entry.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchValue::Null => write!(f, "null"),
            MatchValue::NotNull => write!(f, "!null"),
            MatchValue::Literal(s) if s == "null" => write!(f, "\"null\""),
            MatchValue::Literal(s) => write!(f, "{s}"),
            MatchValue::NegatedLiteral(s) if s == "null" => {
                write!(f, "!\"null\"")
            }
            MatchValue::NegatedLiteral(s) => write!(f, "!{s}"),
            MatchValue::Regex(re) => write!(f, "~{}", re.as_str()),
            MatchValue::NegatedRegex(re) => write!(f, "!~{}", re.as_str()),
        }
    }
}

#[cfg(test)]
impl PartialEq for MatchValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Literal(a), Self::Literal(b)) => a == b,
            (Self::NegatedLiteral(a), Self::NegatedLiteral(b)) => a == b,
            (Self::Regex(a), Self::Regex(b)) => a.as_str() == b.as_str(),
            (Self::NegatedRegex(a), Self::NegatedRegex(b)) => {
                a.as_str() == b.as_str()
            }
            (Self::Null, Self::Null) => true,
            (Self::NotNull, Self::NotNull) => true,
            _ => false,
        }
    }
}

impl std::str::FromStr for MatchValue {
    type Err = regex::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "null" => Ok(MatchValue::Null),
            "!null" => Ok(MatchValue::NotNull),
            "\"null\"" => Ok(MatchValue::Literal(String::from("null"))),
            "!\"null\"" => Ok(MatchValue::NegatedLiteral(String::from("null"))),
            s if s.starts_with("!~") => {
                Ok(MatchValue::NegatedRegex(Regex::new(&s[2..])?))
            }
            s if s.starts_with("~") => {
                Ok(MatchValue::Regex(Regex::new(&s[1..])?))
            }
            s if s.starts_with("!") => {
                Ok(MatchValue::NegatedLiteral(s[1..].to_string()))
            }
            s => Ok(MatchValue::Literal(s.to_string())),
        }
    }
}

impl MatchValue {
    fn matches(&self, value: Option<&str>) -> bool {
        match self {
            MatchValue::Null => value.is_none(),
            MatchValue::NotNull => value.is_some(),
            MatchValue::Literal(expected) => {
                value.is_some_and(|v| v == expected)
            }
            MatchValue::NegatedLiteral(expected) => {
                value.map_or(true, |v| v != expected)
            }
            MatchValue::Regex(re) => value.is_some_and(|v| re.is_match(v)),
            MatchValue::NegatedRegex(re) => {
                value.map_or(true, |v| !re.is_match(v))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_null() {
        assert!(matches!(
            "null".parse::<MatchValue>().unwrap(),
            MatchValue::Null
        ));
    }

    #[test]
    fn parse_not_null() {
        assert!(matches!(
            "!null".parse::<MatchValue>().unwrap(),
            MatchValue::NotNull
        ));
    }

    #[test]
    fn parse_quoted_null() {
        let val = "\"null\"".parse::<MatchValue>().unwrap();
        assert!(matches!(val, MatchValue::Literal(s) if s == "null"));
    }

    #[test]
    fn parse_negated_quoted_null() {
        let val = "!\"null\"".parse::<MatchValue>().unwrap();
        assert!(matches!(val, MatchValue::NegatedLiteral(s) if s == "null"));
    }

    #[test]
    fn parse_literal() {
        let val = "hello".parse::<MatchValue>().unwrap();
        assert!(matches!(val, MatchValue::Literal(s) if s == "hello"));
    }

    #[test]
    fn parse_negated_literal() {
        let val = "!hello".parse::<MatchValue>().unwrap();
        assert!(matches!(val, MatchValue::NegatedLiteral(s) if s == "hello"));
    }

    #[test]
    fn parse_regex() {
        let val = "~^foo.*bar$".parse::<MatchValue>().unwrap();
        assert!(matches!(val, MatchValue::Regex(_)));
    }

    #[test]
    fn parse_negated_regex() {
        let val = "!~^foo.*bar$".parse::<MatchValue>().unwrap();
        assert!(matches!(val, MatchValue::NegatedRegex(_)));
    }

    #[test]
    fn parse_invalid_regex() {
        assert!("~[invalid".parse::<MatchValue>().is_err());
    }

    #[test]
    fn null_matches_none() {
        let val = "null".parse::<MatchValue>().unwrap();
        assert!(val.matches(None));
        assert!(!val.matches(Some("anything")));
    }

    #[test]
    fn not_null_matches_some() {
        let val = "!null".parse::<MatchValue>().unwrap();
        assert!(val.matches(Some("anything")));
        assert!(!val.matches(None));
    }

    #[test]
    fn literal_matches_exact() {
        let val = "hello".parse::<MatchValue>().unwrap();
        assert!(val.matches(Some("hello")));
        assert!(!val.matches(Some("world")));
        assert!(!val.matches(None));
    }

    #[test]
    fn negated_literal_matches_different() {
        let val = "!hello".parse::<MatchValue>().unwrap();
        assert!(!val.matches(Some("hello")));
        assert!(val.matches(Some("world")));
        assert!(val.matches(None));
    }

    #[test]
    fn regex_matches_pattern() {
        let val = "~^foo\\d+$".parse::<MatchValue>().unwrap();
        assert!(val.matches(Some("foo123")));
        assert!(!val.matches(Some("bar123")));
        assert!(!val.matches(None));
    }

    #[test]
    fn negated_regex_matches_non_pattern() {
        let val = "!~^foo\\d+$".parse::<MatchValue>().unwrap();
        assert!(!val.matches(Some("foo123")));
        assert!(val.matches(Some("bar123")));
        assert!(val.matches(None));
    }

    #[test]
    fn quoted_null_matches_literal_null_string() {
        let val = "\"null\"".parse::<MatchValue>().unwrap();
        assert!(val.matches(Some("null")));
        assert!(!val.matches(None));
    }

    #[test]
    fn negated_quoted_null_rejects_literal_null_string() {
        let val = "!\"null\"".parse::<MatchValue>().unwrap();
        assert!(!val.matches(Some("null")));
        assert!(val.matches(Some("other")));
        assert!(val.matches(None));
    }

    fn round_trip(s: &str) {
        let val = s.parse::<MatchValue>().unwrap();
        assert_eq!(val.to_string(), s);
        let reparsed = val.to_string().parse::<MatchValue>().unwrap();
        assert_eq!(val, reparsed);
    }

    #[test]
    fn display_round_trips_null() {
        round_trip("null");
    }

    #[test]
    fn display_round_trips_not_null() {
        round_trip("!null");
    }

    #[test]
    fn display_round_trips_quoted_null_literal() {
        round_trip("\"null\"");
    }

    #[test]
    fn display_round_trips_negated_quoted_null_literal() {
        round_trip("!\"null\"");
    }

    #[test]
    fn display_round_trips_literal() {
        round_trip("hello");
    }

    #[test]
    fn display_round_trips_negated_literal() {
        round_trip("!hello");
    }

    #[test]
    fn display_round_trips_regex() {
        round_trip("~^foo.*bar$");
    }

    #[test]
    fn display_round_trips_negated_regex() {
        round_trip("!~^foo.*bar$");
    }
}

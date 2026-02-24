//! Parsing for override matching.
//! This provides backwards compatibility of the old format (property + value)
//! with the new format (PipeWire match rules).

use std::collections::HashMap;

use serde::Deserialize;

use crate::config::{
    matching, names, property_key, NameOverride, OverrideType,
};

impl<'de> Deserialize<'de> for NameOverride {
    fn deserialize<D: serde::Deserializer<'de>>(
        d: D,
    ) -> Result<Self, D::Error> {
        NameOverrideRaw::deserialize(d)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct NameOverrideRaw {
    types: Vec<OverrideType>,

    // Legacy fields
    property: Option<property_key::PropertyKey>,
    value: Option<String>,
    // New fields
    matches: Option<Vec<matching::MatchCondition>>,

    templates: Vec<names::NameTemplate>,
}

impl TryFrom<NameOverrideRaw> for NameOverride {
    type Error = String;

    fn try_from(raw: NameOverrideRaw) -> Result<Self, Self::Error> {
        let matches = match (raw.matches, raw.property, raw.value) {
            (Some(matches), None, None) => matches,
            (None, Some(property), Some(value)) => {
                vec![matching::MatchCondition(HashMap::from([(
                    property,
                    matching::MatchValue::Literal(value),
                )]))]
            }
            (None, None, None) => {
                return Err(
                    "must specify either `matches` or `property`/`value`"
                        .into(),
                );
            }
            (Some(_), Some(_), _) | (Some(_), _, Some(_)) => {
                return Err(
                    "cannot specify both `matches` and `property`/`value`"
                        .into(),
                );
            }
            (None, Some(_), None) => {
                return Err("`property` requires `value`".into());
            }
            (None, None, Some(_)) => {
                return Err("`value` requires `property`".into());
            }
        };

        Ok(NameOverride {
            types: raw.types,
            matches,
            templates: raw.templates,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_style() {
        let toml = r#"
            types = ["stream"]
            matches = [{ "node:node.name" = "spotify" }]
            templates = ["{node:node.name}"]
        "#;
        let ovr: NameOverride = toml::from_str(toml).unwrap();
        assert_eq!(ovr.matches.len(), 1);
    }

    #[test]
    fn legacy_style() {
        let toml = r#"
            types = ["stream"]
            property = "node:node.name"
            value = "spotify"
            templates = ["{node:node.name}"]
        "#;
        let ovr: NameOverride = toml::from_str(toml).unwrap();
        assert_eq!(ovr.matches.len(), 1);
    }

    #[test]
    fn both_is_error() {
        let toml = r#"
            types = ["stream"]
            property = "node:node.name"
            value = "spotify"
            matches = [{ "node:node.name" = "spotify" }]
            templates = ["{node:node.name}"]
        "#;
        assert!(toml::from_str::<NameOverride>(toml).is_err());
    }

    #[test]
    fn neither_is_error() {
        let toml = r#"
            types = ["stream"]
            templates = ["{node:node.name}"]
        "#;
        assert!(toml::from_str::<NameOverride>(toml).is_err());
    }

    #[test]
    fn property_without_value_is_error() {
        let toml = r#"
            types = ["stream"]
            property = "node:node.name"
            templates = ["{node:node.name}"]
        "#;
        assert!(toml::from_str::<NameOverride>(toml).is_err());
    }

    #[test]
    fn value_without_property_is_error() {
        let toml = r#"
            types = ["stream"]
            value = "spotify"
            templates = ["{node:node.name}"]
        "#;
        assert!(toml::from_str::<NameOverride>(toml).is_err());
    }

    #[test]
    fn legacy_equivalent_to_new() {
        let legacy = r#"
            types = ["stream"]
            property = "node:node.name"
            value = "spotify"
            templates = ["{node:node.name}"]
        "#;

        let new = r#"
            types = ["stream"]
            matches = [{ "node:node.name" = "spotify" }]
            templates = ["{node:node.name}"]
        "#;

        assert_eq!(
            toml::from_str::<NameOverride>(legacy).unwrap(),
            toml::from_str::<NameOverride>(new).unwrap()
        );
    }
}

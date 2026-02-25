use std::collections::HashMap;

use serde::Deserialize;

use crate::config::matching::{MatchCondition, MatchValue};
use crate::config::property_key::PropertyKey;
use crate::config::Filter;

impl Filter {
    pub fn defaults() -> Vec<Filter> {
        vec![
            Filter {
                // We shouldn't monitor our own capture streams.
                // No id prevents this from being overridden.
                id: None,
                matches: vec![MatchCondition(HashMap::from([(
                    PropertyKey::Bare(String::from("node.name")),
                    MatchValue::Literal(String::from("wiremix-capture")),
                )]))],
            },
            Filter {
                id: Some(String::from("pavucontrol-capture")),
                matches: vec![MatchCondition(HashMap::from([(
                    PropertyKey::Bare(String::from("node.name")),
                    MatchValue::Literal(String::from(
                        "PulseAudio Volume Control",
                    )),
                )]))],
            },
            Filter {
                id: Some(String::from("ncpamixer-capture")),
                matches: vec![MatchCondition(HashMap::from([(
                    PropertyKey::Bare(String::from("node.name")),
                    MatchValue::Literal(String::from("ncpamixer")),
                )]))],
            },
        ]
    }

    pub fn merge<'de, D>(deserializer: D) -> Result<Vec<Filter>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let configured = Vec::<Filter>::deserialize(deserializer)?;
        let mut merged = Self::defaults();
        for filter in configured {
            if let Some(id) = &filter.id {
                if let Some(pos) =
                    merged.iter().position(|f| f.id.as_ref() == Some(id))
                {
                    merged[pos] = filter;
                    continue;
                }
            }
            merged.push(filter);
        }
        Ok(merged)
    }
}

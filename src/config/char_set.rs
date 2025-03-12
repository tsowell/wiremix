//! Implementation for [`CharSet`](`crate::config::CharSet`). Defines default
//! character sets and handles merging of configured char sets with defaults.

use std::collections::HashMap;

use serde::{de::Error, Deserialize};

use crate::config::CharSet;

// This is what actually gets parsed from the config.
#[derive(Deserialize, Debug)]
pub struct CharSetOverlay {
    inherit: Option<String>,
    default_endpoint: Option<String>,
    default_target: Option<String>,
    object_selected_top: Option<String>,
    object_selected_center: Option<String>,
    object_selected_bottom: Option<String>,
    objects_more: Option<String>,
    volume_bar_background: Option<String>,
    volume_bar_foreground: Option<String>,
    meter_left: Option<String>,
    meter_left_overload: Option<String>,
    meter_left_unlit: Option<String>,
    meter_right: Option<String>,
    meter_right_overload: Option<String>,
    meter_right_unlit: Option<String>,
    meter_live_left: Option<String>,
    meter_live_left_unlit: Option<String>,
    meter_live_right: Option<String>,
    meter_live_right_unlit: Option<String>,
    dropdown: Option<String>,
    dropdown_item_selected: Option<String>,
    dropdown_more: Option<String>,
    tab_selected_left: Option<String>,
    tab_selected_right: Option<String>,
}

impl Default for CharSet {
    fn default() -> Self {
        Self {
            default_endpoint: String::from("◇"),
            default_target: String::from("◇"),
            object_selected_top: String::from("░"),
            object_selected_center: String::from("▒"),
            object_selected_bottom: String::from("░"),
            objects_more: String::from("•••"),
            volume_bar_background: String::from("╌"),
            volume_bar_foreground: String::from("━"),
            meter_left: String::from("▮"),
            meter_left_overload: String::from("▮"),
            meter_left_unlit: String::from("▮"),
            meter_right: String::from("▮"),
            meter_right_overload: String::from("▮"),
            meter_right_unlit: String::from("▮"),
            meter_live_left: String::from("■"),
            meter_live_left_unlit: String::from("■"),
            meter_live_right: String::from("■"),
            meter_live_right_unlit: String::from("■"),
            dropdown: String::from("▼"),
            dropdown_item_selected: String::from(">"),
            dropdown_more: String::from("•••"),
            tab_selected_left: String::from("["),
            tab_selected_right: String::from("]"),
        }
    }
}

impl TryFrom<CharSetOverlay> for CharSet {
    type Error = anyhow::Error;

    fn try_from(overlay: CharSetOverlay) -> Result<Self, Self::Error> {
        let mut char_set: Self = match overlay.inherit.as_deref() {
            Some("default") => CharSet::default(),
            Some(inherit) => {
                anyhow::bail!("'{}' is not a built-in character set", inherit)
            }
            None => CharSet::default(),
        };

        macro_rules! validate_and_set {
            // Overwrite default char with char from overlay while validating
            // width. Length of 0 means don't check width.
            ($field:ident, $length:expr) => {
                if let Some(value) = overlay.$field {
                    if $length > 0
                        && unicode_width::UnicodeWidthStr::width(value.as_str())
                            != $length
                    {
                        anyhow::bail!(
                            "{} must be {} characters wide",
                            stringify!($field),
                            $length
                        );
                    }
                    char_set.$field = value;
                }
            };
            ($field:ident, [$($fallback_field:ident),+], $length:expr) => {
                // Do the same, but for multiple fields, using the first one
                // as a default for the fallack fields.
                $(
                    if let (Some(value), None) =
                        (&overlay.$field, &overlay.$fallback_field)
                    {
                        char_set.$fallback_field = value.clone();
                    }
                )+
                validate_and_set!($field, $length);
                $(
                    validate_and_set!($fallback_field, $length);
                )+
            };
        }

        validate_and_set!(default_endpoint, 1);
        validate_and_set!(default_target, 1);
        validate_and_set!(object_selected_top, 1);
        validate_and_set!(object_selected_center, 1);
        validate_and_set!(object_selected_bottom, 1);
        validate_and_set!(objects_more, 0);
        validate_and_set!(volume_bar_background, 1);
        validate_and_set!(volume_bar_foreground, 1);
        validate_and_set!(
            meter_left,
            [meter_left_overload, meter_left_unlit],
            1
        );
        validate_and_set!(
            meter_right,
            [meter_right_overload, meter_right_unlit],
            1
        );
        validate_and_set!(meter_live_left, [meter_live_left_unlit], 1);
        validate_and_set!(meter_live_right, [meter_live_right_unlit], 1);
        validate_and_set!(dropdown, 1);
        validate_and_set!(dropdown_item_selected, 1);
        validate_and_set!(dropdown_more, 0);
        validate_and_set!(tab_selected_left, 1);
        validate_and_set!(tab_selected_right, 1);

        Ok(char_set)
    }
}

impl CharSet {
    pub fn defaults() -> HashMap<String, CharSet> {
        HashMap::from([(String::from("default"), Default::default())])
    }

    /// Merge deserialized charsets with defaults
    pub fn merge<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<String, CharSet>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let configured =
            HashMap::<String, CharSetOverlay>::deserialize(deserializer)?;
        let mut merged = configured
            .into_iter()
            .map(|(key, value)| {
                CharSet::try_from(value)
                    .map_err(D::Error::custom)
                    .map(move |charset| (key, charset))
            })
            .collect::<Result<HashMap<String, CharSet>, D::Error>>()?;
        if !merged.contains_key("default") {
            merged.insert(String::from("default"), Default::default());
        }
        Ok(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_overlay() {
        let config = r#""#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        CharSet::try_from(overlay).unwrap();
    }

    #[test]
    fn test_override_default() {
        let config = r#"
        dropdown = "$"
        "#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        let char_set = CharSet::try_from(overlay).unwrap();

        assert_eq!(char_set.dropdown, "$")
    }

    #[test]
    fn test_override_fallbacks_unset() {
        let config = r#"
        meter_right = "$"
        "#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        let char_set = CharSet::try_from(overlay).unwrap();

        assert_eq!(char_set.meter_right, "$");
        assert_eq!(char_set.meter_right_overload, "$");
        assert_eq!(char_set.meter_right_unlit, "$");
    }

    #[test]
    fn test_override_fallbacks_set() {
        let config = r#"
        meter_right = "$"
        meter_right_overload = "%"
        meter_right_unlit = "."
        "#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        let char_set = CharSet::try_from(overlay).unwrap();

        assert_eq!(char_set.meter_right, "$");
        assert_eq!(char_set.meter_right_overload, "%");
        assert_eq!(char_set.meter_right_unlit, ".");
    }

    #[test]
    fn test_width_too_narrow() {
        let config = r#"
        meter_right = ""
        "#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        let char_set = CharSet::try_from(overlay);
        assert!(char_set.is_err());
    }

    #[test]
    fn test_width_too_wide() {
        let config = r#"
        meter_right = "$$"
        "#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        let char_set = CharSet::try_from(overlay);
        assert!(char_set.is_err());
    }

    #[test]
    fn test_width_correct() {
        let config = r#"
        meter_right = "$"
        "#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        let char_set = CharSet::try_from(overlay).unwrap();
        assert_eq!(char_set.meter_right, "$");
    }

    #[test]
    fn test_width_grapheme_cluster() {
        let config = r#"
        meter_right = "⚓︎"
        "#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        let char_set = CharSet::try_from(overlay).unwrap();
        assert_eq!(char_set.meter_right, "⚓︎");
    }

    #[test]
    fn test_width_unlimited() {
        let config = r#"
        objects_more = ""
        dropdown_more = "$$$$$$$$$$$$$$$$$$$$$$$$"
        "#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        let char_set = CharSet::try_from(overlay).unwrap();
        assert_eq!(char_set.objects_more, "");
        assert_eq!(char_set.dropdown_more, "$$$$$$$$$$$$$$$$$$$$$$$$");
    }

    #[test]
    fn test_inherit_nonexistent() {
        let config = r#"
        inherit = "doesntexist"
        meter_right = "$"
        "#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        let char_set = CharSet::try_from(overlay);
        assert!(char_set.is_err());
    }

    #[test]
    fn test_inherit_default() {
        let config = r#"
        inherit = "default"
        meter_right = "$"
        "#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        let char_set = CharSet::try_from(overlay).unwrap();
        assert_eq!(char_set.meter_right, "$");
        assert_eq!(char_set.meter_left, "▮");
    }
}

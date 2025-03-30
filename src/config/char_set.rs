//! Implementation for [`CharSet`](`crate::config::CharSet`). Defines default
//! character sets and handles merging of configured char sets with defaults.

use std::collections::HashMap;

use ratatui::widgets::block::BorderType;
use serde::{de::Error, Deserialize};

use crate::config::CharSet;

// This is what actually gets parsed from the config.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct CharSetOverlay {
    inherit: Option<String>,
    default_device: Option<String>,
    default_stream: Option<String>,
    selector_top: Option<String>,
    selector_middle: Option<String>,
    selector_bottom: Option<String>,
    tab_marker_left: Option<String>,
    tab_marker_right: Option<String>,
    list_more: Option<String>,
    volume_empty: Option<String>,
    volume_filled: Option<String>,
    meter_left_inactive: Option<String>,
    meter_left_active: Option<String>,
    meter_left_overload: Option<String>,
    meter_right_inactive: Option<String>,
    meter_right_active: Option<String>,
    meter_right_overload: Option<String>,
    meter_center_left_inactive: Option<String>,
    meter_center_left_active: Option<String>,
    meter_center_right_inactive: Option<String>,
    meter_center_right_active: Option<String>,
    dropdown_icon: Option<String>,
    dropdown_selector: Option<String>,
    dropdown_more: Option<String>,
    dropdown_border: Option<BorderTypeDef>,
}

#[derive(Deserialize, Debug)]
enum BorderTypeDef {
    Plain,
    Rounded,
    Double,
    Thick,
    QuadrantInside,
    QuadrantOutside,
}

impl From<BorderTypeDef> for BorderType {
    fn from(def: BorderTypeDef) -> Self {
        match def {
            BorderTypeDef::Plain => Self::Plain,
            BorderTypeDef::Rounded => Self::Rounded,
            BorderTypeDef::Double => Self::Double,
            BorderTypeDef::Thick => Self::Thick,
            BorderTypeDef::QuadrantInside => Self::QuadrantInside,
            BorderTypeDef::QuadrantOutside => Self::QuadrantOutside,
        }
    }
}

impl TryFrom<CharSetOverlay> for CharSet {
    type Error = anyhow::Error;

    fn try_from(overlay: CharSetOverlay) -> Result<Self, Self::Error> {
        let mut char_set: Self = match overlay.inherit.as_deref() {
            Some("default") => CharSet::default(),
            Some("compat") => CharSet::compat(),
            Some("extracompat") => CharSet::extracompat(),
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
        }

        validate_and_set!(default_device, 1);
        validate_and_set!(default_stream, 1);
        validate_and_set!(selector_top, 1);
        validate_and_set!(selector_middle, 1);
        validate_and_set!(selector_bottom, 1);
        validate_and_set!(tab_marker_left, 1);
        validate_and_set!(tab_marker_right, 1);
        validate_and_set!(list_more, 0);
        validate_and_set!(volume_empty, 1);
        validate_and_set!(volume_filled, 1);
        validate_and_set!(meter_left_inactive, 1);
        validate_and_set!(meter_left_active, 1);
        validate_and_set!(meter_left_overload, 1);
        validate_and_set!(meter_right_inactive, 1);
        validate_and_set!(meter_right_active, 1);
        validate_and_set!(meter_right_overload, 1);
        validate_and_set!(meter_center_left_inactive, 1);
        validate_and_set!(meter_center_left_active, 1);
        validate_and_set!(meter_center_right_inactive, 1);
        validate_and_set!(meter_center_right_active, 1);
        validate_and_set!(dropdown_icon, 1);
        validate_and_set!(dropdown_selector, 1);
        validate_and_set!(dropdown_more, 0);

        if let Some(dropdown_border) = overlay.dropdown_border {
            char_set.dropdown_border = dropdown_border.into();
        }

        Ok(char_set)
    }
}

impl Default for CharSet {
    fn default() -> Self {
        Self {
            default_device: String::from("◇"),
            default_stream: String::from("◇"),
            selector_top: String::from("░"),
            selector_middle: String::from("▒"),
            selector_bottom: String::from("░"),
            tab_marker_left: String::from("["),
            tab_marker_right: String::from("]"),
            list_more: String::from("•••"),
            volume_empty: String::from("╌"),
            volume_filled: String::from("━"),
            meter_left_inactive: String::from("▮"),
            meter_left_active: String::from("▮"),
            meter_left_overload: String::from("▮"),
            meter_right_inactive: String::from("▮"),
            meter_right_active: String::from("▮"),
            meter_right_overload: String::from("▮"),
            meter_center_left_inactive: String::from("▮"),
            meter_center_left_active: String::from("▮"),
            meter_center_right_inactive: String::from("▮"),
            meter_center_right_active: String::from("▮"),
            dropdown_icon: String::from("▼"),
            dropdown_selector: String::from(">"),
            dropdown_more: String::from("•••"),
            dropdown_border: BorderType::Rounded,
        }
    }
}

impl CharSet {
    pub fn defaults() -> HashMap<String, CharSet> {
        HashMap::from([
            (String::from("default"), CharSet::default()),
            (String::from("compat"), CharSet::compat()),
            (String::from("extracompat"), CharSet::extracompat()),
        ])
    }

    fn compat() -> CharSet {
        Self {
            default_device: String::from("◊"),
            default_stream: String::from("◊"),
            selector_top: String::from("░"),
            selector_middle: String::from("▒"),
            selector_bottom: String::from("░"),
            tab_marker_left: String::from("["),
            tab_marker_right: String::from("]"),
            list_more: String::from("•••"),
            volume_empty: String::from("─"),
            volume_filled: String::from("━"),
            meter_left_inactive: String::from("┃"),
            meter_left_active: String::from("┃"),
            meter_left_overload: String::from("┃"),
            meter_right_inactive: String::from("┃"),
            meter_right_active: String::from("┃"),
            meter_right_overload: String::from("┃"),
            meter_center_left_inactive: String::from("█"),
            meter_center_left_active: String::from("█"),
            meter_center_right_inactive: String::from("█"),
            meter_center_right_active: String::from("█"),
            dropdown_icon: String::from("▼"),
            dropdown_selector: String::from(">"),
            dropdown_more: String::from("•••"),
            dropdown_border: BorderType::Plain,
        }
    }

    fn extracompat() -> CharSet {
        Self {
            default_device: String::from("*"),
            default_stream: String::from("*"),
            selector_top: String::from("-"),
            selector_middle: String::from("="),
            selector_bottom: String::from("-"),
            tab_marker_left: String::from("["),
            tab_marker_right: String::from("]"),
            list_more: String::from("~~~"),
            volume_empty: String::from("-"),
            volume_filled: String::from("="),
            meter_left_inactive: String::from("="),
            meter_left_active: String::from("#"),
            meter_left_overload: String::from("!"),
            meter_right_inactive: String::from("="),
            meter_right_active: String::from("#"),
            meter_right_overload: String::from("!"),
            meter_center_left_inactive: String::from("["),
            meter_center_left_active: String::from("["),
            meter_center_right_inactive: String::from("]"),
            meter_center_right_active: String::from("]"),
            dropdown_icon: String::from("\\"),
            dropdown_selector: String::from(">"),
            dropdown_more: String::from("~~~"),
            dropdown_border: BorderType::Plain,
        }
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
            merged.insert(String::from("default"), CharSet::default());
        }
        if !merged.contains_key("compat") {
            merged.insert(String::from("compat"), CharSet::compat());
        }
        if !merged.contains_key("extracompat") {
            merged.insert(String::from("extracompat"), CharSet::extracompat());
        }
        Ok(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_overlay() {
        let config = r#""#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        CharSet::try_from(overlay).unwrap();
    }

    #[test]
    fn builtins_present() {
        #[derive(Deserialize)]
        struct S {
            #[serde(deserialize_with = "CharSet::merge")]
            char_sets: HashMap<String, CharSet>,
        }
        let config = r#"[char_sets.test]"#;

        let s = toml::from_str::<S>(&config).unwrap();
        for name in CharSet::defaults().keys() {
            assert!(s.char_sets.contains_key(name));
        }
    }

    #[test]
    fn override_default() {
        let config = r#"
        dropdown_icon = "$"
        "#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        let char_set = CharSet::try_from(overlay).unwrap();

        assert_eq!(char_set.dropdown_icon, "$")
    }

    #[test]
    fn width_too_narrow() {
        let config = r#"
        meter_right_active = ""
        "#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        let char_set = CharSet::try_from(overlay);
        assert!(char_set.is_err());
    }

    #[test]
    fn width_too_wide() {
        let config = r#"
        meter_right_active = "$$"
        "#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        let char_set = CharSet::try_from(overlay);
        assert!(char_set.is_err());
    }

    #[test]
    fn width_correct() {
        let config = r#"
        meter_right_active = "$"
        "#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        let char_set = CharSet::try_from(overlay).unwrap();
        assert_eq!(char_set.meter_right_active, "$");
    }

    #[test]
    fn width_1_column_grapheme_cluster() {
        let config = r#"
        meter_right_active = "⚓︎"
        "#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        let char_set = CharSet::try_from(overlay).unwrap();
        assert_eq!(char_set.meter_right_active, "⚓︎");
    }

    #[test]
    fn width_2_column_grapheme_cluster() {
        let config = r#"
        meter_right_active = "🏳️‍🌈"
        "#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        let char_set = CharSet::try_from(overlay);
        assert!(char_set.is_err());
    }

    #[test]
    fn width_unlimited() {
        let config = r#"
        list_more = ""
        dropdown_more = "$$$$$$$$$$$$$$$$$$$$$$$$"
        "#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        let char_set = CharSet::try_from(overlay).unwrap();
        assert_eq!(char_set.list_more, "");
        assert_eq!(char_set.dropdown_more, "$$$$$$$$$$$$$$$$$$$$$$$$");
    }

    #[test]
    fn inherit_nonexistent() {
        let config = r#"
        inherit = "doesntexist"
        meter_right_active = "$"
        "#;

        let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
        let char_set = CharSet::try_from(overlay);
        assert!(char_set.is_err());
    }

    #[test]
    fn inherit() {
        for (builtin_key, builtin) in CharSet::defaults().iter() {
            let config = format!(
                r#"
            inherit = "{}"
            meter_right_active = "$"
            "#,
                builtin_key
            );

            let overlay = toml::from_str::<CharSetOverlay>(&config).unwrap();
            let char_set = CharSet::try_from(overlay).unwrap();
            assert_eq!(char_set.meter_right_active, "$");
            assert_eq!(char_set.meter_left_active, builtin.meter_left_active);
        }
    }

    #[test]
    fn unknown_field() {
        let config = r#"
        unknown = "unknown"
        "#;
        assert!(toml::from_str::<CharSetOverlay>(&config).is_err());
    }
}

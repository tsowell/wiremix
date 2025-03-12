//! Implementation for [`CharSet`](`crate::config::CharSet`). Defines default
//! character sets and handles merging of configured char sets with defaults.

use std::collections::HashMap;

use serde::{de::Error, Deserialize};

use crate::config::CharSet;

// This is what actually gets parsed from the config.
#[derive(Deserialize, Debug)]
struct CharSetOverlay {
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
        let mut char_set: Self = Default::default();

        macro_rules! validate_and_set {
            // Overwrite default char with char from overlay while validating
            // width. Length of 0 means don't check width.
            ($field:ident, $length:expr) => {
                if let Some(value) = overlay.$field {
                    if $length > 0
                        && unicode_width::UnicodeWidthStr::width(value.as_str())
                            != $length
                    {
                        anyhow::bail!(format!(
                            "{} must be {} characters wide",
                            stringify!($field),
                            $length
                        ));
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

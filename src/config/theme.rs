use std::collections::HashMap;

use ratatui::style::{Color, Modifier, Style};
use serde::{de::Error, Deserialize};

use crate::config::Theme;

// This is what actually gets parsed from the config.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ThemeOverlay {
    default_device: Option<StyleDef>,
    default_stream: Option<StyleDef>,
    selector: Option<StyleDef>,
    tab: Option<StyleDef>,
    tab_selected: Option<StyleDef>,
    tab_marker: Option<StyleDef>,
    list_more: Option<StyleDef>,
    node_title: Option<StyleDef>,
    node_target: Option<StyleDef>,
    volume: Option<StyleDef>,
    volume_empty: Option<StyleDef>,
    volume_filled: Option<StyleDef>,
    meter_inactive: Option<StyleDef>,
    meter_active: Option<StyleDef>,
    meter_overload: Option<StyleDef>,
    meter_center_inactive: Option<StyleDef>,
    meter_center_active: Option<StyleDef>,
    config_device: Option<StyleDef>,
    config_profile: Option<StyleDef>,
    dropdown_icon: Option<StyleDef>,
    dropdown_border: Option<StyleDef>,
    dropdown_item: Option<StyleDef>,
    dropdown_selected: Option<StyleDef>,
    dropdown_more: Option<StyleDef>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct StyleDef {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub underline_color: Option<Color>,
    #[serde(default = "default_modifier")]
    pub add_modifier: Modifier,
    #[serde(default = "default_modifier")]
    pub sub_modifier: Modifier,
}

fn default_modifier() -> Modifier {
    Modifier::empty()
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            default_device: Style::default(),
            default_stream: Style::default(),
            selector: Style::default().fg(Color::LightCyan),
            tab: Style::default(),
            tab_selected: Style::default().fg(Color::LightCyan),
            tab_marker: Style::default().fg(Color::LightCyan),
            list_more: Style::default().fg(Color::DarkGray),
            node_title: Style::default(),
            node_target: Style::default(),
            volume: Style::default(),
            volume_empty: Style::default().fg(Color::DarkGray),
            volume_filled: Style::default().fg(Color::LightBlue),
            meter_inactive: Style::default().fg(Color::DarkGray),
            meter_active: Style::default().fg(Color::LightGreen),
            meter_overload: Style::default().fg(Color::Red),
            meter_center_inactive: Style::default().fg(Color::DarkGray),
            meter_center_active: Style::default().fg(Color::LightGreen),
            config_device: Style::default(),
            config_profile: Style::default(),
            dropdown_icon: Style::default(),
            dropdown_border: Style::default(),
            dropdown_item: Style::default(),
            dropdown_selected: Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::REVERSED),
            dropdown_more: Style::default().fg(Color::DarkGray),
        }
    }
}

impl From<StyleDef> for Style {
    fn from(def: StyleDef) -> Self {
        Self {
            fg: def.fg,
            bg: def.bg,
            underline_color: def.underline_color,
            add_modifier: def.add_modifier,
            sub_modifier: def.sub_modifier,
        }
    }
}

impl TryFrom<ThemeOverlay> for Theme {
    type Error = anyhow::Error;

    fn try_from(overlay: ThemeOverlay) -> Result<Self, Self::Error> {
        let mut theme = Theme::default();

        macro_rules! set {
            ($field:ident) => {
                if let Some($field) = overlay.$field {
                    theme.$field = $field.into();
                }
            };
        }

        set!(default_device);
        set!(default_stream);
        set!(selector);
        set!(tab);
        set!(tab_selected);
        set!(tab_marker);
        set!(list_more);
        set!(node_title);
        set!(node_target);
        set!(volume);
        set!(volume_empty);
        set!(volume_filled);
        set!(meter_inactive);
        set!(meter_active);
        set!(meter_overload);
        set!(meter_center_inactive);
        set!(meter_center_active);
        set!(config_device);
        set!(config_profile);
        set!(dropdown_icon);
        set!(dropdown_border);
        set!(dropdown_item);
        set!(dropdown_selected);
        set!(dropdown_more);

        Ok(theme)
    }
}

impl Theme {
    pub fn defaults() -> HashMap<String, Theme> {
        HashMap::from([(String::from("default"), Theme::default())])
    }

    /// Merge deserialized themes with defaults
    pub fn merge<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<String, Theme>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let configured =
            HashMap::<String, ThemeOverlay>::deserialize(deserializer)?;
        let mut merged = configured
            .into_iter()
            .map(|(key, value)| {
                Theme::try_from(value)
                    .map_err(D::Error::custom)
                    .map(move |theme| (key, theme))
            })
            .collect::<Result<HashMap<String, Theme>, D::Error>>()?;
        if !merged.contains_key("default") {
            merged.insert(String::from("default"), Theme::default());
        }
        Ok(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unknown_field_theme() {
        let config = r#"
        unknown = "unknown"
        "#;
        assert!(toml::from_str::<ThemeOverlay>(&config).is_err());
    }

    #[test]
    fn test_unknown_field_style() {
        let config = r#"
        unknown = "unknown"
        "#;
        assert!(toml::from_str::<StyleDef>(&config).is_err());
    }
}

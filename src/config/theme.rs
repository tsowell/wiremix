use std::collections::HashMap;

use ratatui::style::{Color, Modifier, Style};
use serde::{de::Error, Deserialize};

use crate::config::Theme;

// This is what actually gets parsed from the config.
#[derive(Deserialize, Debug)]
pub struct ThemeOverlay {
    tab: Option<StyleDef>,
    tab_selected: Option<StyleDef>,
    tab_selected_symbols: Option<StyleDef>,
    object_selected_symbols: Option<StyleDef>,
    objects_more: Option<StyleDef>,
    node_name: Option<StyleDef>,
    node_default_symbol: Option<StyleDef>,
    volume: Option<StyleDef>,
    volume_bar_foreground: Option<StyleDef>,
    volume_bar_background: Option<StyleDef>,
    meter_unlit: Option<StyleDef>,
    meter: Option<StyleDef>,
    meter_overload: Option<StyleDef>,
    meter_live_unlit: Option<StyleDef>,
    meter_live: Option<StyleDef>,
    target: Option<StyleDef>,
    target_default_symbol: Option<StyleDef>,
    device_name: Option<StyleDef>,
    device_dropdown_symbol: Option<StyleDef>,
    device_profile: Option<StyleDef>,
    dropdown_border: Option<StyleDef>,
    dropdown_item: Option<StyleDef>,
    dropdown_item_selected: Option<StyleDef>,
    dropdown_more: Option<StyleDef>,
}

#[derive(Deserialize, Debug)]
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
            tab: Style::default(),
            tab_selected: Style::default().fg(Color::LightCyan),
            tab_selected_symbols: Style::default().fg(Color::LightCyan),
            object_selected_symbols: Style::default().fg(Color::LightCyan),
            objects_more: Style::default().fg(Color::DarkGray),
            node_name: Style::default(),
            node_default_symbol: Style::default(),
            volume: Style::default(),
            volume_bar_foreground: Style::default().fg(Color::Blue),
            volume_bar_background: Style::default().fg(Color::DarkGray),
            meter_unlit: Style::default().fg(Color::DarkGray),
            meter: Style::default().fg(Color::LightGreen),
            meter_overload: Style::default().fg(Color::Red),
            meter_live_unlit: Style::default().fg(Color::DarkGray),
            meter_live: Style::default().fg(Color::LightGreen),
            target: Style::default(),
            target_default_symbol: Style::default(),
            device_name: Style::default(),
            device_dropdown_symbol: Style::default(),
            device_profile: Style::default(),
            dropdown_border: Style::default(),
            dropdown_item: Style::default(),
            dropdown_item_selected: Style::default()
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

        set!(tab);
        set!(tab_selected);
        set!(tab_selected_symbols);
        set!(object_selected_symbols);
        set!(objects_more);
        set!(node_name);
        set!(node_default_symbol);
        set!(volume);
        set!(volume_bar_foreground);
        set!(volume_bar_background);
        set!(meter_unlit);
        set!(meter);
        set!(meter_overload);
        set!(meter_live_unlit);
        set!(meter_live);
        set!(target);
        set!(target_default_symbol);
        set!(device_name);
        set!(device_dropdown_symbol);
        set!(device_profile);
        set!(dropdown_border);
        set!(dropdown_item);
        set!(dropdown_item_selected);
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

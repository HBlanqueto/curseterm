use std::sync::LazyLock;

use alacritty_terminal::vte::ansi::Rgb;
use barrier_kit_next::tokens::MONO;
use serde::{Deserialize, Deserializer, Serialize};

pub const APP_ID: &str = "org.curseterm.CurseTerm";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Defaults {
    pub font_family: Option<String>,
    pub font_size: Option<u32>,
    pub color_scheme: Option<String>,
}

impl Defaults {
    pub fn font_family(&self) -> &str {
        self.font_family.as_deref().unwrap_or(MONO.family)
    }

    pub fn font_size(&self) -> u32 {
        self.font_size.unwrap_or(MONO.size)
    }

    pub fn scheme(&self, dark: bool) -> &'static Scheme {
        let named = self
            .color_scheme
            .as_deref()
            .and_then(|name| SCHEMES.iter().find(|scheme| scheme.name == name));
        named.unwrap_or(&SCHEMES[usize::from(dark)])
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Scheme {
    pub name: String,
    #[serde(deserialize_with = "hex")]
    pub foreground: Rgb,
    #[serde(deserialize_with = "hex")]
    pub background: Rgb,
    #[serde(deserialize_with = "hex")]
    pub cursor: Rgb,
    #[serde(deserialize_with = "hex")]
    pub cursor_text: Rgb,
    #[serde(deserialize_with = "hex")]
    pub selection: Rgb,
    #[serde(deserialize_with = "hex_list")]
    pub ansi: [Rgb; 16],
}

pub static SCHEMES: LazyLock<Vec<Scheme>> = LazyLock::new(|| {
    [
        include_str!("../schemes/claro.toml"),
        include_str!("../schemes/oscuro.toml"),
    ]
    .iter()
    .map(|source| toml::from_str(source).expect("built-in scheme"))
    .collect()
});

fn parse_hex(text: &str) -> Option<Rgb> {
    let digits = text.strip_prefix('#')?;
    if digits.len() != 6 {
        return None;
    }
    let value = u32::from_str_radix(digits, 16).ok()?;
    Some(Rgb {
        r: (value >> 16) as u8,
        g: (value >> 8) as u8,
        b: value as u8,
    })
}

fn hex<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Rgb, D::Error> {
    let text = String::deserialize(deserializer)?;
    parse_hex(&text)
        .ok_or_else(|| serde::de::Error::custom(format!("not a #rrggbb colour: {text}")))
}

fn hex_list<'de, D: Deserializer<'de>>(deserializer: D) -> Result<[Rgb; 16], D::Error> {
    let texts = Vec::<String>::deserialize(deserializer)?;
    let colours: Option<Vec<Rgb>> = texts.iter().map(|text| parse_hex(text)).collect();
    colours
        .and_then(|colours| colours.try_into().ok())
        .ok_or_else(|| serde::de::Error::custom("ansi needs 16 #rrggbb colours"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_schemes_parse_light_first_then_dark() {
        assert_eq!(SCHEMES.len(), 2);
        let defaults = Defaults::default();
        assert_eq!(defaults.scheme(false).name, "CurseTerm Claro");
        assert_eq!(defaults.scheme(true).name, "CurseTerm Oscuro");
    }

    #[test]
    fn a_named_scheme_wins_over_the_system_appearance() {
        let defaults = Defaults {
            color_scheme: Some("CurseTerm Oscuro".into()),
            ..Defaults::default()
        };
        assert_eq!(defaults.scheme(false).name, "CurseTerm Oscuro");
    }
}

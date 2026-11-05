use std::collections::HashMap;
use std::path::Path;

use alacritty_terminal::vte::ansi::Rgb;
use serde::Deserialize;

use crate::config::Scheme;

#[derive(Deserialize)]
struct Component {
    #[serde(rename = "Red Component")]
    red: f64,
    #[serde(rename = "Green Component")]
    green: f64,
    #[serde(rename = "Blue Component")]
    blue: f64,
}

impl Component {
    fn rgb(&self) -> Rgb {
        let channel = |value: f64| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        Rgb {
            r: channel(self.red),
            g: channel(self.green),
            b: channel(self.blue),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("cannot read the file: {0}")]
    Read(#[from] std::io::Error),
    #[error("not an .itermcolors property list: {0}")]
    Format(#[from] plist::Error),
    #[error("the scheme has no «{0}»")]
    Missing(String),
}

pub fn import(path: &Path) -> Result<Scheme, ImportError> {
    let name = path.file_stem().map_or_else(
        || "Imported".into(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    parse(&std::fs::read(path)?, name)
}

fn parse(bytes: &[u8], name: String) -> Result<Scheme, ImportError> {
    let colours: HashMap<String, Component> = plist::from_bytes(bytes)?;
    let colour = |key: &str| {
        colours
            .get(key)
            .map(Component::rgb)
            .ok_or_else(|| ImportError::Missing(key.to_string()))
    };
    let mut ansi = [Rgb::default(); 16];
    for (index, slot) in ansi.iter_mut().enumerate() {
        *slot = colour(&format!("Ansi {index} Color"))?;
    }
    let background = colour("Background Color")?;
    let foreground = colour("Foreground Color")?;
    Ok(Scheme {
        name,
        cursor: colour("Cursor Color").unwrap_or(foreground),
        cursor_text: colour("Cursor Text Color").unwrap_or(background),
        selection: colour("Selection Color").unwrap_or(ansi[8]),
        foreground,
        background,
        ansi,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, red: f64, green: f64, blue: f64) -> String {
        format!(
            "<key>{key}</key><dict><key>Alpha Component</key><real>1</real>\
             <key>Blue Component</key><real>{blue}</real>\
             <key>Color Space</key><string>sRGB</string>\
             <key>Green Component</key><real>{green}</real>\
             <key>Red Component</key><real>{red}</real></dict>"
        )
    }

    fn plist(entries: &[String]) -> Vec<u8> {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
             <plist version=\"1.0\"><dict>{}</dict></plist>",
            entries.concat()
        )
        .into_bytes()
    }

    #[test]
    fn an_itermcolors_file_maps_its_ansi_background_and_foreground_and_fills_the_rest() {
        let mut entries: Vec<String> = (0..16)
            .map(|index| {
                entry(
                    &format!("Ansi {index} Color"),
                    index as f64 / 15.0,
                    0.0,
                    0.0,
                )
            })
            .collect();
        entries.push(entry("Background Color", 0.0, 0.0, 0.0));
        entries.push(entry("Foreground Color", 1.0, 1.0, 1.0));
        let scheme = parse(&plist(&entries), "Prueba".into()).expect("valid scheme");
        assert_eq!(scheme.ansi[15], Rgb { r: 255, g: 0, b: 0 });
        assert_eq!(scheme.ansi[1].r, 17);
        assert_eq!(scheme.cursor, scheme.foreground);
        assert_eq!(scheme.cursor_text, scheme.background);
        assert_eq!(scheme.selection, scheme.ansi[8]);
    }

    #[test]
    fn a_scheme_without_ansi_colours_is_rejected_by_name() {
        let entries = vec![entry("Background Color", 0.0, 0.0, 0.0)];
        let error = parse(&plist(&entries), "Mala".into()).expect_err("incomplete scheme");
        assert!(error.to_string().contains("Ansi 0 Color"));
    }
}

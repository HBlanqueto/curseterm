use std::path::PathBuf;
use std::sync::LazyLock;

use alacritty_terminal::vte::ansi::Rgb;
use barrier_kit_next::tokens::{MONO, TERMINAL};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::arrangement::Arrangement;
use crate::find_mode::FindMode;
use crate::key_bindings::Binding;
use crate::paste_warning::PasteWarnings;
use crate::snippets::Snippet;

pub const APP_ID: &str = "org.curseterm.CurseTerm";
pub const APP_NAME: &str = "CurseTerm";
const DEFAULT_PROFILE: &str = "Default";
// Grid::clear_history reallocates every row up to this limit (alacritty_terminal 0.26 grid/mod.rs).
pub const SCROLLBACK_LIMIT: u32 = 100_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Defaults {
    pub default_profile: Option<String>,
    pub profiles: Vec<Profile>,
    pub schemes: Vec<Scheme>,
    pub show_tab_bar: bool,
    pub copy_on_select: bool,
    pub focus_follows_mouse: bool,
    pub arrangements: Vec<Arrangement>,
    pub mark_alert: MarkAlert,
    pub alert_on_offscreen_marks: bool,
    pub theme: Theme,
    pub paste_warnings: PasteWarnings,
    pub paste_tab_spaces: u8,
    pub find_mode: FindMode,
    pub filter_mode: FindMode,
    pub global_search_mode: FindMode,
    pub snippets: Vec<Snippet>,
    pub key_bindings: Vec<Binding>,
    pub show_tabs_in_fullscreen: bool,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            default_profile: None,
            profiles: Vec::new(),
            schemes: Vec::new(),
            show_tab_bar: false,
            copy_on_select: true,
            focus_follows_mouse: false,
            arrangements: Vec::new(),
            mark_alert: MarkAlert::default(),
            alert_on_offscreen_marks: false,
            theme: Theme::default(),
            paste_warnings: PasteWarnings::default(),
            // iTermNumberOfSpacesAccessoryViewController.m: four by default.
            paste_tab_spaces: 4,
            find_mode: FindMode::default(),
            filter_mode: FindMode::default(),
            global_search_mode: FindMode::default(),
            snippets: Vec::new(),
            key_bindings: Vec::new(),
            // iTermPreferences.m: ShowFullScreenTabBar YES.
            show_tabs_in_fullscreen: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub name: String,
    pub font_family: String,
    pub font_size: u32,
    pub color_scheme: Option<String>,
    pub command: Option<String>,
    pub initial_text: String,
    pub working_directory: WorkingDirectory,
    pub triggers: Vec<Trigger>,
    pub scrollback_lines: u32,
    pub flash_visual_bell: bool,
    pub bell_icon_in_tabs: bool,
    pub columns: u16,
    pub rows: u16,
    pub when_session_ends: SessionEnd,
    pub prompt_before_closing: ClosePrompt,
    pub alt_key: AltKey,
    pub report_keys_csi_u: bool,
    pub apps_change_key_reporting: bool,
    pub key_mappings: Vec<Binding>,
    pub notifications: bool,
    pub notify_bell: bool,
    pub notify_idle: bool,
    pub notify_new_output: bool,
    pub notify_session_ended: bool,
    pub notify_escape_sequences: bool,
    pub suppress_alerts_in_active_session: bool,
    pub badge: String,
    pub transparency: u8,
    pub status_bar: bool,
    pub cursor: CursorType,
    pub blinking_cursor: bool,
    pub cursor_guide: bool,
    pub window_style: WindowStyle,
}

// iTermPreferences.h iTermPreferencesTabStyle, in the order of PreferencePanel.xib's Theme
// popup; +computedTabStyle makes Regular the default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    Regular,
    Minimal,
    Compact,
    Light,
    Dark,
    LightHighContrast,
    DarkHighContrast,
}

impl Theme {
    pub const ALL: [Self; 7] = [
        Self::Regular,
        Self::Minimal,
        Self::Compact,
        Self::Light,
        Self::Dark,
        Self::LightHighContrast,
        Self::DarkHighContrast,
    ];
}

// iTermApplicationDelegate.m -markAlertAction: posting a notification is the default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkAlert {
    ModalAlert,
    #[default]
    PostNotification,
    PlaySound,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEnd {
    #[default]
    Close,
    Restart,
    KeepOpen,
}

impl SessionEnd {
    pub const ALL: [Self; 3] = [Self::Close, Self::Restart, Self::KeepOpen];
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClosePrompt {
    #[default]
    IfJobs,
    Always,
    Never,
}

impl ClosePrompt {
    pub const ALL: [Self; 3] = [Self::IfJobs, Self::Always, Self::Never];
}

// ITAddressBookMgr.h WINDOW_TYPE_*: the styles a Wayland client can ask for; where the window
// goes is the compositor's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowStyle {
    #[default]
    Normal,
    FullScreen,
    Maximized,
    NoTitleBar,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AltKey {
    Normal,
    #[default]
    Escape,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorType {
    #[default]
    Box,
    VerticalBar,
    Underline,
}

impl CursorType {
    pub const ALL: [Self; 3] = [Self::Box, Self::VerticalBar, Self::Underline];
}

impl AltKey {
    pub const ALL: [Self; 2] = [Self::Normal, Self::Escape];
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Trigger {
    pub regex: String,
    pub action: TriggerAction,
    pub parameter: String,
    pub instant: bool,
    pub enabled: bool,
}

impl Default for Trigger {
    fn default() -> Self {
        Self {
            regex: String::new(),
            action: TriggerAction::default(),
            parameter: String::new(),
            instant: false,
            enabled: true,
        }
    }
}

// TriggerController.m +triggerClassesForTerminal:, sorted by title as iTerm2 lists them; the
// ones that need macOS, the Python API or a browser are left out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerAction {
    Annotate,
    HighlightLine,
    #[default]
    Highlight,
    Hyperlink,
    Notification,
    PromptDetected,
    ReportDirectory,
    Bell,
    RunCommand,
    Coprocess,
    SendText,
    Mark,
    SetTitle,
    Alert,
    Stop,
}

impl TriggerAction {
    pub const ALL: [Self; 15] = [
        Self::Annotate,
        Self::HighlightLine,
        Self::Highlight,
        Self::Hyperlink,
        Self::Notification,
        Self::PromptDetected,
        Self::ReportDirectory,
        Self::Bell,
        Self::RunCommand,
        Self::Coprocess,
        Self::SendText,
        Self::Mark,
        Self::SetTitle,
        Self::Alert,
        Self::Stop,
    ];

    // Trigger.m -takesParameter.
    pub fn takes_parameter(self) -> bool {
        !matches!(
            self,
            Self::Bell | Self::PromptDetected | Self::Mark | Self::Stop
        )
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: DEFAULT_PROFILE.into(),
            font_family: MONO.family.into(),
            font_size: MONO.size,
            color_scheme: None,
            command: None,
            initial_text: String::new(),
            working_directory: WorkingDirectory::Home,
            triggers: Vec::new(),
            scrollback_lines: 1000,
            flash_visual_bell: true,
            bell_icon_in_tabs: true,
            columns: 80,
            rows: 25,
            when_session_ends: SessionEnd::Close,
            prompt_before_closing: ClosePrompt::IfJobs,
            alt_key: AltKey::Escape,
            // iTermProfilePreferences.m: KEY_USE_LIBTICKIT_PROTOCOL NO, KEY_ALLOW_MODIFY_OTHER_KEYS YES.
            report_keys_csi_u: false,
            apps_change_key_reporting: true,
            key_mappings: Vec::new(),
            notifications: false,
            notify_bell: true,
            notify_idle: false,
            notify_new_output: false,
            notify_session_ended: true,
            notify_escape_sequences: true,
            suppress_alerts_in_active_session: false,
            badge: String::new(),
            transparency: 0,
            status_bar: false,
            cursor: CursorType::Box,
            blinking_cursor: false,
            cursor_guide: false,
            window_style: WindowStyle::Normal,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkingDirectory {
    #[default]
    Home,
    Previous,
    Fixed(PathBuf),
}

impl Profile {
    pub fn opacity(&self) -> f32 {
        1.0 - f32::from(self.transparency.min(90)) / 100.0
    }

    pub fn text_size(&self, zoom: i32) -> f32 {
        let (smallest, largest) = TERMINAL.font_sizes;
        (self.font_size as i32 + zoom).clamp(smallest as i32, largest as i32) as f32
    }
}

impl Defaults {
    pub fn profiles(&self) -> Vec<Profile> {
        if self.profiles.is_empty() {
            vec![Profile::default()]
        } else {
            self.profiles.clone()
        }
    }

    pub fn default_profile_name(&self) -> String {
        let profiles = self.profiles();
        let named = self
            .default_profile
            .as_ref()
            .filter(|name| profiles.iter().any(|profile| &profile.name == *name));
        named.cloned().unwrap_or_else(|| profiles[0].name.clone())
    }

    pub fn profile(&self, name: &str) -> Profile {
        let profiles = self.profiles();
        let wanted = profiles
            .iter()
            .find(|profile| profile.name == name)
            .cloned();
        wanted.unwrap_or_else(|| {
            let fallback = self.default_profile_name();
            profiles
                .into_iter()
                .find(|profile| profile.name == fallback)
                .unwrap_or_default()
        })
    }

    pub fn schemes(&self) -> impl Iterator<Item = &Scheme> {
        SCHEMES.iter().chain(&self.schemes)
    }

    pub fn scheme(&self, profile: &Profile, dark: bool) -> Scheme {
        profile
            .color_scheme
            .as_deref()
            .and_then(|name| self.schemes().find(|scheme| scheme.name == name))
            .unwrap_or(&SCHEMES[usize::from(dark)])
            .clone()
    }

    pub fn ensure_profiles(&mut self) {
        if self.profiles.is_empty() {
            self.profiles = self.profiles();
        }
    }

    pub fn unique_profile_name(&self, base: &str) -> String {
        let taken = |name: &str| self.profiles().iter().any(|profile| profile.name == name);
        (1..)
            .map(|copy| match copy {
                1 => base.to_string(),
                _ => format!("{base} {copy}"),
            })
            .find(|name| !taken(name))
            .unwrap_or_default()
    }
}

pub fn change_defaults(change: impl FnOnce(&mut Defaults)) {
    let mut defaults: Defaults = barrier_kit::app::load_defaults(APP_ID);
    let before = defaults.clone();
    change(&mut defaults);
    if defaults != before
        && let Err(error) = barrier_kit::app::save_defaults(APP_ID, &defaults)
    {
        tracing::warn!(%error, "could not save the settings");
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scheme {
    pub name: String,
    #[serde(with = "hex")]
    pub foreground: Rgb,
    #[serde(with = "hex")]
    pub background: Rgb,
    #[serde(with = "hex")]
    pub cursor: Rgb,
    #[serde(with = "hex")]
    pub cursor_text: Rgb,
    #[serde(with = "hex")]
    pub selection: Rgb,
    #[serde(with = "hex_list")]
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

fn to_hex(rgb: &Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb.r, rgb.g, rgb.b)
}

mod hex {
    use super::*;

    pub fn serialize<S: Serializer>(rgb: &Rgb, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&to_hex(rgb))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Rgb, D::Error> {
        let text = String::deserialize(deserializer)?;
        parse_hex(&text)
            .ok_or_else(|| serde::de::Error::custom(format!("not a #rrggbb colour: {text}")))
    }
}

mod hex_list {
    use super::*;

    pub fn serialize<S: Serializer>(ansi: &[Rgb; 16], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(ansi.iter().map(to_hex))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<[Rgb; 16], D::Error> {
        let texts = Vec::<String>::deserialize(deserializer)?;
        let colours: Option<Vec<Rgb>> = texts.iter().map(|text| parse_hex(text)).collect();
        colours
            .and_then(|colours| colours.try_into().ok())
            .ok_or_else(|| serde::de::Error::custom("ansi needs 16 #rrggbb colours"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_saved_profiles_there_is_one_default_that_follows_the_system() {
        let defaults = Defaults::default();
        let profile = defaults.profile("missing");
        assert_eq!(profile.name, DEFAULT_PROFILE);
        assert_eq!(defaults.scheme(&profile, false).name, "CurseTerm Claro");
        assert_eq!(defaults.scheme(&profile, true).name, "CurseTerm Oscuro");
    }

    #[test]
    fn profiles_and_imported_schemes_survive_a_round_trip_through_the_domain_file() {
        let mut imported = SCHEMES[1].clone();
        imported.name = "Importado".into();
        let defaults = Defaults {
            default_profile: Some("Trabajo".into()),
            profiles: vec![Profile {
                name: "Trabajo".into(),
                color_scheme: Some("Importado".into()),
                working_directory: WorkingDirectory::Fixed("/tmp".into()),
                command: Some("htop".into()),
                ..Profile::default()
            }],
            schemes: vec![imported],
            show_tab_bar: true,
            ..Defaults::default()
        };
        let text = toml::to_string(&defaults).expect("serialize");
        let back: Defaults = toml::from_str(&text).expect("deserialize");
        assert_eq!(back, defaults);
        let profile = back.profile(&back.default_profile_name());
        assert_eq!(back.scheme(&profile, false).name, "Importado");
    }

    #[test]
    fn a_new_profile_name_never_repeats_an_existing_one() {
        let defaults = Defaults::default();
        assert_eq!(defaults.unique_profile_name("Default"), "Default 2");
        assert_eq!(defaults.unique_profile_name("Otro"), "Otro");
    }
}

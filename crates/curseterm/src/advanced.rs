use std::sync::{LazyLock, RwLock};
use std::time::Duration;

use barrier_kit_next::tokens::TERMINAL;
use serde::{Deserialize, Serialize};

use crate::hints::URL_PATTERN;
use crate::localize::tr;

// iTermAdvancedSettingsModel.m: the settings CurseTerm has behaviour for, with iTerm2's defaults.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Advanced {
    pub idle_time_seconds: f64,
    pub alert_trigger_rate_limit: f64,
    pub user_notification_trigger_rate_limit: f64,
    pub move_left_after_closing_tab: bool,
    pub find_urls_regex: String,
    pub time_between_blinks: f64,
    pub slow_paste_bytes_per_call: f64,
    pub slow_paste_delay_between_calls: f64,
    pub paste_history_max_options: f64,
}

impl Default for Advanced {
    fn default() -> Self {
        Self {
            idle_time_seconds: 2.0,
            alert_trigger_rate_limit: 1.0,
            user_notification_trigger_rate_limit: 0.0,
            move_left_after_closing_tab: false,
            find_urls_regex: URL_PATTERN.to_string(),
            time_between_blinks: TERMINAL.cursor_blink_ms as f64 / 1000.0,
            slow_paste_bytes_per_call: 16.0,
            slow_paste_delay_between_calls: 0.125,
            paste_history_max_options: 20.0,
        }
    }
}

fn seconds(value: f64) -> Duration {
    Duration::from_secs_f64(value.max(0.0))
}

impl Advanced {
    // PTYSession.m -isIdle adds a second to idleTimeSeconds.
    pub fn idle_after(&self) -> Duration {
        seconds(self.idle_time_seconds + 1.0)
    }

    pub fn alert_rate_limit(&self) -> Duration {
        seconds(self.alert_trigger_rate_limit)
    }

    pub fn notification_rate_limit(&self) -> Duration {
        seconds(self.user_notification_trigger_rate_limit)
    }

    pub fn blink_period(&self) -> Duration {
        seconds(self.time_between_blinks).max(Duration::from_millis(1))
    }

    pub fn slow_paste_chunk(&self) -> usize {
        (self.slow_paste_bytes_per_call as usize).max(1)
    }

    pub fn slow_paste_pause(&self) -> Duration {
        seconds(self.slow_paste_delay_between_calls)
    }

    pub fn paste_history_limit(&self) -> usize {
        self.paste_history_max_options as usize
    }
}

static CURRENT: LazyLock<RwLock<Advanced>> = LazyLock::new(RwLock::default);

pub fn current() -> Advanced {
    CURRENT
        .read()
        .map(|advanced| advanced.clone())
        .unwrap_or_default()
}

pub fn store(advanced: &Advanced) {
    if let Ok(mut current) = CURRENT.write()
        && *current != *advanced
    {
        *current = advanced.clone();
    }
}

#[derive(Clone, Copy)]
pub enum Value {
    Switch(fn(&Advanced) -> bool, fn(&mut Advanced, bool)),
    Number(fn(&Advanced) -> f64, fn(&mut Advanced, f64)),
    Text(fn(&Advanced) -> &str, fn(&mut Advanced, String)),
}

#[derive(Clone, Copy)]
pub struct Entry {
    pub section: fn() -> &'static str,
    pub description: fn() -> &'static str,
    pub value: Value,
}

// iTermAdvancedSettingsViewController.m: grouped by section, in the model's order.
pub const ENTRIES: [Entry; 9] = [
    Entry {
        section: || tr!("advanced-section-tabs"),
        description: || tr!("advanced-move-left-after-closing-tab"),
        value: Value::Switch(
            |advanced| advanced.move_left_after_closing_tab,
            |advanced, on| advanced.move_left_after_closing_tab = on,
        ),
    },
    Entry {
        section: || tr!("advanced-section-terminal"),
        description: || tr!("advanced-time-between-blinks"),
        value: Value::Number(
            |advanced| advanced.time_between_blinks,
            |advanced, value| advanced.time_between_blinks = value,
        ),
    },
    Entry {
        section: || tr!("advanced-section-terminal"),
        description: || tr!("advanced-find-urls-regex"),
        value: Value::Text(
            |advanced| &advanced.find_urls_regex,
            |advanced, value| advanced.find_urls_regex = value,
        ),
    },
    Entry {
        section: || tr!("advanced-section-general"),
        description: || tr!("advanced-idle-time-seconds"),
        value: Value::Number(
            |advanced| advanced.idle_time_seconds,
            |advanced, value| advanced.idle_time_seconds = value,
        ),
    },
    Entry {
        section: || tr!("advanced-section-general"),
        description: || tr!("advanced-alert-trigger-rate-limit"),
        value: Value::Number(
            |advanced| advanced.alert_trigger_rate_limit,
            |advanced, value| advanced.alert_trigger_rate_limit = value,
        ),
    },
    Entry {
        section: || tr!("advanced-section-general"),
        description: || tr!("advanced-user-notification-trigger-rate-limit"),
        value: Value::Number(
            |advanced| advanced.user_notification_trigger_rate_limit,
            |advanced, value| advanced.user_notification_trigger_rate_limit = value,
        ),
    },
    Entry {
        section: || tr!("advanced-section-pasteboard"),
        description: || tr!("advanced-slow-paste-bytes-per-call"),
        value: Value::Number(
            |advanced| advanced.slow_paste_bytes_per_call,
            |advanced, value| advanced.slow_paste_bytes_per_call = value,
        ),
    },
    Entry {
        section: || tr!("advanced-section-pasteboard"),
        description: || tr!("advanced-slow-paste-delay-between-calls"),
        value: Value::Number(
            |advanced| advanced.slow_paste_delay_between_calls,
            |advanced, value| advanced.slow_paste_delay_between_calls = value,
        ),
    },
    Entry {
        section: || tr!("advanced-section-pasteboard"),
        description: || tr!("advanced-paste-history-max-options"),
        value: Value::Number(
            |advanced| advanced.paste_history_max_options,
            |advanced, value| advanced.paste_history_max_options = value,
        ),
    },
];

// iTermAdvancedSettingsViewController.m -filteredAdvancedSettings: every word of the query in the
// section or the description.
pub fn matches(entry: &Entry, query: &str) -> bool {
    let haystack = format!("{} {}", (entry.section)(), (entry.description)()).to_lowercase();
    query
        .to_lowercase()
        .split_whitespace()
        .all(|word| haystack.contains(word))
}

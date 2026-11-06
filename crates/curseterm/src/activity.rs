use std::time::{Duration, Instant};

// iTermAdvancedSettingsModel.m idleTimeSeconds (2) plus the second PTYSession.m -isIdle adds.
pub const IDLE_AFTER: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, Default)]
pub struct Activity {
    last_output: Option<Instant>,
    new_output: bool,
    posted_new_output: bool,
    posted_idle: bool,
}

impl Activity {
    // PTYSession.m -updateDisplayBecause: and PTYTab.m -setLabelAttributesForActiveTab:; true
    // when a New Output notification is due.
    pub fn output(&mut self, at: Instant, seen: bool, may_notify: bool) -> bool {
        self.last_output = Some(at);
        if seen {
            self.new_output = false;
            return false;
        }
        self.new_output = true;
        if self.posted_new_output || !may_notify {
            return false;
        }
        self.posted_new_output = true;
        self.posted_idle = false;
        true
    }

    // PTYTab.m -setLabelAttributesForIdleTab; true when an Idle notification is due.
    pub fn idle(&mut self, at: Instant, may_notify: bool) -> bool {
        let idle = self
            .last_output
            .is_some_and(|last| at.duration_since(last) >= IDLE_AFTER);
        if !self.new_output || !idle || self.posted_idle || !may_notify {
            return false;
        }
        self.posted_idle = true;
        self.posted_new_output = false;
        true
    }

    pub fn idle_at(&self) -> Option<Instant> {
        self.last_output.map(|last| last + IDLE_AFTER)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unseen_output_notifies_once_then_idle_notifies_once_and_rearms_new_output() {
        let start = Instant::now();
        let mut activity = Activity::default();
        assert!(activity.output(start, false, true));
        assert!(!activity.output(start, false, true));
        assert!(!activity.idle(start + Duration::from_secs(1), true));
        assert!(activity.idle(start + IDLE_AFTER, true));
        assert!(!activity.idle(start + IDLE_AFTER * 2, true));
        assert!(activity.output(start + IDLE_AFTER * 2, false, true));
    }

    #[test]
    fn output_in_the_visible_tab_is_not_new_and_never_becomes_idle() {
        let start = Instant::now();
        let mut activity = Activity::default();
        assert!(!activity.output(start, true, true));
        assert!(!activity.idle(start + IDLE_AFTER, true));
    }
}

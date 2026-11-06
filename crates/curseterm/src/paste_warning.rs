use serde::{Deserialize, Serialize};

// The three Edit › Paste Special warning switches, with iTerm2's defaults:
// noSyncDoNotWarnBeforeMultilinePaste NO, promptForPasteWhenNotAtPrompt NO and
// noSyncDoNotWarnBeforePastingOneLineEndingInNewlineAtShellPrompt NO.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PasteWarnings {
    pub multi_line: bool,
    pub only_at_prompt: bool,
    pub one_line_at_prompt: bool,
}

impl Default for PasteWarnings {
    fn default() -> Self {
        Self {
            multi_line: true,
            only_at_prompt: true,
            one_line_at_prompt: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Warning {
    Lines(usize),
    LinesAtPrompt(usize),
    OneLine,
    OneLineAtPrompt,
}

// iTermPasteHelper.m -maybeWarnAboutMultiLinePaste:
pub fn warning(text: &str, at_prompt: bool, warnings: PasteWarnings) -> Option<Warning> {
    if !warnings.multi_line || !text.contains(['\r', '\n']) {
        return None;
    }
    let lines = text.replace("\r\n", "\n").replace('\r', "\n");
    let count = lines
        .strip_suffix('\n')
        .unwrap_or(&lines)
        .split('\n')
        .count();
    match (count > 1, at_prompt) {
        (true, true) => Some(Warning::LinesAtPrompt(count)),
        (true, false) => (!warnings.only_at_prompt).then_some(Warning::Lines(count)),
        (false, true) => warnings
            .one_line_at_prompt
            .then_some(Warning::OneLineAtPrompt),
        (false, false) => (!warnings.only_at_prompt).then_some(Warning::OneLine),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn by_default_only_a_paste_with_a_newline_at_the_shell_prompt_is_questioned() {
        let defaults = PasteWarnings::default();
        assert_eq!(warning("ls", true, defaults), None);
        assert_eq!(
            warning("a\nb\r\nc\n", true, defaults),
            Some(Warning::LinesAtPrompt(3))
        );
        assert_eq!(warning("a\nb", false, defaults), None);
        assert_eq!(
            warning("make\n", true, defaults),
            Some(Warning::OneLineAtPrompt)
        );
        let everywhere = PasteWarnings {
            only_at_prompt: false,
            ..defaults
        };
        assert_eq!(warning("a\nb", false, everywhere), Some(Warning::Lines(2)));
        assert_eq!(warning("make\n", false, everywhere), Some(Warning::OneLine));
        let off = PasteWarnings {
            multi_line: false,
            ..defaults
        };
        assert_eq!(warning("a\nb", true, off), None);
    }
}

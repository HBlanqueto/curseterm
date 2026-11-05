use alacritty_terminal::term::TermMode;
use freya_core::prelude::{Modifiers, MouseButton};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pointer {
    Press(MouseButton),
    Release(MouseButton),
    Drag(MouseButton),
    Motion,
    WheelUp,
    WheelDown,
}

pub fn reported(mode: TermMode, held: Modifiers) -> bool {
    mode.intersects(TermMode::MOUSE_MODE) && !held.contains(Modifiers::SHIFT)
}

pub fn report(
    pointer: Pointer,
    (column, line): (usize, usize),
    held: Modifiers,
    mode: TermMode,
) -> Option<Vec<u8>> {
    let button = |button| match button {
        MouseButton::Left => Some(0),
        MouseButton::Middle => Some(1),
        MouseButton::Right => Some(2),
        _ => None,
    };
    let motion = mode.intersects(TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION);
    let (code, released) = match pointer {
        Pointer::Press(pressed) => (button(pressed)?, false),
        Pointer::Release(pressed) => (button(pressed)?, true),
        Pointer::Drag(pressed) if motion => (button(pressed)? + 32, false),
        Pointer::Motion if mode.contains(TermMode::MOUSE_MOTION) => (35, false),
        Pointer::WheelUp => (64, false),
        Pointer::WheelDown => (65, false),
        _ => return None,
    };
    let modifiers = 8 * u8::from(held.contains(Modifiers::ALT))
        + 16 * u8::from(held.contains(Modifiers::CONTROL));
    if mode.contains(TermMode::SGR_MOUSE) {
        let end = if released { 'm' } else { 'M' };
        let code = code + modifiers;
        return Some(format!("\x1b[<{code};{};{}{end}", column + 1, line + 1).into_bytes());
    }
    let code = if released { 3 } else { code } + modifiers;
    let x = u8::try_from(column + 33).ok()?;
    let y = u8::try_from(line + 33).ok()?;
    Some(vec![0x1b, b'[', b'M', code + 32, x, y])
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLICKS: TermMode = TermMode::MOUSE_REPORT_CLICK;

    #[test]
    fn sgr_reports_press_and_release_with_the_button_and_one_based_cells() {
        let mode = CLICKS | TermMode::SGR_MOUSE;
        let press = report(
            Pointer::Press(MouseButton::Left),
            (4, 1),
            Modifiers::empty(),
            mode,
        );
        let release = report(
            Pointer::Release(MouseButton::Left),
            (4, 1),
            Modifiers::empty(),
            mode,
        );
        assert_eq!(press, Some(b"\x1b[<0;5;2M".to_vec()));
        assert_eq!(release, Some(b"\x1b[<0;5;2m".to_vec()));
    }

    #[test]
    fn the_classic_format_releases_as_button_three_and_skips_far_cells() {
        let press = report(
            Pointer::Press(MouseButton::Right),
            (0, 0),
            Modifiers::CONTROL,
            CLICKS,
        );
        let release = report(
            Pointer::Release(MouseButton::Right),
            (0, 0),
            Modifiers::empty(),
            CLICKS,
        );
        assert_eq!(press, Some(vec![0x1b, b'[', b'M', 32 + 2 + 16, 33, 33]));
        assert_eq!(release, Some(vec![0x1b, b'[', b'M', 32 + 3, 33, 33]));
        assert_eq!(
            report(
                Pointer::Press(MouseButton::Left),
                (300, 0),
                Modifiers::empty(),
                CLICKS
            ),
            None
        );
    }

    #[test]
    fn drags_need_mode_1002_motion_needs_1003_and_shift_lets_the_mouse_select() {
        let drag = Pointer::Drag(MouseButton::Left);
        assert_eq!(report(drag, (0, 0), Modifiers::empty(), CLICKS), None);
        assert!(
            report(
                drag,
                (0, 0),
                Modifiers::empty(),
                CLICKS | TermMode::MOUSE_DRAG
            )
            .is_some()
        );
        assert_eq!(
            report(
                Pointer::Motion,
                (0, 0),
                Modifiers::empty(),
                TermMode::MOUSE_DRAG
            ),
            None
        );
        assert!(reported(CLICKS, Modifiers::empty()));
        assert!(!reported(CLICKS, Modifiers::SHIFT));
    }
}

use std::cell::RefCell;
use std::collections::HashMap;

use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::color::Colors;
use alacritty_terminal::term::{RenderableContent, RenderableCursor, Term};
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor, Rgb};
use barrier_kit_next::canvas::CanvasContext;
use freya_engine::prelude::{
    Color4f, Font, FontEdging, FontMgr, FontStyle, Paint, PaintStyle, Rect as SkRect, Typeface,
};

use crate::config::Scheme;
use crate::session::Listener;

pub struct Fonts {
    family: String,
    size: f32,
    faces: [Font; 4],
    fallback: RefCell<HashMap<(char, usize), Option<Font>>>,
    pub cell_width: f32,
    pub cell_height: f32,
    ascent: f32,
}

impl PartialEq for Fonts {
    fn eq(&self, other: &Self) -> bool {
        self.family == other.family && self.size == other.size
    }
}

const STYLES: [fn() -> FontStyle; 4] = [
    FontStyle::normal,
    FontStyle::bold,
    FontStyle::italic,
    FontStyle::bold_italic,
];

impl Fonts {
    pub fn new(family: &str, size: f32) -> Self {
        let manager = FontMgr::default();
        let faces = STYLES.map(|style| font(typeface(&manager, family, style()), size));
        let (_, metrics) = faces[0].metrics();
        let ascent = -metrics.ascent;
        let (advance, _) = faces[0].measure_str("M", None);
        Self {
            family: family.to_string(),
            size,
            faces,
            fallback: RefCell::new(HashMap::new()),
            cell_width: advance,
            cell_height: (ascent + metrics.descent + metrics.leading).ceil(),
            ascent,
        }
    }

    fn face(&self, c: char, style: usize) -> Font {
        let face = &self.faces[style];
        if c.is_ascii() || face.unichar_to_glyph(c as i32) != 0 {
            return face.clone();
        }
        self.fallback
            .borrow_mut()
            .entry((c, style))
            .or_insert_with(|| {
                FontMgr::default()
                    .match_family_style_character(&self.family, STYLES[style](), &[], c as i32)
                    .map(|typeface| font(typeface, self.size))
            })
            .clone()
            .unwrap_or_else(|| face.clone())
    }
}

fn typeface(manager: &FontMgr, family: &str, style: FontStyle) -> Typeface {
    manager
        .match_family_style(family, style)
        .or_else(|| manager.legacy_make_typeface(None, style))
        .expect("the font manager has no fonts")
}

fn font(typeface: Typeface, size: f32) -> Font {
    let mut font = Font::from_typeface(typeface, size);
    font.set_edging(FontEdging::AntiAlias);
    font.set_subpixel(true);
    font
}

pub struct Palette<'a> {
    pub scheme: &'a Scheme,
    pub colors: &'a Colors,
}

impl Palette<'_> {
    pub fn resolve(&self, color: Color) -> Rgb {
        match color {
            Color::Spec(rgb) => rgb,
            Color::Named(named) => self.colors[named].unwrap_or_else(|| self.named(named)),
            Color::Indexed(index) => {
                self.colors[usize::from(index)].unwrap_or_else(|| indexed(self.scheme, index))
            }
        }
    }

    fn named(&self, named: NamedColor) -> Rgb {
        let scheme = self.scheme;
        match named {
            NamedColor::Foreground | NamedColor::BrightForeground => scheme.foreground,
            NamedColor::Background => scheme.background,
            NamedColor::Cursor => scheme.cursor,
            NamedColor::DimForeground => dim(scheme.foreground),
            other if (other as usize) < 16 => scheme.ansi[other as usize],
            other => dim(scheme.ansi[other as usize - NamedColor::DimBlack as usize]),
        }
    }
}

pub fn indexed(scheme: &Scheme, index: u8) -> Rgb {
    match index {
        0..16 => scheme.ansi[usize::from(index)],
        16..232 => {
            let cube = index - 16;
            let level = |value: u8| if value == 0 { 0 } else { 55 + 40 * value };
            Rgb {
                r: level(cube / 36),
                g: level(cube / 6 % 6),
                b: level(cube % 6),
            }
        }
        _ => {
            let gray = 8 + 10 * (index - 232);
            Rgb {
                r: gray,
                g: gray,
                b: gray,
            }
        }
    }
}

fn dim(rgb: Rgb) -> Rgb {
    let scale = |value: u8| (u16::from(value) * 2 / 3) as u8;
    Rgb {
        r: scale(rgb.r),
        g: scale(rgb.g),
        b: scale(rgb.b),
    }
}

fn paint(rgb: Rgb) -> Paint {
    let mut paint = Paint::default();
    paint.set_color4f(
        Color4f::new(
            f32::from(rgb.r) / 255.0,
            f32::from(rgb.g) / 255.0,
            f32::from(rgb.b) / 255.0,
            1.0,
        ),
        None,
    );
    paint
}

fn bold_bright(color: Color, flags: Flags) -> Color {
    match color {
        Color::Named(named) if flags.contains(Flags::BOLD) && (named as usize) < 8 => {
            Color::Named(named.to_bright())
        }
        other => other,
    }
}

pub fn draw(
    ctx: &mut CanvasContext,
    term: &Term<Listener>,
    fonts: &Fonts,
    scheme: &Scheme,
    focused: bool,
) {
    let RenderableContent {
        display_iter,
        selection,
        cursor,
        display_offset,
        colors,
        ..
    } = term.renderable_content();
    let palette = Palette { scheme, colors };
    let background = palette.resolve(Color::Named(NamedColor::Background));
    let mut fill = paint(background);
    fill.set_anti_alias(false);
    ctx.canvas
        .draw_rect(SkRect::new(0.0, 0.0, ctx.width, ctx.height), &fill);
    let offset = display_offset as i32;
    let frame = Frame {
        palette,
        fonts,
        background,
    };
    for indexed in display_iter {
        let selected = selection.is_some_and(|range| range.contains(indexed.point));
        let line = indexed.point.line.0 + offset;
        draw_cell(
            ctx,
            &frame,
            indexed.cell,
            indexed.point.column.0,
            line,
            selected,
        );
    }
    draw_cursor(ctx, term, cursor, offset, &frame.palette, fonts, focused);
}

struct Frame<'a> {
    palette: Palette<'a>,
    fonts: &'a Fonts,
    background: Rgb,
}

fn draw_cell(
    ctx: &mut CanvasContext,
    frame: &Frame<'_>,
    cell: &Cell,
    column: usize,
    line: i32,
    selected: bool,
) {
    let Frame {
        palette,
        fonts,
        background,
    } = frame;
    let background = *background;
    if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
        return;
    }
    let x = column as f32 * fonts.cell_width;
    let y = line as f32 * fonts.cell_height;
    let columns = if cell.flags.contains(Flags::WIDE_CHAR) {
        2.0
    } else {
        1.0
    };
    let mut fg = palette.resolve(bold_bright(cell.fg, cell.flags));
    let mut bg = palette.resolve(cell.bg);
    if cell.flags.contains(Flags::INVERSE) {
        std::mem::swap(&mut fg, &mut bg);
    }
    if cell.flags.contains(Flags::DIM) {
        fg = dim(fg);
    }
    if selected {
        bg = palette.scheme.selection;
    }
    let area = SkRect::new(x, y, x + columns * fonts.cell_width, y + fonts.cell_height);
    if bg != background {
        let mut fill = paint(bg);
        fill.set_anti_alias(false);
        ctx.canvas.draw_rect(area, &fill);
    }
    if cell.flags.contains(Flags::HIDDEN) {
        return;
    }
    let ink = paint(fg);
    if cell.c != ' ' {
        draw_char(ctx, fonts, cell.c, cell.flags, x, y, &ink);
    }
    if cell.flags.intersects(Flags::ALL_UNDERLINES) {
        let line = SkRect::new(area.left, area.bottom - 2.0, area.right, area.bottom - 1.0);
        ctx.canvas.draw_rect(line, &ink);
    }
    if cell.flags.contains(Flags::STRIKEOUT) {
        let middle = area.top + fonts.cell_height / 2.0;
        let line = SkRect::new(area.left, middle, area.right, middle + 1.0);
        ctx.canvas.draw_rect(line, &ink);
    }
}

fn draw_char(
    ctx: &mut CanvasContext,
    fonts: &Fonts,
    c: char,
    flags: Flags,
    x: f32,
    y: f32,
    ink: &Paint,
) {
    let style =
        usize::from(flags.contains(Flags::BOLD)) + 2 * usize::from(flags.contains(Flags::ITALIC));
    let mut buffer = [0; 4];
    ctx.canvas.draw_str(
        c.encode_utf8(&mut buffer),
        (x, y + fonts.ascent),
        &fonts.face(c, style),
        ink,
    );
}

fn draw_cursor(
    ctx: &mut CanvasContext,
    term: &Term<Listener>,
    cursor: RenderableCursor,
    offset: i32,
    palette: &Palette<'_>,
    fonts: &Fonts,
    focused: bool,
) {
    let line = cursor.point.line.0 + offset;
    if cursor.shape == CursorShape::Hidden || line < 0 {
        return;
    }
    let x = cursor.point.column.0 as f32 * fonts.cell_width;
    let y = line as f32 * fonts.cell_height;
    let cell = &term.grid()[cursor.point];
    let columns = if cell.flags.contains(Flags::WIDE_CHAR) {
        2.0
    } else {
        1.0
    };
    let area = SkRect::new(x, y, x + columns * fonts.cell_width, y + fonts.cell_height);
    let color = palette.resolve(Color::Named(NamedColor::Cursor));
    let mut ink = paint(color);
    match cursor.shape {
        _ if !focused => {
            ink.set_style(PaintStyle::Stroke);
            ink.set_stroke_width(1.0);
            ctx.canvas.draw_rect(area.with_inset((0.5, 0.5)), &ink);
        }
        CursorShape::Beam => {
            ctx.canvas
                .draw_rect(SkRect::new(x, y, x + 2.0, area.bottom), &ink);
        }
        CursorShape::Underline => {
            ctx.canvas.draw_rect(
                SkRect::new(x, area.bottom - 2.0, area.right, area.bottom),
                &ink,
            );
        }
        _ => {
            ctx.canvas.draw_rect(area, &ink);
            if cell.c != ' ' {
                draw_char(
                    ctx,
                    fonts,
                    cell.c,
                    cell.flags,
                    x,
                    y,
                    &paint(palette.scheme.cursor_text),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SCHEMES;

    #[test]
    fn the_256_colour_cube_and_grey_ramp_follow_xterm() {
        let scheme = &SCHEMES[0];
        assert_eq!(indexed(scheme, 1), scheme.ansi[1]);
        assert_eq!(indexed(scheme, 16), Rgb { r: 0, g: 0, b: 0 });
        assert_eq!(indexed(scheme, 196), Rgb { r: 255, g: 0, b: 0 });
        assert_eq!(
            indexed(scheme, 231),
            Rgb {
                r: 255,
                g: 255,
                b: 255
            }
        );
        assert_eq!(indexed(scheme, 232), Rgb { r: 8, g: 8, b: 8 });
        assert_eq!(
            indexed(scheme, 255),
            Rgb {
                r: 238,
                g: 238,
                b: 238
            }
        );
    }
}

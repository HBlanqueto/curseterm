use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::time::Duration;

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::color::Colors;
use alacritty_terminal::term::search::Match;
use alacritty_terminal::term::{RenderableContent, RenderableCursor, Term};
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor, Rgb};
use barrier_kit_next::canvas::CanvasContext;
use barrier_kit_next::tokens::TERMINAL;
use freya_engine::prelude::{
    ClipOp, Color4f, Font, FontEdging, FontMgr, FontStyle, Paint, PaintStyle, PathBuilder, RRect,
    Rect as SkRect, Typeface,
};
use vow_design::{DURATION_FAST_MS, DURATION_SLOW_MS};

use crate::clock::local_time;
use crate::config::Scheme;
use crate::io::{Extras, absolute_line};
use crate::session::Listener;

const TIMESTAMP_COLUMNS: usize = "00:00:00".len();

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

pub struct Overlay<'a> {
    pub extras: &'a Extras,
    pub matches: &'a [Match],
    pub inset: f32,
    pub broadcast: Option<Rgb>,
    pub badge: Option<&'a str>,
    pub opacity: f32,
}

pub fn draw(
    ctx: &mut CanvasContext,
    term: &Term<Listener>,
    fonts: &Fonts,
    scheme: &Scheme,
    focused: bool,
    overlay: &Overlay<'_>,
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
    fill.set_alpha_f(overlay.opacity);
    ctx.canvas
        .draw_rect(SkRect::new(0.0, 0.0, ctx.width, ctx.height), &fill);
    if let Some(tint) = overlay.broadcast {
        draw_broadcast(ctx, tint);
    }
    if display_offset > 0 {
        draw_scroll_indicator(ctx, term, display_offset, scheme);
    }
    if let Some(badge) = overlay.badge {
        draw_badge(ctx, badge, fonts, scheme);
    }
    ctx.canvas.save();
    ctx.canvas.translate((overlay.inset, overlay.inset));
    let offset = display_offset as i32;
    let top = term.history_size() as i64 - i64::from(offset);
    draw_highlights(ctx, overlay.extras, top, fonts, scheme);
    draw_matches(ctx, overlay.matches, offset, term.columns(), fonts, scheme);
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
    let spotlight = (
        overlay.inset + (cursor.point.column.0 as f32 + 0.5) * fonts.cell_width,
        overlay.inset + ((cursor.point.line.0 + offset) as f32 + 0.5) * fonts.cell_height,
    );
    draw_cursor(ctx, term, cursor, offset, &frame.palette, fonts, focused);
    draw_images(ctx, overlay.extras, top, fonts);
    draw_marks(ctx, overlay, top, term.screen_lines() as i64, fonts, scheme);
    if overlay.extras.show_timestamps {
        let last = absolute_line(term) - top;
        draw_timestamps(
            ctx,
            overlay.extras,
            top,
            last,
            term.columns(),
            fonts,
            scheme,
        );
    }
    ctx.canvas.restore();
    let ringing = overlay
        .extras
        .bell
        .is_some_and(|rang| rang.elapsed() < Duration::from_millis(DURATION_FAST_MS));
    if ringing {
        let mut flash = paint(scheme.foreground);
        flash.set_alpha_f(TERMINAL.bell_alpha);
        ctx.canvas
            .draw_rect(SkRect::new(0.0, 0.0, ctx.width, ctx.height), &flash);
    }
    let finding_cursor = overlay
        .extras
        .cursor_spotlight
        .is_some_and(|found| found.elapsed() < Duration::from_millis(DURATION_SLOW_MS));
    if finding_cursor {
        draw_spotlight(ctx, spotlight, scheme);
    }
}

fn draw_spotlight(ctx: &mut CanvasContext, (x, y): (f32, f32), scheme: &Scheme) {
    let radius = TERMINAL.spotlight_radius;
    let circle = RRect::new_oval(SkRect::new(x - radius, y - radius, x + radius, y + radius));
    let mut dim = paint(scheme.background);
    dim.set_alpha_f(TERMINAL.spotlight_alpha);
    ctx.canvas.save();
    ctx.canvas.clip_rrect(circle, ClipOp::Difference, true);
    ctx.canvas
        .draw_rect(SkRect::new(0.0, 0.0, ctx.width, ctx.height), &dim);
    ctx.canvas.restore();
}

fn draw_matches(
    ctx: &mut CanvasContext,
    matches: &[Match],
    offset: i32,
    columns: usize,
    fonts: &Fonts,
    scheme: &Scheme,
) {
    let ink = highlight_ink(scheme);
    for found in matches {
        let (start, end) = (*found.start(), *found.end());
        for line in start.line.0..=end.line.0 {
            let first = if line == start.line.0 {
                start.column.0
            } else {
                0
            };
            let last = if line == end.line.0 {
                end.column.0
            } else {
                columns - 1
            };
            let y = (line + offset) as f32 * fonts.cell_height;
            fill_cells(ctx, &ink, y, first..=last, fonts);
        }
    }
}

fn highlight_ink(scheme: &Scheme) -> Paint {
    let mut ink = paint(scheme.ansi[usize::from(NamedColor::Yellow as u8)]);
    ink.set_alpha_f(TERMINAL.highlight_alpha);
    ink
}

fn fill_cells(
    ctx: &mut CanvasContext,
    ink: &Paint,
    y: f32,
    columns: RangeInclusive<usize>,
    fonts: &Fonts,
) {
    let left = *columns.start() as f32 * fonts.cell_width;
    let right = (*columns.end() + 1) as f32 * fonts.cell_width;
    ctx.canvas
        .draw_rect(SkRect::new(left, y, right, y + fonts.cell_height), ink);
}

fn draw_highlights(
    ctx: &mut CanvasContext,
    extras: &Extras,
    top: i64,
    fonts: &Fonts,
    scheme: &Scheme,
) {
    let ink = highlight_ink(scheme);
    for highlight in &extras.highlights {
        let y = (highlight.line - top) as f32 * fonts.cell_height;
        if y + fonts.cell_height < 0.0 || y > ctx.height {
            continue;
        }
        fill_cells(ctx, &ink, y, highlight.columns.clone(), fonts);
    }
}

fn draw_images(ctx: &mut CanvasContext, extras: &Extras, top: i64, fonts: &Fonts) {
    for placed in &extras.images {
        let y = (placed.line - top) as f32 * fonts.cell_height;
        if y + placed.height < 0.0 || y > ctx.height {
            continue;
        }
        let x = placed.column as f32 * fonts.cell_width;
        let area = SkRect::new(x, y, x + placed.width, y + placed.height);
        ctx.canvas
            .draw_image_rect(&placed.image, None, area, &Paint::default());
    }
}

fn draw_marks(
    ctx: &mut CanvasContext,
    overlay: &Overlay<'_>,
    top: i64,
    lines: i64,
    fonts: &Fonts,
    scheme: &Scheme,
) {
    let mut ink = paint(scheme.ansi[usize::from(NamedColor::Blue as u8)]);
    ink.set_anti_alias(true);
    let size = overlay.inset;
    for mark in &overlay.extras.marks {
        let line = mark - top;
        if !(0..lines).contains(&line) {
            continue;
        }
        let middle = line as f32 * fonts.cell_height + fonts.cell_height / 2.0;
        let mut path = PathBuilder::new();
        path.move_to((-size, middle - size / 2.0));
        path.line_to((0.0, middle));
        path.line_to((-size, middle + size / 2.0));
        path.close();
        ctx.canvas.draw_path(&path.detach(), &ink);
    }
}

fn draw_timestamps(
    ctx: &mut CanvasContext,
    extras: &Extras,
    top: i64,
    last: i64,
    columns: usize,
    fonts: &Fonts,
    scheme: &Scheme,
) {
    let width = TIMESTAMP_COLUMNS as f32 * fonts.cell_width;
    let x = columns as f32 * fonts.cell_width - width;
    let mut fill = paint(scheme.background);
    fill.set_anti_alias(false);
    let ink = paint(dim(scheme.foreground));
    for row in 0..=last {
        let Some(at) = extras.stamp_of(top + row) else {
            continue;
        };
        let y = row as f32 * fonts.cell_height;
        ctx.canvas.draw_rect(
            SkRect::new(x - fonts.cell_width, y, x + width, y + fonts.cell_height),
            &fill,
        );
        ctx.canvas.draw_str(
            local_time(at),
            (x, y + fonts.ascent),
            &fonts.face('0', 0),
            &ink,
        );
    }
}

fn draw_scroll_indicator(
    ctx: &mut CanvasContext,
    term: &Term<Listener>,
    offset: usize,
    scheme: &Scheme,
) {
    let total = term.total_lines() as f32;
    let screen = term.screen_lines() as f32;
    let above = (term.history_size() - offset) as f32;
    let height = (ctx.height * screen / total).max(TERMINAL.scrollbar_width * 2.0);
    let top = (ctx.height - height) * above / (total - screen).max(1.0);
    let width = TERMINAL.scrollbar_width;
    let thumb = SkRect::new(
        ctx.width - width * 2.0,
        top,
        ctx.width - width,
        top + height,
    );
    let mut ink = paint(scheme.foreground);
    ink.set_alpha_f(TERMINAL.scrollbar_alpha);
    ctx.canvas
        .draw_rrect(RRect::new_rect_xy(thumb, width / 2.0, width / 2.0), &ink);
}

fn draw_badge(ctx: &mut CanvasContext, badge: &str, fonts: &Fonts, scheme: &Scheme) {
    let Some(font) = fonts
        .face('A', 1)
        .with_size(fonts.size * TERMINAL.badge_scale)
    else {
        return;
    };
    let (width, _) = font.measure_str(badge, None);
    let margin = fonts.cell_width * 2.0;
    let mut ink = paint(scheme.foreground);
    ink.set_alpha_f(TERMINAL.badge_alpha);
    ctx.canvas.draw_str(
        badge,
        (ctx.width - width - margin, margin + font.size()),
        &font,
        &ink,
    );
}

fn draw_broadcast(ctx: &mut CanvasContext, tint: Rgb) {
    let mut stripe = paint(tint);
    stripe.set_alpha_f(TERMINAL.broadcast_alpha);
    stripe.set_stroke_width(TERMINAL.broadcast_stripe);
    stripe.set_style(PaintStyle::Stroke);
    let step = TERMINAL.broadcast_stripe * 3.0;
    let mut x = -ctx.height;
    while x < ctx.width {
        ctx.canvas
            .draw_line((x, ctx.height), (x + ctx.height, 0.0), &stripe);
        x += step;
    }
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

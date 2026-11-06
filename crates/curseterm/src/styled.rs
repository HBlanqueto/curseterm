use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::vte::ansi::Rgb;

use crate::config::Scheme;
use crate::grid;
use crate::session::Session;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Styled {
    pub c: char,
    pub fg: Rgb,
    pub bg: Rgb,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

// BK-035: only patch 0011 can offer text/html.
#[cfg(feature = "kit-parcheado")]
pub fn html(lines: &[Vec<Styled>], background: Rgb, family: &str) -> String {
    let mut out = format!(
        "<pre style=\"font-family:'{family}',monospace;background:{};margin:0\">",
        hex(background)
    );
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        for run in line.chunk_by(same_style) {
            let first = run[0];
            out.push_str(&format!(
                "<span style=\"color:{};background:{}{}{}{}\">",
                hex(first.fg),
                hex(first.bg),
                if first.bold { ";font-weight:bold" } else { "" },
                if first.italic {
                    ";font-style:italic"
                } else {
                    ""
                },
                if first.underline {
                    ";text-decoration:underline"
                } else {
                    ""
                },
            ));
            for cell in run {
                match cell.c {
                    '&' => out.push_str("&amp;"),
                    '<' => out.push_str("&lt;"),
                    '>' => out.push_str("&gt;"),
                    c => out.push(c),
                }
            }
            out.push_str("</span>");
        }
    }
    out.push_str("</pre>");
    out
}

// PTYTextView -copyWithControlSequences:: the text with the SGR that draws it.
pub fn control_sequences(lines: &[Vec<Styled>]) -> String {
    let mut out = String::new();
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        for run in line.chunk_by(same_style) {
            let first = run[0];
            let (fg, bg) = (first.fg, first.bg);
            out.push_str(&format!(
                "\x1b[0;38;2;{};{};{};48;2;{};{};{}{}{}{}m",
                fg.r,
                fg.g,
                fg.b,
                bg.r,
                bg.g,
                bg.b,
                if first.bold { ";1" } else { "" },
                if first.italic { ";3" } else { "" },
                if first.underline { ";4" } else { "" },
            ));
            out.extend(run.iter().map(|cell| cell.c));
        }
        out.push_str("\x1b[0m");
    }
    out
}

pub fn selection(session: &Session, scheme: &Scheme) -> Option<Vec<Vec<Styled>>> {
    let term = session.term.lock();
    let range = term.selection.as_ref()?.to_range(&term)?;
    let palette = grid::Palette {
        scheme,
        colors: term.colors(),
    };
    let last = term.columns() - 1;
    let lines = (range.start.line.0..=range.end.line.0)
        .map(|line| {
            let first = if range.is_block || line == range.start.line.0 {
                range.start.column.0
            } else {
                0
            };
            let end = if range.is_block || line == range.end.line.0 {
                range.end.column.0
            } else {
                last
            };
            let row = &term.grid()[Line(line)];
            let mut cells: Vec<Styled> = (first..=end.min(last))
                .map(|column| &row[Column(column)])
                .filter(|cell| !cell.flags.contains(Flags::WIDE_CHAR_SPACER))
                .map(|cell| {
                    let (fg, bg) = palette.cell_colors(cell);
                    Styled {
                        c: cell.c,
                        fg,
                        bg,
                        bold: cell.flags.contains(Flags::BOLD),
                        italic: cell.flags.contains(Flags::ITALIC),
                        underline: cell.flags.intersects(Flags::ALL_UNDERLINES),
                    }
                })
                .collect();
            while cells.last().is_some_and(|cell| cell.c == ' ') {
                cells.pop();
            }
            cells
        })
        .collect();
    Some(lines)
}

fn same_style(a: &Styled, b: &Styled) -> bool {
    (a.fg, a.bg, a.bold, a.italic, a.underline) == (b.fg, b.bg, b.bold, b.italic, b.underline)
}

#[cfg(feature = "kit-parcheado")]
fn hex(rgb: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb.r, rgb.g, rgb.b)
}

#[cfg(all(test, feature = "kit-parcheado"))]
mod tests {
    use super::*;

    #[test]
    fn runs_of_one_style_share_a_span_and_markup_is_escaped() {
        let red = Rgb { r: 255, g: 0, b: 0 };
        let black = Rgb { r: 0, g: 0, b: 0 };
        let cell = |c, fg, bold| Styled {
            c,
            fg,
            bg: black,
            bold,
            italic: false,
            underline: false,
        };
        let lines = vec![
            vec![
                cell('a', red, true),
                cell('<', red, true),
                cell('b', black, false),
            ],
            vec![cell('&', black, false)],
        ];
        assert_eq!(
            html(&lines, black, "Mono"),
            "<pre style=\"font-family:'Mono',monospace;background:#000000;margin:0\">\
             <span style=\"color:#ff0000;background:#000000;font-weight:bold\">a&lt;</span>\
             <span style=\"color:#000000;background:#000000\">b</span>\n\
             <span style=\"color:#000000;background:#000000\">&amp;</span></pre>"
        );
    }
}

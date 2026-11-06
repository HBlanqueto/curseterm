use std::ops::Range;

use alacritty_terminal::grid::{Dimensions, Row};
use alacritty_terminal::index::Line;
use alacritty_terminal::term::Term;
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::vte::ansi::{Color, NamedColor};
use serde_json::Value;

use crate::io::Extras;
use crate::session::Listener;

fn line(term: &Term<Listener>, absolute: i64) -> Line {
    Line((absolute - term.history_size() as i64) as i32)
}

// VT100ScreenMutableState.m -replaceRange:withLines:…: the rows of `lines` become `rows`; what
// comes after keeps its place and what came before moves up or down. Returns the new rows' lines.
pub fn replace_lines(
    term: &mut Term<Listener>,
    extras: &mut Extras,
    lines: Range<i64>,
    rows: Vec<Row<Cell>>,
    history_limit: usize,
) -> Range<i64> {
    let (mut start, mut end) = (lines.start, lines.end);
    let added = rows.len() as i64;
    let removed = end - start;
    if added > removed {
        let grown = (added - removed) as usize;
        let before = term.history_size();
        let screen = term.screen_lines() as i32;
        term.grid_mut()
            .scroll_up::<Color>(&(Line(0)..Line(screen)), grown);
        let lost = (before + grown - term.history_size()) as i64;
        if lost > 0 {
            extras.forget_lines(0..lost);
            start = (start - lost).max(0);
            end = (end - lost).max(start);
        }
        let total = (term.history_size() + term.screen_lines()) as i64;
        let shift = grown as i64;
        for absolute in (end + shift..total).rev() {
            let row = term.grid()[line(term, absolute - shift)].clone();
            let at = line(term, absolute);
            term.grid_mut()[at] = row;
        }
        extras.remap_lines(|line| match line {
            _ if (start..end).contains(&line) => None,
            _ if line >= end => Some(line + shift),
            _ => Some(line),
        });
    } else if added < removed {
        let shift = removed - added;
        for absolute in (shift..start + shift).rev() {
            let row = term.grid()[line(term, absolute - shift)].clone();
            let at = line(term, absolute);
            term.grid_mut()[at] = row;
        }
        let columns = term.columns();
        for absolute in 0..shift.min(start + shift) {
            let at = line(term, absolute);
            term.grid_mut()[at] = Row::new(columns);
        }
        // The oldest rows are now blank; dropping them shrinks the buffer as iTerm2's does, and
        // what history cannot absorb comes off the screen, the cursor rising with its line.
        let dropped = (shift as usize).min(term.history_size());
        let kept = term.history_size() - dropped;
        term.grid_mut().update_history(kept);
        term.grid_mut().update_history(history_limit);
        let rest = shift - dropped as i64;
        if rest > 0 {
            let total = (term.history_size() + term.screen_lines()) as i64;
            for absolute in rest..total {
                let row = term.grid()[line(term, absolute)].clone();
                let at = line(term, absolute - rest);
                term.grid_mut()[at] = row;
            }
            for absolute in total - rest..total {
                let at = line(term, absolute);
                term.grid_mut()[at] = Row::new(columns);
            }
            let cursor = &mut term.grid_mut().cursor.point.line;
            *cursor = Line((cursor.0 - rest as i32).max(0));
        }
        extras.remap_lines(|line| match line {
            _ if (start..end).contains(&line) => None,
            _ if line < start => Some(line),
            _ => Some(line - shift),
        });
    } else {
        extras.remap_lines(|line| (!(start..end).contains(&line)).then_some(line));
    }
    for (offset, row) in rows.into_iter().enumerate() {
        let at = line(term, start + offset as i64);
        term.grid_mut()[at] = row;
    }
    term.selection = None;
    start..start + added
}

pub fn rows_of(term: &Term<Listener>, lines: Range<i64>) -> Vec<Row<Cell>> {
    lines
        .map(|absolute| term.grid()[line(term, absolute)].clone())
        .collect()
}

fn text_of(row: &Row<Cell>) -> String {
    (0..row.len())
        .map(|column| &row[alacritty_terminal::index::Column(column)])
        .filter(|cell| !cell.flags.contains(Flags::WIDE_CHAR_SPACER))
        .map(|cell| cell.c)
        .collect::<String>()
        .trim_end()
        .to_string()
}

// PTYTextView.swift fold placeholders: default colours, faint and italic.
fn placeholder(text: &str, columns: usize) -> Row<Cell> {
    let mut cells: Vec<Cell> = text
        .chars()
        .take(columns)
        .map(|c| Cell {
            c,
            flags: Flags::DIM | Flags::ITALIC,
            ..Cell::default()
        })
        .collect();
    cells.resize(
        columns,
        Cell {
            flags: Flags::DIM | Flags::ITALIC,
            ..Cell::default()
        },
    );
    Row::from_vec(cells, columns)
}

// PTYTextView.swift -firstEllipsisLast:…: as much of the first and last lines as fits round
// “ …N lines… ”, trimming the longer side first.
pub fn fold_placeholder(rows: &[Row<Cell>], columns: usize, middle: &str) -> Row<Cell> {
    let first = rows.first().map(text_of).unwrap_or_default();
    let last = rows.last().map(text_of).unwrap_or_default();
    let last = last.trim_start().to_string();
    let middle = if middle.chars().count() + 10 > columns {
        "…".to_string()
    } else {
        middle.to_string()
    };
    let room = columns.saturating_sub(middle.chars().count());
    let (mut first_length, mut last_length) = (first.chars().count(), last.chars().count());
    while first_length + last_length > room {
        if first_length >= last_length {
            first_length -= 1;
        } else {
            last_length -= 1;
        }
    }
    let text: String = first
        .chars()
        .take(first_length)
        .chain(middle.chars())
        .chain(last.chars().take(last_length))
        .collect();
    placeholder(&text, columns)
}

pub fn fold_all_placeholder(text: &str, columns: usize) -> Row<Cell> {
    placeholder(text, columns)
}

#[derive(Clone, Copy)]
enum Part {
    Punctuation,
    Key,
    Text,
    Number,
    Bool,
}

// JSONPrettyPrinter.swift: four-space indent; strings green, numbers red, booleans yellow,
// punctuation and null inverted.
pub fn json_rows(text: &str, columns: usize) -> Option<Vec<Row<Cell>>> {
    let value: Value = serde_json::from_str(text.trim()).ok()?;
    if !value.is_object() && !value.is_array() {
        return None;
    }
    let mut lines = vec![Vec::new()];
    append(&value, 0, &mut lines);
    Some(
        lines
            .iter()
            .flat_map(|parts| wrapped(parts, columns))
            .collect(),
    )
}

fn push(lines: &mut [Vec<(String, Part)>], text: impl Into<String>, part: Part) {
    if let Some(line) = lines.last_mut() {
        line.push((text.into(), part));
    }
}

fn append(value: &Value, level: usize, lines: &mut Vec<Vec<(String, Part)>>) {
    let indent = "    ".repeat(level);
    match value {
        Value::Object(map) => {
            push(lines, "{", Part::Punctuation);
            for (index, (key, item)) in map.iter().enumerate() {
                lines.push(Vec::new());
                push(lines, format!("{indent}    "), Part::Punctuation);
                push(lines, Value::String(key.clone()).to_string(), Part::Key);
                push(lines, ": ", Part::Punctuation);
                append(item, level + 1, lines);
                if index + 1 < map.len() {
                    push(lines, ",", Part::Punctuation);
                }
            }
            lines.push(Vec::new());
            push(lines, format!("{indent}}}"), Part::Punctuation);
        }
        Value::Array(items) => {
            push(lines, "[", Part::Punctuation);
            for (index, item) in items.iter().enumerate() {
                lines.push(Vec::new());
                push(lines, format!("{indent}    "), Part::Punctuation);
                append(item, level + 1, lines);
                if index + 1 < items.len() {
                    push(lines, ",", Part::Punctuation);
                }
            }
            lines.push(Vec::new());
            push(lines, format!("{indent}]"), Part::Punctuation);
        }
        Value::String(_) => push(lines, value.to_string(), Part::Text),
        Value::Number(_) => push(lines, value.to_string(), Part::Number),
        Value::Bool(_) => push(lines, value.to_string(), Part::Bool),
        Value::Null => push(lines, "null", Part::Punctuation),
    }
}

fn styled(c: char, part: Part) -> Cell {
    let color = |named| Color::Named(named);
    match part {
        Part::Punctuation if c != ' ' => Cell {
            c,
            flags: Flags::INVERSE,
            ..Cell::default()
        },
        Part::Text => Cell {
            c,
            fg: color(NamedColor::Green),
            ..Cell::default()
        },
        Part::Number => Cell {
            c,
            fg: color(NamedColor::Red),
            ..Cell::default()
        },
        Part::Bool => Cell {
            c,
            fg: color(NamedColor::Yellow),
            ..Cell::default()
        },
        _ => Cell {
            c,
            ..Cell::default()
        },
    }
}

// ScreenCharArray -split:maxWidth:: a long line wraps onto the next row.
fn wrapped(parts: &[(String, Part)], columns: usize) -> Vec<Row<Cell>> {
    let cells: Vec<Cell> = parts
        .iter()
        .flat_map(|(text, part)| text.chars().map(|c| styled(c, *part)))
        .collect();
    let chunks: Vec<&[Cell]> = if cells.is_empty() {
        vec![&[]]
    } else {
        cells.chunks(columns).collect()
    };
    let count = chunks.len();
    chunks
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| {
            let mut row = chunk.to_vec();
            row.resize(columns, Cell::default());
            if index + 1 < count {
                row[columns - 1].flags.insert(Flags::WRAPLINE);
            }
            Row::from_vec(row, columns)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(rows: &[Row<Cell>]) -> Vec<String> {
        rows.iter().map(text_of).collect()
    }

    #[test]
    fn json_is_indented_by_four_and_wraps_at_the_width() {
        let rows = json_rows(r#"{"a":[1,true,null],"b":"x"}"#, 40).unwrap();
        assert_eq!(
            text(&rows),
            [
                "{",
                "    \"a\": [",
                "        1,",
                "        true,",
                "        null",
                "    ],",
                "    \"b\": \"x\"",
                "}"
            ]
        );
        assert!(json_rows("42", 40).is_none());
        assert_eq!(json_rows("[\"abcdefgh\"]", 6).unwrap().len(), 5);
    }

    #[test]
    fn a_fold_keeps_the_ends_of_its_first_and_last_lines() {
        let row = |text: &str| placeholder(text, 30);
        let folded = fold_placeholder(
            &[row("first line"), row("x"), row("  last line")],
            30,
            " …1 lines… ",
        );
        assert_eq!(text_of(&folded), "first line …1 lines… last line");
        let short = fold_placeholder(&[row("first line"), row("last line")], 24, " …0 lines… ");
        assert_eq!(text_of(&short).chars().count(), 24);
    }
}

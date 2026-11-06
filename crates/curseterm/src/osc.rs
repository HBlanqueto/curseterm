//! iTerm2's OSC 7, 9, 133 and 1337 split out of the PTY stream (ADR 0005).

const ESC: u8 = 0x1b;
const BEL: u8 = 0x07;
const WANTED: [&[u8]; 4] = [b"7", b"9", b"133", b"1337"];
const MAX_ID: usize = 4;
const MAX_PAYLOAD: usize = 64 << 20;

#[derive(Debug, PartialEq, Eq)]
pub enum Piece {
    Bytes(Vec<u8>),
    Osc(Vec<u8>),
}

#[derive(Default)]
enum State {
    #[default]
    Text,
    Escape,
    Id,
    Payload,
    PayloadEscape,
}

#[derive(Default)]
pub struct Splitter {
    state: State,
    held: Vec<u8>,
}

impl Splitter {
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Piece> {
        let mut pieces = Vec::new();
        let mut text = Vec::with_capacity(bytes.len());
        for &byte in bytes {
            match self.state {
                State::Text => self.text(byte, &mut text),
                State::Escape if byte == b']' => {
                    self.held.clear();
                    self.state = State::Id;
                }
                State::Escape => {
                    text.push(ESC);
                    self.text(byte, &mut text);
                }
                State::Id if byte.is_ascii_digit() && self.held.len() < MAX_ID => {
                    self.held.push(byte);
                }
                State::Id if byte == b';' && WANTED.contains(&self.held.as_slice()) => {
                    self.held.push(byte);
                    self.state = State::Payload;
                }
                State::Id => {
                    text.extend_from_slice(&[ESC, b']']);
                    text.append(&mut self.held);
                    self.text(byte, &mut text);
                }
                State::Payload if byte == BEL => self.finish(&mut pieces, &mut text),
                State::Payload if byte == ESC => self.state = State::PayloadEscape,
                State::Payload => self.hold(byte),
                State::PayloadEscape if byte == b'\\' => self.finish(&mut pieces, &mut text),
                State::PayloadEscape => {
                    self.hold(ESC);
                    self.hold(byte);
                    self.state = State::Payload;
                }
            }
        }
        if !text.is_empty() {
            pieces.push(Piece::Bytes(text));
        }
        pieces
    }

    fn text(&mut self, byte: u8, text: &mut Vec<u8>) {
        if byte == ESC {
            self.state = State::Escape;
        } else {
            text.push(byte);
            self.state = State::Text;
        }
    }

    fn hold(&mut self, byte: u8) {
        if self.held.len() < MAX_PAYLOAD {
            self.held.push(byte);
        }
    }

    fn finish(&mut self, pieces: &mut Vec<Piece>, text: &mut Vec<u8>) {
        if !text.is_empty() {
            pieces.push(Piece::Bytes(std::mem::take(text)));
        }
        pieces.push(Piece::Osc(std::mem::take(&mut self.held)));
        self.state = State::Text;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(text: &str) -> Piece {
        Piece::Bytes(text.as_bytes().to_vec())
    }

    fn osc(text: &str) -> Piece {
        Piece::Osc(text.as_bytes().to_vec())
    }

    #[test]
    fn iterm_sequences_leave_the_stream_in_place_and_the_rest_passes_untouched() {
        let mut splitter = Splitter::default();
        let pieces =
            splitter.feed(b"a\x1b[31mb\x1b]133;A\x07c\x1b]0;title\x07d\x1b]7;file://h/tmp\x1b\\e");
        assert_eq!(
            pieces,
            vec![
                bytes("a\x1b[31mb"),
                osc("133;A"),
                bytes("c\x1b]0;title\x07d"),
                osc("7;file://h/tmp"),
                bytes("e"),
            ]
        );
    }

    #[test]
    fn a_sequence_split_across_reads_is_reassembled() {
        let mut splitter = Splitter::default();
        assert_eq!(splitter.feed(b"x\x1b"), vec![bytes("x")]);
        assert_eq!(splitter.feed(b"]13"), vec![]);
        assert_eq!(splitter.feed(b"37;CurrentDir=/srv"), vec![]);
        assert_eq!(
            splitter.feed(b"\x07y"),
            vec![osc("1337;CurrentDir=/srv"), bytes("y")]
        );
    }

    #[test]
    fn an_escape_that_is_not_an_osc_is_kept_even_at_the_end_of_a_read() {
        let mut splitter = Splitter::default();
        assert_eq!(splitter.feed(b"\x1b"), vec![]);
        assert_eq!(splitter.feed(b"[H\x1b\x1b"), vec![bytes("\x1b[H\x1b")]);
        assert_eq!(splitter.feed(b"[2J"), vec![bytes("\x1b[2J")]);
    }
}

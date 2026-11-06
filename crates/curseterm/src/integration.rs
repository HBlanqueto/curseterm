use std::path::PathBuf;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

#[derive(Debug, PartialEq)]
pub enum Report {
    Directory(PathBuf),
    Mark,
    OutputStart,
    CommandEnd(Option<i32>),
    Image(InlineImage),
    Notification(Notification),
}

#[derive(Debug, Default, PartialEq)]
pub struct Notification {
    pub rich: bool,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub message: String,
}

#[derive(Debug, PartialEq)]
pub struct InlineImage {
    pub data: Vec<u8>,
    pub width: Dimension,
    pub height: Dimension,
    pub preserve_aspect_ratio: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Dimension {
    Auto,
    Cells(u32),
    Pixels(u32),
    Percent(u32),
}

pub fn parse(payload: &[u8]) -> Option<Report> {
    let text = std::str::from_utf8(payload).ok();
    match payload {
        [b'7', b';', url @ ..] => file_url(std::str::from_utf8(url).ok()?).map(Report::Directory),
        [b'9', b';', ..] => growl(text?.strip_prefix("9;")?),
        [b'1', b'3', b'3', b';', b'A', ..] => Some(Report::Mark),
        [b'1', b'3', b'3', b';', b'C', ..] => Some(Report::OutputStart),
        [b'1', b'3', b'3', b';', b'D', rest @ ..] => {
            Some(Report::CommandEnd(std::str::from_utf8(rest).ok().and_then(
                |rest| rest.strip_prefix(';')?.split(';').next()?.parse().ok(),
            )))
        }
        _ => {
            let iterm = text?.strip_prefix("1337;")?;
            if let Some(path) = iterm.strip_prefix("CurrentDir=") {
                return Some(Report::Directory(path.into()));
            }
            if iterm == "SetMark" {
                return Some(Report::Mark);
            }
            if let Some(fields) = iterm.strip_prefix("Notification=") {
                return Some(Report::Notification(rich_notification(fields)));
            }
            iterm
                .strip_prefix("File=")
                .and_then(inline_image)
                .map(Report::Image)
        }
    }
}

// VT100ScreenMutableState+TerminalDelegate.m -terminalPostUserNotification: a numeric first
// field is ConEmu's progress report, not a message.
fn growl(message: &str) -> Option<Report> {
    let first = message.split(';').next().unwrap_or_default();
    let progress = !first.is_empty() && first.bytes().all(|byte| byte.is_ascii_digit());
    (!progress).then(|| {
        Report::Notification(Notification {
            message: message.to_string(),
            ..Notification::default()
        })
    })
}

// VT100Terminal.m `Notification` key: base64 fields joined by `;`.
fn rich_notification(fields: &str) -> Notification {
    let mut notification = Notification {
        rich: true,
        ..Notification::default()
    };
    for (key, encoded) in fields.split(';').filter_map(|field| field.split_once('=')) {
        let Some(value) = STANDARD
            .decode(encoded.trim())
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
        else {
            continue;
        };
        match key {
            "title" => notification.title = Some(value),
            "subtitle" => notification.subtitle = Some(value),
            "message" => notification.message = value,
            _ => {}
        }
    }
    notification
}

fn file_url(url: &str) -> Option<PathBuf> {
    let rest = url.strip_prefix("file://")?;
    let path = &rest[rest.find('/')?..];
    Some(percent_decode(path).into())
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let hex = bytes
            .get(index + 1..index + 3)
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        match (bytes[index], hex) {
            (b'%', Some(value)) => {
                out.push(value);
                index += 3;
            }
            (byte, _) => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn inline_image(arguments: &str) -> Option<InlineImage> {
    let (options, data) = arguments.split_once(':')?;
    let mut inline = false;
    let mut image = InlineImage {
        data: Vec::new(),
        width: Dimension::Auto,
        height: Dimension::Auto,
        preserve_aspect_ratio: true,
    };
    for option in options.split(';') {
        match option.split_once('=') {
            Some(("inline", value)) => inline = value == "1",
            Some(("width", value)) => image.width = dimension(value),
            Some(("height", value)) => image.height = dimension(value),
            Some(("preserveAspectRatio", value)) => image.preserve_aspect_ratio = value != "0",
            _ => {}
        }
    }
    let clean: String = data.chars().filter(|c| !c.is_whitespace()).collect();
    image.data = STANDARD.decode(clean).ok()?;
    inline.then_some(image)
}

fn dimension(value: &str) -> Dimension {
    let number = |digits: &str| digits.parse().ok();
    if let Some(pixels) = value.strip_suffix("px").and_then(number) {
        Dimension::Pixels(pixels)
    } else if let Some(percent) = value.strip_suffix('%').and_then(number) {
        Dimension::Percent(percent)
    } else {
        number(value).map_or(Dimension::Auto, Dimension::Cells)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_current_directory_comes_from_osc_7_or_iterm_current_dir() {
        assert_eq!(
            parse(b"7;file://vm/home/user/Mis%20cosas"),
            Some(Report::Directory("/home/user/Mis cosas".into()))
        );
        assert_eq!(
            parse(b"1337;CurrentDir=/srv/www"),
            Some(Report::Directory("/srv/www".into()))
        );
    }

    #[test]
    fn a_prompt_start_or_set_mark_is_a_mark_and_the_command_input_is_ignored() {
        assert_eq!(parse(b"133;A"), Some(Report::Mark));
        assert_eq!(parse(b"1337;SetMark"), Some(Report::Mark));
        assert_eq!(parse(b"133;C"), Some(Report::OutputStart));
        assert_eq!(parse(b"133;D;0"), Some(Report::CommandEnd(Some(0))));
        assert_eq!(parse(b"133;D;127"), Some(Report::CommandEnd(Some(127))));
        assert_eq!(parse(b"133;D"), Some(Report::CommandEnd(None)));
        assert_eq!(parse(b"133;B"), None);
    }

    #[test]
    fn an_inline_file_carries_its_decoded_bytes_and_size_and_a_download_is_ignored() {
        let Some(Report::Image(image)) =
            parse(b"1337;File=name=eA==;width=10;height=50%;inline=1:aGk=")
        else {
            panic!("an inline image");
        };
        assert_eq!(image.data, b"hi");
        assert_eq!(image.width, Dimension::Cells(10));
        assert_eq!(image.height, Dimension::Percent(50));
        assert_eq!(parse(b"1337;File=inline=0:aGk="), None);
    }

    #[test]
    fn osc_9_posts_its_text_unless_it_is_a_progress_report_and_1337_notification_is_base64() {
        assert_eq!(
            parse(b"9;make: done"),
            Some(Report::Notification(Notification {
                message: "make: done".into(),
                ..Notification::default()
            }))
        );
        assert_eq!(parse(b"9;4;1;50"), None);
        assert_eq!(
            parse(b"1337;Notification=title=SGVjaG8=;message=TGlzdG8="),
            Some(Report::Notification(Notification {
                rich: true,
                title: Some("Hecho".into()),
                subtitle: None,
                message: "Listo".into(),
            }))
        );
    }
}

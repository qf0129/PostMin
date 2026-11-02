use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{Color32, FontFamily, FontId};
use postmin_core::format::BodyFormat;

use crate::theme;

fn format(font_size: f32, color: Color32) -> TextFormat {
    TextFormat {
        font_id: FontId::new(font_size, FontFamily::Monospace),
        color,
        ..Default::default()
    }
}

fn push(job: &mut LayoutJob, text: &str, color: Color32, font_size: f32) {
    if !text.is_empty() {
        job.append(text, 0.0, format(font_size, color));
    }
}

fn push_italic(job: &mut LayoutJob, text: &str, color: Color32, font_size: f32) {
    if !text.is_empty() {
        let mut text_format = format(font_size, color);
        text_format.italics = true;
        job.append(text, 0.0, text_format);
    }
}

pub fn layout(text: &str, kind: BodyFormat, font_size: f32, wrap_width: f32) -> LayoutJob {
    let font_size = theme::font_size(font_size);
    let mut job = match kind {
        BodyFormat::Json => json_job(text, font_size),
        BodyFormat::Xml => xml_job(text, font_size),
        BodyFormat::Text => {
            let mut job = LayoutJob::default();
            push(&mut job, text, theme::text(), font_size);
            job
        }
    };
    job.wrap.max_width = wrap_width;
    job
}

fn json_job(text: &str, font_size: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut index = 0;
    while index < len {
        let byte = bytes[index];
        match byte {
            b'"' => {
                let start = index;
                index += 1;
                while index < len {
                    match bytes[index] {
                        b'\\' => index = (index + 2).min(len),
                        b'"' => {
                            index += 1;
                            break;
                        }
                        _ => index += 1,
                    }
                }
                let mut cursor = index;
                while cursor < len && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                let is_key = cursor < len && bytes[cursor] == b':';
                let color = if is_key {
                    theme::cyan()
                } else {
                    theme::green()
                };
                push(&mut job, &text[start..index], color, font_size);
            }
            b'0'..=b'9' | b'-' | b'+' => {
                let start = index;
                index += 1;
                while index < len
                    && matches!(
                        bytes[index],
                        b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-' | b'x' | b'X'
                            | b'a'..=b'f' | b'A'..=b'F'
                    )
                {
                    index += 1;
                }
                push(&mut job, &text[start..index], theme::yellow(), font_size);
            }
            b't' | b'f' | b'n' => {
                let remaining = &text[index..];
                if remaining.starts_with("true") {
                    push(&mut job, "true", theme::purple(), font_size);
                    index += 4;
                } else if remaining.starts_with("false") {
                    push(&mut job, "false", theme::purple(), font_size);
                    index += 5;
                } else if remaining.starts_with("null") {
                    push(&mut job, "null", theme::red(), font_size);
                    index += 4;
                } else {
                    let start = index;
                    index += text[index..]
                        .chars()
                        .next()
                        .map(char::len_utf8)
                        .unwrap_or(1);
                    push(&mut job, &text[start..index], theme::text(), font_size);
                }
            }
            b'{' | b'}' | b'[' | b']' | b':' | b',' => {
                push(&mut job, &text[index..index + 1], theme::muted(), font_size);
                index += 1;
            }
            b'\n' | b'\r' | b'\t' | b' ' => {
                let start = index;
                index += 1;
                while index < len && matches!(bytes[index], b'\n' | b'\r' | b'\t' | b' ') {
                    index += 1;
                }
                push(&mut job, &text[start..index], theme::text(), font_size);
            }
            _ => {
                let start = index;
                index += text[index..]
                    .chars()
                    .next()
                    .map(char::len_utf8)
                    .unwrap_or(1);
                push(&mut job, &text[start..index], theme::text(), font_size);
            }
        }
    }
    job
}

fn find_until(text: &str, from: usize, needle: &str) -> usize {
    text[from..]
        .find(needle)
        .map(|position| from + position + needle.len())
        .unwrap_or(text.len())
}

fn tag_end(text: &str, from: usize) -> usize {
    let bytes = text.as_bytes();
    let mut index = from;
    let mut quote: Option<u8> = None;
    while index < bytes.len() {
        match bytes[index] {
            b'"' | b'\'' => {
                if quote == Some(bytes[index]) {
                    quote = None;
                } else if quote.is_none() {
                    quote = Some(bytes[index]);
                }
            }
            b'>' if quote.is_none() => return index + 1,
            _ => {}
        }
        index += 1;
    }
    bytes.len()
}

fn xml_job(text: &str, font_size: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut index = 0;

    while index < len {
        if bytes[index] != b'<' {
            let end = text[index..]
                .find('<')
                .map(|position| index + position)
                .unwrap_or(len);
            push(&mut job, &text[index..end], theme::text(), font_size);
            index = end;
            continue;
        }

        let remaining = &text[index..];
        if remaining.starts_with("<!--") {
            let end = find_until(text, index + 4, "-->");
            push_italic(&mut job, &text[index..end], theme::muted(), font_size);
            index = end;
            continue;
        }
        if remaining.starts_with("<![CDATA[") {
            let end = find_until(text, index + 9, "]]>");
            push(&mut job, &text[index..end], theme::yellow(), font_size);
            index = end;
            continue;
        }
        if remaining.starts_with("<?") {
            let end = find_until(text, index + 2, "?>");
            push(&mut job, &text[index..end], theme::purple(), font_size);
            index = end;
            continue;
        }
        if remaining.starts_with("<!") {
            let end = find_until(text, index + 2, ">");
            push(&mut job, &text[index..end], theme::purple(), font_size);
            index = end;
            continue;
        }

        let end = tag_end(text, index + 1);
        if end <= index + 1 {
            push(&mut job, "<", theme::muted(), font_size);
            index += 1;
            continue;
        }
        let inner_end = end.saturating_sub(1);
        let mut cursor = index + 1;
        push(&mut job, "<", theme::muted(), font_size);
        if cursor < inner_end && bytes[cursor] == b'/' {
            push(&mut job, "/", theme::muted(), font_size);
            cursor += 1;
        }
        let name_start = cursor;
        while cursor < inner_end
            && !bytes[cursor].is_ascii_whitespace()
            && bytes[cursor] != b'/'
            && bytes[cursor] != b'>'
        {
            cursor += 1;
        }
        push(
            &mut job,
            &text[name_start..cursor],
            theme::accent_soft(),
            font_size,
        );

        while cursor < inner_end {
            let byte = bytes[cursor];
            if byte.is_ascii_whitespace() {
                let start = cursor;
                cursor += 1;
                while cursor < inner_end && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                push(&mut job, &text[start..cursor], theme::text(), font_size);
            } else if byte == b'/' || byte == b'>' {
                push(
                    &mut job,
                    &text[cursor..cursor + 1],
                    theme::muted(),
                    font_size,
                );
                cursor += 1;
            } else if byte == b'=' {
                push(&mut job, "=", theme::muted(), font_size);
                cursor += 1;
            } else if byte == b'"' || byte == b'\'' {
                let quote = byte;
                let start = cursor;
                cursor += 1;
                while cursor < inner_end && bytes[cursor] != quote {
                    cursor += 1;
                }
                if cursor < inner_end {
                    cursor += 1;
                }
                push(&mut job, &text[start..cursor], theme::green(), font_size);
            } else {
                let start = cursor;
                while cursor < inner_end
                    && !bytes[cursor].is_ascii_whitespace()
                    && bytes[cursor] != b'='
                    && bytes[cursor] != b'/'
                    && bytes[cursor] != b'>'
                {
                    cursor += 1;
                }
                push(&mut job, &text[start..cursor], theme::blue(), font_size);
            }
        }
        if end > index {
            push(&mut job, ">", theme::muted(), font_size);
        }
        index = end;
    }

    job
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_json_jobs() {
        let job = layout(
            r#"{"name":"postmin","count":3,"ok":true}"#,
            BodyFormat::Json,
            12.0,
            400.0,
        );
        assert!(!job.sections.is_empty());
        assert_eq!(job.wrap.max_width, 400.0);
    }

    #[test]
    fn builds_xml_jobs() {
        let job = layout(
            r#"<root id="1"><item>hi</item></root>"#,
            BodyFormat::Xml,
            12.0,
            300.0,
        );
        assert!(!job.sections.is_empty());
    }
}

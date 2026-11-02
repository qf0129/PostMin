use encoding_rs::Encoding;
use quick_xml::events::Event;
use quick_xml::{Reader, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyFormat {
    Json,
    Xml,
    Text,
}

impl BodyFormat {
    pub fn label(self) -> &'static str {
        match self {
            BodyFormat::Json => "JSON",
            BodyFormat::Xml => "XML",
            BodyFormat::Text => "TEXT",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DecodedBody {
    pub text: String,
    pub charset: Option<String>,
    pub binary: bool,
}

pub fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

pub fn detect_format(content_type: &str, body: &str) -> BodyFormat {
    let content_type = content_type.to_ascii_lowercase();
    if content_type.contains("json") {
        return BodyFormat::Json;
    }
    if content_type.contains("xml") || content_type.contains("html") {
        return BodyFormat::Xml;
    }
    let trimmed = body.trim_start();
    if (trimmed.starts_with('{') || trimmed.starts_with('['))
        && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
    {
        return BodyFormat::Json;
    }
    if trimmed.starts_with('<') {
        return BodyFormat::Xml;
    }
    BodyFormat::Text
}

pub fn pretty_json(input: &str) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|error| format!("JSON 解析失败：{error}"))?;
    serde_json::to_string_pretty(&value).map_err(|error| format!("JSON 格式化失败：{error}"))
}

pub fn pretty_xml(input: &str) -> Result<String, String> {
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(false);
    reader.config_mut().check_end_names = true;
    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
    let mut buffer = Vec::new();
    let mut depth: i64 = 0;
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Eof) => break,
            Ok(event) => {
                match &event {
                    Event::Start(_) => depth += 1,
                    Event::End(_) => {
                        depth -= 1;
                        if depth < 0 {
                            return Err("XML 解析失败：存在多余的结束标签".to_string());
                        }
                    }
                    _ => {}
                }
                writer
                    .write_event(event)
                    .map_err(|error| format!("XML 格式化失败：{error}"))?;
            }
            Err(error) => return Err(format!("XML 解析失败：{error}")),
        }
        buffer.clear();
    }
    if depth != 0 {
        return Err("XML 解析失败：标签未闭合".to_string());
    }
    String::from_utf8(writer.into_inner()).map_err(|error| format!("XML 编码错误：{error}"))
}

pub fn charset_from_content_type(content_type: &str) -> Option<String> {
    for part in content_type.split(';').skip(1) {
        if let Some((key, value)) = part.split_once('=') {
            if key.trim().eq_ignore_ascii_case("charset") {
                let value = value.trim().trim_matches('"').trim_matches('\'');
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

fn looks_binary_content_type(content_type: &str) -> bool {
    let content_type = content_type.to_ascii_lowercase();
    content_type.starts_with("image/")
        || content_type.starts_with("audio/")
        || content_type.starts_with("video/")
        || content_type.starts_with("font/")
        || content_type.contains("octet-stream")
        || content_type.contains("pdf")
        || content_type.contains("zip")
        || content_type.contains("gzip")
        || content_type.contains("protobuf")
        || content_type.contains("msgpack")
}

fn type_is_textual(content_type: &str) -> bool {
    let content_type = content_type.to_ascii_lowercase();
    content_type.is_empty()
        || content_type.starts_with("text/")
        || content_type.contains("json")
        || content_type.contains("xml")
        || content_type.contains("javascript")
        || content_type.contains("ecmascript")
        || content_type.contains("x-www-form-urlencoded")
        || content_type.contains("graphql")
}

pub fn decode_body(headers: &[(String, String)], bytes: &[u8]) -> DecodedBody {
    let content_type = header_value(headers, "content-type")
        .unwrap_or_default()
        .to_string();
    let charset = charset_from_content_type(&content_type);

    let mut binary = looks_binary_content_type(&content_type);
    if !binary && !type_is_textual(&content_type) {
        binary = bytes.iter().take(1024).any(|byte| *byte == 0);
    }
    if binary {
        return DecodedBody {
            text: String::new(),
            charset,
            binary: true,
        };
    }

    let (text, used_charset) = if let Some(label) = charset.as_deref() {
        match Encoding::for_label(label.as_bytes()) {
            Some(encoding) => {
                let (text, _, _) = encoding.decode(bytes);
                (text.into_owned(), Some(encoding.name().to_string()))
            }
            None => (String::from_utf8_lossy(bytes).into_owned(), None),
        }
    } else {
        (String::from_utf8_lossy(bytes).into_owned(), None)
    };

    let text = text.trim_start_matches('\u{feff}').to_string();
    DecodedBody {
        text,
        charset: charset.or(used_charset),
        binary: false,
    }
}

pub fn hex_dump(bytes: &[u8], max_bytes: usize) -> String {
    let mut output = String::new();
    let limit = bytes.len().min(max_bytes);
    for offset in (0..limit).step_by(16) {
        let end = (offset + 16).min(limit);
        let slice = &bytes[offset..end];
        output.push_str(&format!("{offset:08x}  "));
        for position in 0..16 {
            if let Some(byte) = slice.get(position) {
                output.push_str(&format!("{byte:02x} "));
            } else {
                output.push_str("   ");
            }
            if position == 7 {
                output.push(' ');
            }
        }
        output.push_str(" |");
        for byte in slice {
            let character = *byte as char;
            if character.is_ascii_graphic() || character == ' ' {
                output.push(character);
            } else {
                output.push('.');
            }
        }
        output.push_str("|\n");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_json_and_preserves_key_order() {
        let input = r#"{"z":1,"a":{"b":[1,2,{"c":true}]}}"#;
        let output = pretty_json(input).expect("valid json");
        assert!(output.contains("\n  \"z\": 1,"));
        assert!(output.contains("\n  \"a\": {"));
        assert!(output.find("\"z\"").unwrap() < output.find("\"a\"").unwrap());
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(pretty_json("{oops}").is_err());
    }

    #[test]
    fn formats_xml_with_indentation() {
        let input = r#"<?xml version="1.0"?><root attr="1"><item>hello</item><empty/><nested><x>1</x></nested></root>"#;
        let output = pretty_xml(input).expect("valid xml");
        assert!(output.contains("<?xml version=\"1.0\"?>"));
        assert!(output.contains("attr=\"1\""));
        assert!(output.contains("\n  <item>"));
        assert!(output.contains("<empty/>") || output.contains("<empty></empty>"));
        assert!(output.contains("\n    <x>"));
        assert!(pretty_xml("<root>").is_err());
    }

    #[test]
    fn detects_body_formats() {
        assert_eq!(
            detect_format("application/json; charset=utf-8", "{}"),
            BodyFormat::Json
        );
        assert_eq!(detect_format("", "  {\"a\": 1}"), BodyFormat::Json);
        assert_eq!(detect_format("", "<root/>"), BodyFormat::Xml);
        assert_eq!(detect_format("text/plain", "hello"), BodyFormat::Text);
        assert_eq!(detect_format("", "not json"), BodyFormat::Text);
    }

    #[test]
    fn decodes_charsets_and_binary() {
        let headers = vec![(
            "Content-Type".to_string(),
            "text/plain; charset=gbk".to_string(),
        )];
        let (encoded, _, _) = encoding_rs::GBK.encode("中文");
        let decoded = decode_body(&headers, &encoded);
        assert_eq!(decoded.text, "中文");
        assert!(!decoded.binary);

        let image_headers = vec![("Content-Type".to_string(), "image/png".to_string())];
        let decoded = decode_body(&image_headers, &[0x89, b'P', b'N', b'G']);
        assert!(decoded.binary);
    }

    #[test]
    fn extracts_charset() {
        assert_eq!(
            charset_from_content_type("text/html; Charset=\"utf-8\""),
            Some("utf-8".to_string())
        );
        assert_eq!(charset_from_content_type("application/json"), None);
    }

    #[test]
    fn hex_dump_contains_offsets() {
        let dump = hex_dump(b"hello", 16);
        assert!(dump.starts_with("00000000  "));
        assert!(dump.contains("hello"));
    }
}

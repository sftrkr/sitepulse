use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PageMeta {
    pub title: Option<String>,
    pub description: Option<String>,
    pub canonical_url: Option<String>,
}

pub fn extract_page_meta(html: &str) -> PageMeta {
    let mut reader = Reader::from_str(html);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut meta = PageMeta::default();
    let mut in_title = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                handle_start_or_empty(&e, &mut meta, &mut in_title);
            }
            Ok(Event::Empty(e)) => {
                handle_start_or_empty(&e, &mut meta, &mut in_title);
            }
            Ok(Event::End(e)) if e.name().as_ref().eq_ignore_ascii_case(b"title") => {
                in_title = false;
            }
            Ok(Event::Text(e)) if in_title && meta.title.is_none() => {
                if let Ok(text) = e.unescape() {
                    let title = normalize_whitespace(text.trim());
                    if !title.is_empty() {
                        meta.title = Some(title);
                    }
                }
            }
            Ok(Event::CData(e)) if in_title && meta.title.is_none() => {
                let title = normalize_whitespace(&String::from_utf8_lossy(e.as_ref()));
                if !title.is_empty() {
                    meta.title = Some(title);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }

        if meta.title.is_some() && meta.description.is_some() && meta.canonical_url.is_some() {
            break;
        }
        buf.clear();
    }

    meta
}

fn handle_start_or_empty(e: &BytesStart<'_>, meta: &mut PageMeta, in_title: &mut bool) {
    let name = e.name().as_ref().to_ascii_lowercase();
    if name == b"title" {
        *in_title = true;
    } else if name == b"link" && meta.canonical_url.is_none() {
        parse_link(e, meta);
    } else if name == b"meta" && meta.description.is_none() {
        parse_meta(e, meta);
    }
}

fn parse_link(e: &BytesStart<'_>, meta: &mut PageMeta) {
    let mut rel = None;
    let mut href = None;

    for attr in e.attributes().flatten() {
        let key = attr.key.as_ref().to_ascii_lowercase();
        let value = String::from_utf8_lossy(attr.value.as_ref())
            .trim()
            .to_string();
        if key == b"rel" {
            rel = Some(value);
        } else if key == b"href" {
            href = Some(value);
        }
    }

    let is_canonical = rel
        .as_deref()
        .map(|value| {
            value
                .split_ascii_whitespace()
                .any(|token| token.eq_ignore_ascii_case("canonical"))
        })
        .unwrap_or(false);

    if is_canonical {
        meta.canonical_url = href.filter(|value| !value.is_empty());
    }
}

fn parse_meta(e: &BytesStart<'_>, meta: &mut PageMeta) {
    let mut name = None;
    let mut property = None;
    let mut content = None;

    for attr in e.attributes().flatten() {
        let key = attr.key.as_ref().to_ascii_lowercase();
        let value = String::from_utf8_lossy(attr.value.as_ref())
            .trim()
            .to_string();
        if key == b"name" {
            name = Some(value);
        } else if key == b"property" {
            property = Some(value);
        } else if key == b"content" {
            content = Some(value);
        }
    }

    let is_description = name
        .as_deref()
        .map(|value| value.eq_ignore_ascii_case("description"))
        .unwrap_or(false)
        || property
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case("og:description"))
            .unwrap_or(false);

    if is_description {
        meta.description = content.filter(|value| !value.is_empty());
    }
}

fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_title_meta_description_and_canonical() {
        let meta = extract_page_meta(
            r#"<!doctype html><html><head><title> Example title </title><meta name="description" content="Example description"><link rel="canonical" href="https://example.com/canonical"></head></html>"#,
        );

        assert_eq!(meta.title.as_deref(), Some("Example title"));
        assert_eq!(meta.description.as_deref(), Some("Example description"));
        assert_eq!(
            meta.canonical_url.as_deref(),
            Some("https://example.com/canonical")
        );
    }

    #[test]
    fn extracts_self_closing_meta_and_canonical() {
        let meta = extract_page_meta(
            r#"<html><head><title>Test</title><meta name="description" content="Self closing desc" /><link href="https://example.com/self" rel="canonical" /></head></html>"#,
        );

        assert_eq!(meta.description.as_deref(), Some("Self closing desc"));
        assert_eq!(
            meta.canonical_url.as_deref(),
            Some("https://example.com/self")
        );
    }

    #[test]
    fn extracts_canonical_when_rel_has_multiple_tokens() {
        let meta = extract_page_meta(
            r#"<html><head><link rel="alternate canonical" href="https://example.com/canonical" /></head></html>"#,
        );

        assert_eq!(
            meta.canonical_url.as_deref(),
            Some("https://example.com/canonical")
        );
    }

    #[test]
    fn falls_back_to_og_description() {
        let meta = extract_page_meta(
            r#"<html><head><meta property="og:description" content="OG description" /></head></html>"#,
        );

        assert_eq!(meta.description.as_deref(), Some("OG description"));
    }

    #[test]
    fn ignores_missing_meta() {
        let meta = extract_page_meta("<html><head></head><body></body></html>");
        assert_eq!(meta, PageMeta::default());
    }
}

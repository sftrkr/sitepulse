use quick_xml::events::Event;
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
                let name = e.name().as_ref().to_ascii_lowercase();
                if name == b"title" {
                    in_title = true;
                } else if name == b"link" && meta.canonical_url.is_none() {
                    let mut is_canonical = false;
                    let mut href = None;

                    for attr in e.attributes().flatten() {
                        let key = attr.key.as_ref().to_ascii_lowercase();
                        let value = String::from_utf8_lossy(attr.value.as_ref())
                            .trim()
                            .to_string();
                        if key == b"rel" && value.eq_ignore_ascii_case("canonical") {
                            is_canonical = true;
                        } else if key == b"href" {
                            href = Some(value);
                        }
                    }

                    if is_canonical {
                        meta.canonical_url = href.filter(|value| !value.is_empty());
                    }
                } else if name == b"meta" && meta.description.is_none() {
                    let mut is_description = false;
                    let mut content = None;

                    for attr in e.attributes().flatten() {
                        let key = attr.key.as_ref().to_ascii_lowercase();
                        let value = String::from_utf8_lossy(attr.value.as_ref())
                            .trim()
                            .to_string();
                        if key == b"name" && value.eq_ignore_ascii_case("description") {
                            is_description = true;
                        } else if key == b"content" {
                            content = Some(value);
                        }
                    }

                    if is_description {
                        meta.description = content.filter(|value| !value.is_empty());
                    }
                }
            }
            Ok(Event::End(e)) if e.name().as_ref().eq_ignore_ascii_case(b"title") => {
                in_title = false;
            }
            Ok(Event::Text(e)) if in_title && meta.title.is_none() => {
                if let Ok(text) = e.unescape() {
                    let title = text.trim().to_string();
                    if !title.is_empty() {
                        meta.title = Some(title);
                    }
                }
            }
            Ok(Event::CData(e)) if in_title && meta.title.is_none() => {
                let title = String::from_utf8_lossy(e.as_ref()).trim().to_string();
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
    fn ignores_missing_meta() {
        let meta = extract_page_meta("<html><head></head><body></body></html>");
        assert_eq!(meta, PageMeta::default());
    }
}

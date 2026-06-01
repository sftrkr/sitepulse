use anyhow::{anyhow, Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::Client;
use std::collections::BTreeSet;
use std::io::Read;
use std::time::Duration;

const USER_AGENT: &str = "sitepulse/0.1 (+https://example.local)";
const MAX_DEPTH: usize = 2;
const MAX_SITEMAP_BYTES: u64 = 50 * 1024 * 1024;

pub async fn discover_urls(sitemap_url: &str, timeout_secs: u64) -> Result<Vec<String>> {
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .context("failed to build HTTP client")?;

    let mut seen_sitemaps = BTreeSet::new();
    let mut urls = BTreeSet::new();
    fetch_sitemap_recursive(&client, sitemap_url, 0, &mut seen_sitemaps, &mut urls).await?;
    Ok(urls.into_iter().collect())
}

async fn fetch_sitemap_recursive(
    client: &Client,
    sitemap_url: &str,
    depth: usize,
    seen_sitemaps: &mut BTreeSet<String>,
    urls: &mut BTreeSet<String>,
) -> Result<()> {
    if depth >= MAX_DEPTH || !seen_sitemaps.insert(sitemap_url.to_string()) {
        return Ok(());
    }

    let xml = download_xml(client, sitemap_url).await?;
    let parsed = parse_sitemap_locs(&xml)
        .with_context(|| format!("failed to parse XML from sitemap: {sitemap_url}"))?;

    if parsed.is_index {
        for child in parsed.locs {
            Box::pin(fetch_sitemap_recursive(
                client,
                &child,
                depth + 1,
                seen_sitemaps,
                urls,
            ))
            .await?;
        }
    } else {
        urls.extend(parsed.locs);
    }

    Ok(())
}

async fn download_xml(client: &Client, url: &str) -> Result<String> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to download sitemap: {url}"))?;

    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!("sitemap download failed for {url}: HTTP {status}"));
    }

    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("failed to read sitemap body: {url}"))?;

    if bytes.len() as u64 > MAX_SITEMAP_BYTES {
        return Err(anyhow!(
            "sitemap body is too large for {url}: {} bytes",
            bytes.len()
        ));
    }

    decode_sitemap_body(url, &bytes)
}

fn decode_sitemap_body(url: &str, bytes: &[u8]) -> Result<String> {
    if is_gzip(url, bytes) {
        decode_gzip(bytes).with_context(|| format!("failed to decode gzip sitemap: {url}"))
    } else {
        String::from_utf8(bytes.to_vec())
            .with_context(|| format!("sitemap is not valid UTF-8: {url}"))
    }
}

fn is_gzip(url: &str, bytes: &[u8]) -> bool {
    url.ends_with(".gz") || bytes.starts_with(&[0x1f, 0x8b])
}

fn decode_gzip(bytes: &[u8]) -> Result<String> {
    let mut decoder = flate2::read::GzDecoder::new(bytes);
    let mut output = String::new();
    decoder.read_to_string(&mut output)?;
    Ok(output)
}

#[derive(Debug, PartialEq, Eq)]
struct ParsedSitemap {
    is_index: bool,
    locs: Vec<String>,
}

fn parse_sitemap_locs(xml: &str) -> Result<ParsedSitemap> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut locs = Vec::new();
    let mut in_loc = false;
    let mut is_index = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"sitemapindex" => is_index = true,
                b"loc" => in_loc = true,
                _ => {}
            },
            Ok(Event::End(e)) if e.name().as_ref() == b"loc" => in_loc = false,
            Ok(Event::Text(e)) if in_loc => {
                let loc = e.unescape()?.trim().to_string();
                if !loc.is_empty() {
                    locs.push(loc);
                }
            }
            Ok(Event::CData(e)) if in_loc => {
                let loc = String::from_utf8_lossy(e.as_ref()).trim().to_string();
                if !loc.is_empty() {
                    locs.push(loc);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow!("XML parse error: {e}")),
            _ => {}
        }
        buf.clear();
    }

    Ok(ParsedSitemap { is_index, locs })
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    #[test]
    fn parses_urlset() {
        let xml = r#"<urlset><url><loc>https://example.com/</loc></url></urlset>"#;
        let parsed = parse_sitemap_locs(xml).unwrap();
        assert!(!parsed.is_index);
        assert_eq!(parsed.locs, vec!["https://example.com/"]);
    }

    #[test]
    fn parses_sitemapindex() {
        let xml = r#"<sitemapindex><sitemap><loc>https://example.com/a.xml</loc></sitemap></sitemapindex>"#;
        let parsed = parse_sitemap_locs(xml).unwrap();
        assert!(parsed.is_index);
        assert_eq!(parsed.locs, vec!["https://example.com/a.xml"]);
    }

    #[test]
    fn decodes_gzip_sitemap_by_extension() {
        let xml = r#"<urlset><url><loc>https://example.com/</loc></url></urlset>"#;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(xml.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();

        let decoded =
            decode_sitemap_body("https://example.com/sitemap.xml.gz", &compressed).unwrap();
        assert_eq!(decoded, xml);
    }

    #[test]
    fn decodes_gzip_sitemap_by_magic_header() {
        let xml = r#"<urlset><url><loc>https://example.com/</loc></url></urlset>"#;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(xml.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();

        let decoded = decode_sitemap_body("https://example.com/sitemap.xml", &compressed).unwrap();
        assert_eq!(decoded, xml);
    }
}

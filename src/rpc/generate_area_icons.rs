use crate::{
    db::{self, image::ImagePool},
    Result,
};
use deadpool_sqlite::Pool;
use image::ImageReader;
use serde::Serialize;
use std::io::Cursor;
use time::OffsetDateTime;

#[derive(Serialize)]
pub struct Res {
    pub areas_checked: i64,
    pub images_inserted: i64,
    pub images_identical: i64,
    pub images_failed: i64,
    pub failures: Vec<Failure>,
    pub time_s: f64,
}

#[derive(Serialize)]
pub struct Failure {
    pub area_id: i64,
    pub alias: String,
    pub url: String,
    pub reason: String,
}

pub async fn run(main_pool: &Pool, image_pool: &ImagePool) -> Result<Res> {
    let started_at = OffsetDateTime::now_utc();
    let areas = db::main::area::queries::select_with_icon_square(main_pool).await?;
    let areas_checked = areas.len() as i64;

    let mut images_inserted = 0i64;
    let mut images_identical = 0i64;
    let mut images_failed = 0i64;
    let mut failures = Vec::new();

    let client = reqwest::Client::new();

    for area in areas {
        let Some(url) = area
            .tags
            .get("icon:square")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned())
        else {
            images_failed += 1;
            failures.push(Failure {
                area_id: area.id,
                alias: area.alias(),
                url: String::new(),
                reason: "icon:square tag is not a string".into(),
            });
            continue;
        };

        let response = match client.get(&url).send().await {
            Ok(res) => res,
            Err(e) => {
                images_failed += 1;
                failures.push(Failure {
                    area_id: area.id,
                    alias: area.alias(),
                    url: url.clone(),
                    reason: format!("request failed: {e}"),
                });
                continue;
            }
        };

        if !response.status().is_success() {
            images_failed += 1;
            failures.push(Failure {
                area_id: area.id,
                alias: area.alias(),
                url: url.clone(),
                reason: format!("HTTP {}", response.status()),
            });
            continue;
        }

        let bytes = match response.bytes().await {
            Ok(b) => b.to_vec(),
            Err(e) => {
                images_failed += 1;
                failures.push(Failure {
                    area_id: area.id,
                    alias: area.alias(),
                    url: url.clone(),
                    reason: format!("failed to read body: {e}"),
                });
                continue;
            }
        };

        let dims = match actix_web::web::block({
            let bytes = bytes.clone();
            move || decode_dimensions(&bytes)
        })
        .await
        {
            Ok(Some(dims)) => dims,
            Ok(None) => {
                images_failed += 1;
                failures.push(Failure {
                    area_id: area.id,
                    alias: area.alias(),
                    url: url.clone(),
                    reason: "failed to decode image dimensions".into(),
                });
                continue;
            }
            Err(e) => {
                images_failed += 1;
                failures.push(Failure {
                    area_id: area.id,
                    alias: area.alias(),
                    url: url.clone(),
                    reason: format!("failed to decode image dimensions: {e}"),
                });
                continue;
            }
        };

        let (width, height) = (dims.0 as i64, dims.1 as i64);

        let existing =
            db::image::area::queries::select_by_area_id_and_type(area.id, "square", image_pool)
                .await?;

        let bytes_len = bytes.len() as i64;

        if let Some(existing) = &existing {
            if existing.image_data == bytes {
                images_identical += 1;
                continue;
            }
            db::image::area::queries::delete(existing.id, image_pool).await?;
        }

        db::image::area::queries::insert(
            area.id, "square", bytes, width, height, bytes_len, image_pool,
        )
        .await?;
        images_inserted += 1;
    }

    Ok(Res {
        areas_checked,
        images_inserted,
        images_identical,
        images_failed,
        failures,
        time_s: (OffsetDateTime::now_utc() - started_at).as_seconds_f64(),
    })
}

pub fn decode_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if looks_like_svg(bytes) {
        return svg_dimensions(bytes);
    }
    ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()
        .and_then(|reader| reader.into_dimensions().ok())
}

pub fn detect_ext(bytes: &[u8]) -> Option<&'static str> {
    if looks_like_svg(bytes) {
        return Some("svg");
    }
    ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()
        .and_then(|reader| reader.format())
        .and_then(|fmt| fmt.extensions_str().first().copied())
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(512)];
    let head = match std::str::from_utf8(head) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let trimmed = head.trim_start();
    trimmed.starts_with("<?xml") || trimmed.starts_with("<svg")
}

fn svg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let text = std::str::from_utf8(bytes).ok()?;
    let open = text.find("<svg")?;
    let closing = text[open..].find('>')? + open;
    let tag = &text[open..=closing];

    let width = parse_svg_length_attr(tag, "width");
    let height = parse_svg_length_attr(tag, "height");
    if let (Some(w), Some(h)) = (width, height) {
        return Some((w, h));
    }

    let viewbox = parse_svg_viewbox(tag)?;
    Some((viewbox.2, viewbox.3))
}

fn parse_svg_viewbox(tag: &str) -> Option<(f64, f64, u32, u32)> {
    let key = "viewBox=";
    let idx = tag.find(key)?;
    let after = &tag[idx + key.len()..];
    let after = after.trim_start();
    let quote = after.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let after = &after[quote.len_utf8()..];
    let end = after.find(quote)?;
    let raw = &after[..end];
    let mut parts = raw
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty());
    let min_x: f64 = parts.next()?.parse().ok()?;
    let min_y: f64 = parts.next()?.parse().ok()?;
    let width: f64 = parts.next()?.parse().ok()?;
    let height: f64 = parts.next()?.parse().ok()?;
    Some((min_x, min_y, width.round() as u32, height.round() as u32))
}

fn parse_svg_length_attr(tag: &str, attr: &str) -> Option<u32> {
    let key = format!("{attr}=");
    let idx = tag.find(&key)?;
    let after = &tag[idx + key.len()..];
    let after = after.trim_start();
    let quote = after.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let after = &after[quote.len_utf8()..];
    let end = after.find(quote)?;
    let raw = &after[..end];
    let numeric = raw
        .trim_end_matches(|c: char| !c.is_ascii_digit() && c != '.')
        .parse::<f64>()
        .ok()?;
    Some(numeric.round() as u32)
}

#[cfg(test)]
mod test {
    use super::{decode_dimensions, looks_like_svg};

    #[test]
    fn detects_svg_payload() {
        assert!(looks_like_svg(b"<?xml version=\"1.0\"?><svg></svg>"));
        assert!(looks_like_svg(b"   <svg width=\"10\" height=\"10\"></svg>"));
        assert!(!looks_like_svg(b"\x89PNG\r\n\x1a\n"));
    }

    #[test]
    fn parses_svg_width_and_height() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80"></svg>"#;
        assert_eq!(Some((120, 80)), decode_dimensions(svg));
    }

    #[test]
    fn parses_svg_width_and_height_with_units() {
        let svg = br#"<svg width="64px" height="64px"></svg>"#;
        assert_eq!(Some((64, 64)), decode_dimensions(svg));
    }

    #[test]
    fn parses_svg_width_and_height_with_single_quotes() {
        let svg = br#"<svg width='32' height='48'></svg>"#;
        assert_eq!(Some((32, 48)), decode_dimensions(svg));
    }

    #[test]
    fn parses_svg_dimensions_with_xml_prologue() {
        let svg = br#"<?xml version="1.0"?><svg width="100" height="50"></svg>"#;
        assert_eq!(Some((100, 50)), decode_dimensions(svg));
    }

    #[test]
    fn svg_without_dimensions_returns_none() {
        let svg = br#"<svg></svg>"#;
        assert_eq!(None, decode_dimensions(svg));
    }

    #[test]
    fn parses_svg_viewbox_when_width_height_missing() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 400"></svg>"#;
        assert_eq!(Some((600, 400)), decode_dimensions(svg));
    }

    #[test]
    fn svg_width_height_takes_precedence_over_viewbox() {
        let svg = br#"<svg width="120" height="80" viewBox="0 0 600 400"></svg>"#;
        assert_eq!(Some((120, 80)), decode_dimensions(svg));
    }

    #[test]
    fn parses_svg_viewbox_with_comma_separators() {
        let svg = br#"<svg viewBox="0,0,256,256"></svg>"#;
        assert_eq!(Some((256, 256)), decode_dimensions(svg));
    }

    #[test]
    fn parses_svg_viewbox_with_xml_prologue() {
        let svg = br#"<?xml version="1.0" encoding="UTF-8"?><svg viewBox="0 0 32 32"></svg>"#;
        assert_eq!(Some((32, 32)), decode_dimensions(svg));
    }
}

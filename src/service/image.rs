use crate::Result;
use base64::prelude::*;
use image::ImageFormat;
use image::ImageReader;
use image::Limits;
use std::io::Cursor;

/// Largest width/height, in pixels, accepted for a user-supplied upload.
pub const MAX_UPLOAD_DIMENSION: u32 = 20_000;
/// Largest decoded allocation, in bytes, accepted for a user-supplied upload.
pub const MAX_UPLOAD_ALLOC: u64 = 256 * 1024 * 1024;
/// Largest decoded upload accepted from a client, in bytes.
pub const MAX_UPLOAD_BYTES: usize = 10 * 1024 * 1024;

/// Raster formats accepted for user-supplied uploads. SVG is deliberately
/// excluded: serving user-supplied SVG from the API origin would be an XSS
/// vector.
const UPLOAD_FORMATS: [ImageFormat; 3] = [ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::WebP];

pub fn looks_like_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(512)];
    let Ok(head) = std::str::from_utf8(head) else {
        return false;
    };
    let trimmed = head.trim_start();
    trimmed.starts_with("<?xml") || trimmed.starts_with("<svg")
}

/// Read the pixel dimensions of a raster or SVG image without fully decoding
/// it. Returns `None` for unsupported or malformed data.
pub fn decode_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if looks_like_svg(bytes) {
        return svg_dimensions(bytes);
    }
    ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()
        .and_then(|reader| reader.into_dimensions().ok())
}

/// File extension inferred from the image bytes, or `None` when unknown.
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

/// Fully decode a user-supplied upload, enforcing raster-only formats and
/// allocation limits. Returns the detected format and decoded dimensions.
///
/// Decoding (rather than trusting header dimensions) is what bounds a
/// malicious "decompression bomb": the reader refuses to allocate more than
/// [`MAX_UPLOAD_ALLOC`] and rejects dimensions above [`MAX_UPLOAD_DIMENSION`].
pub fn decode_upload(bytes: &[u8]) -> Option<(ImageFormat, u32, u32)> {
    if looks_like_svg(bytes) {
        return None;
    }
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let format = reader.format()?;
    if !UPLOAD_FORMATS.contains(&format) {
        return None;
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_UPLOAD_DIMENSION);
    limits.max_image_height = Some(MAX_UPLOAD_DIMENSION);
    limits.max_alloc = Some(MAX_UPLOAD_ALLOC);
    reader.limits(limits);
    let image = reader.decode().ok()?;
    Some((format, image.width(), image.height()))
}

/// A base64 upload that has been decoded and validated as a storable raster.
pub struct DecodedUpload {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Decode a base64 data payload and validate it as a storable raster upload.
///
/// Returns a human-readable reason on rejection so callers can surface it as a
/// `400 invalid_input`.
pub fn decode_upload_base64(data_base64: &str) -> Result<DecodedUpload, String> {
    let bytes = BASE64_STANDARD
        .decode(data_base64.as_bytes())
        .map_err(|_| "image is not valid base64".to_string())?;
    if bytes.len() > MAX_UPLOAD_BYTES {
        return Err("image is too large".to_string());
    }
    let (_, width, height) =
        decode_upload(&bytes).ok_or_else(|| "unsupported image format".to_string())?;
    Ok(DecodedUpload {
        bytes,
        width,
        height,
    })
}

/// Decode any raster image for resizing, bounded by the reader's default
/// allocation limit.
fn decode_for_resize(bytes: &[u8]) -> Option<image::DynamicImage> {
    ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()
}

/// Return the bytes and `Content-Type` for a stored image, resized to fit
/// within `w`/`h` when requested.
///
/// The source is returned untouched when no bounds are requested, when it is an
/// SVG (raster resizing does not apply), or when the format cannot be
/// re-encoded.
pub async fn render(bytes: Vec<u8>, w: Option<u32>, h: Option<u32>) -> Result<(Vec<u8>, String)> {
    let resize_requested = w.is_some() || h.is_some();

    if !resize_requested || looks_like_svg(&bytes) {
        let content_type =
            content_type_for(&bytes).unwrap_or_else(|| "application/octet-stream".to_string());
        return Ok((bytes, content_type));
    }

    actix_web::web::block(move || -> Result<(Vec<u8>, String)> {
        let format = match ImageReader::new(Cursor::new(&bytes))
            .with_guessed_format()
            .ok()
            .and_then(|reader| reader.format())
        {
            Some(format) => format,
            None => {
                let ct = content_type_for(&bytes)
                    .unwrap_or_else(|| "application/octet-stream".to_string());
                return Ok((bytes, ct));
            }
        };

        let content_type: &'static str = match format {
            ImageFormat::Png => "image/png",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::WebP => "image/webp",
            _ => {
                let ct = content_type_for(&bytes)
                    .unwrap_or_else(|| "application/octet-stream".to_string());
                return Ok((bytes, ct));
            }
        };

        let Some(img) = decode_for_resize(&bytes) else {
            let ct =
                content_type_for(&bytes).unwrap_or_else(|| "application/octet-stream".to_string());
            return Ok((bytes, ct));
        };
        let (src_w, src_h) = (img.width(), img.height());
        let (target_w, target_h) = fit_dimensions(src_w, src_h, w, h);

        if target_w == src_w && target_h == src_h {
            return Ok((bytes, content_type.to_string()));
        }

        let resized_img = img.resize(target_w, target_h, image::imageops::FilterType::Triangle);
        let mut out: Vec<u8> = Vec::new();
        match format {
            ImageFormat::Png => {
                let encoder = image::codecs::png::PngEncoder::new(&mut out);
                resized_img.write_with_encoder(encoder)?;
            }
            ImageFormat::Jpeg => {
                let encoder = image::codecs::jpeg::JpegEncoder::new(&mut out);
                resized_img.write_with_encoder(encoder)?;
            }
            ImageFormat::WebP => {
                let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut out);
                resized_img.write_with_encoder(encoder)?;
            }
            _ => unreachable!(),
        }
        Ok((out, content_type.to_string()))
    })
    .await?
}

/// Pick target dimensions that fit the source into the requested box without
/// upsizing. If only one bound is provided, the other is derived from the
/// source aspect ratio. When the source already fits, it is returned as-is.
pub fn fit_dimensions(src_w: u32, src_h: u32, w: Option<u32>, h: Option<u32>) -> (u32, u32) {
    match (w, h) {
        (None, None) => (src_w, src_h),
        (Some(mw), None) => {
            if mw >= src_w {
                (src_w, src_h)
            } else {
                (mw, src_h * mw / src_w)
            }
        }
        (None, Some(mh)) => {
            if mh >= src_h {
                (src_w, src_h)
            } else {
                (src_w * mh / src_h, mh)
            }
        }
        (Some(mw), Some(mh)) => {
            if src_w <= mw && src_h <= mh {
                return (src_w, src_h);
            }
            let ratio = (mw as f64 / src_w as f64).min(mh as f64 / src_h as f64);
            let nw = ((src_w as f64) * ratio).round() as u32;
            let nh = ((src_h as f64) * ratio).round() as u32;
            (nw.max(1), nh.max(1))
        }
    }
}

pub fn content_type_for(bytes: &[u8]) -> Option<String> {
    if looks_like_svg(bytes) {
        return Some("image/svg+xml".to_string());
    }
    let format = image::guess_format(bytes).ok()?;
    Some(
        match format {
            ImageFormat::Png => "image/png",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::WebP => "image/webp",
            ImageFormat::Bmp => "image/bmp",
            _ => "application/octet-stream",
        }
        .to_string(),
    )
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
    use super::{
        content_type_for, decode_dimensions, decode_upload, fit_dimensions, looks_like_svg,
        MAX_UPLOAD_DIMENSION,
    };

    fn encode_png(width: u32, height: u32) -> Vec<u8> {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(width, height, Rgb([255, 0, 0]));
        let mut out: Vec<u8> = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut out);
        img.write_with_encoder(encoder).unwrap();
        out
    }

    #[::core::prelude::v1::test]
    fn content_type_for_detects_png() {
        let png_bytes: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(Some("image/png".to_string()), content_type_for(&png_bytes));
    }

    #[::core::prelude::v1::test]
    fn content_type_for_detects_jpeg() {
        let jpeg_bytes: Vec<u8> = vec![0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(
            Some("image/jpeg".to_string()),
            content_type_for(&jpeg_bytes)
        );
    }

    #[::core::prelude::v1::test]
    fn content_type_for_detects_svg() {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
        assert_eq!(Some("image/svg+xml".to_string()), content_type_for(svg));
    }

    #[::core::prelude::v1::test]
    fn content_type_for_returns_none_for_unknown() {
        let bytes = b"definitely not an image";
        assert_eq!(None, content_type_for(bytes));
    }

    #[::core::prelude::v1::test]
    fn fit_dimensions_no_constraints_returns_source() {
        assert_eq!((100, 200), fit_dimensions(100, 200, None, None));
    }

    #[::core::prelude::v1::test]
    fn fit_dimensions_only_w_scales_height() {
        assert_eq!((50, 100), fit_dimensions(100, 200, Some(50), None));
    }

    #[::core::prelude::v1::test]
    fn fit_dimensions_only_h_scales_width() {
        assert_eq!((50, 100), fit_dimensions(100, 200, None, Some(100)));
    }

    #[::core::prelude::v1::test]
    fn fit_dimensions_w_not_upsizing_returns_source() {
        assert_eq!((100, 200), fit_dimensions(100, 200, Some(200), None));
    }

    #[::core::prelude::v1::test]
    fn fit_dimensions_box_fit_uses_smaller_ratio() {
        // source 200x100 fitting into 100x100 -> width-bound: ratio 0.5 -> 100x50
        assert_eq!((100, 50), fit_dimensions(200, 100, Some(100), Some(100)));
    }

    #[::core::prelude::v1::test]
    fn fit_dimensions_box_already_fits_returns_source() {
        assert_eq!((50, 50), fit_dimensions(50, 50, Some(200), Some(200)));
    }

    #[::core::prelude::v1::test]
    fn detects_svg_payload() {
        assert!(looks_like_svg(b"<?xml version=\"1.0\"?><svg></svg>"));
        assert!(looks_like_svg(b"   <svg width=\"10\" height=\"10\"></svg>"));
        assert!(!looks_like_svg(b"\x89PNG\r\n\x1a\n"));
    }

    #[::core::prelude::v1::test]
    fn parses_svg_width_and_height() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80"></svg>"#;
        assert_eq!(Some((120, 80)), decode_dimensions(svg));
    }

    #[::core::prelude::v1::test]
    fn parses_svg_width_and_height_with_units() {
        let svg = br#"<svg width="64px" height="64px"></svg>"#;
        assert_eq!(Some((64, 64)), decode_dimensions(svg));
    }

    #[::core::prelude::v1::test]
    fn parses_svg_width_and_height_with_single_quotes() {
        let svg = br#"<svg width='32' height='48'></svg>"#;
        assert_eq!(Some((32, 48)), decode_dimensions(svg));
    }

    #[::core::prelude::v1::test]
    fn parses_svg_dimensions_with_xml_prologue() {
        let svg = br#"<?xml version="1.0"?><svg width="100" height="50"></svg>"#;
        assert_eq!(Some((100, 50)), decode_dimensions(svg));
    }

    #[::core::prelude::v1::test]
    fn svg_without_dimensions_returns_none() {
        let svg = br#"<svg></svg>"#;
        assert_eq!(None, decode_dimensions(svg));
    }

    #[::core::prelude::v1::test]
    fn parses_svg_viewbox_when_width_height_missing() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 400"></svg>"#;
        assert_eq!(Some((600, 400)), decode_dimensions(svg));
    }

    #[::core::prelude::v1::test]
    fn svg_width_height_takes_precedence_over_viewbox() {
        let svg = br#"<svg width="120" height="80" viewBox="0 0 600 400"></svg>"#;
        assert_eq!(Some((120, 80)), decode_dimensions(svg));
    }

    #[::core::prelude::v1::test]
    fn parses_svg_viewbox_with_comma_separators() {
        let svg = br#"<svg viewBox="0,0,256,256"></svg>"#;
        assert_eq!(Some((256, 256)), decode_dimensions(svg));
    }

    #[::core::prelude::v1::test]
    fn parses_svg_viewbox_with_xml_prologue() {
        let svg = br#"<?xml version="1.0" encoding="UTF-8"?><svg viewBox="0 0 32 32"></svg>"#;
        assert_eq!(Some((32, 32)), decode_dimensions(svg));
    }

    #[::core::prelude::v1::test]
    fn decode_upload_accepts_png() {
        let png = encode_png(4, 2);
        let (format, width, height) = decode_upload(&png).unwrap();
        assert_eq!(image::ImageFormat::Png, format);
        assert_eq!((4, 2), (width, height));
    }

    #[::core::prelude::v1::test]
    fn decode_upload_rejects_svg() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"></svg>"#;
        assert!(decode_upload(svg).is_none());
    }

    #[::core::prelude::v1::test]
    fn decode_upload_rejects_non_image() {
        assert!(decode_upload(b"not an image").is_none());
    }

    #[::core::prelude::v1::test]
    fn decode_upload_rejects_oversized_dimensions() {
        let png = encode_png(MAX_UPLOAD_DIMENSION + 1, 1);
        assert!(decode_upload(&png).is_none());
    }
}

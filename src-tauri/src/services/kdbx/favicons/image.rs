use crate::services::kdbx::custom_icons::detect_icon_mime;
use image::imageops::FilterType;
use image::ImageFormat;
use std::io::Cursor;

pub(super) fn normalize_favicon_bytes(
    bytes: &[u8],
    content_type: Option<String>,
) -> (Vec<u8>, String) {
    if let Ok(image) = image::load_from_memory(bytes) {
        let resized = image.resize(64, 64, FilterType::Lanczos3);
        let mut output = Cursor::new(Vec::new());
        if resized.write_to(&mut output, ImageFormat::Png).is_ok() {
            return (output.into_inner(), "image/png".to_string());
        }
    }

    let mime = content_type.unwrap_or_else(|| detect_icon_mime(bytes));
    (bytes.to_vec(), mime)
}

pub(super) fn normalize_content_type(value: &str) -> String {
    value
        .split(';')
        .next()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}

pub(super) fn is_potential_image_content_type(content_type: &str) -> bool {
    content_type.starts_with("image/")
        || content_type == "application/xml"
        || content_type == "text/xml"
        || content_type == "application/octet-stream"
}

pub(super) fn has_known_image_signature(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x89, b'P', b'N', b'G'])
        || (bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF)
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || bytes.starts_with(&[0x00, 0x00, 0x01, 0x00])
        || bytes.starts_with(b"BM")
        || (bytes.starts_with(b"RIFF") && bytes.len() > 11 && &bytes[8..12] == b"WEBP")
        || bytes.starts_with(&[0x49, 0x49, 0x2A, 0x00])
        || bytes.starts_with(&[0x4D, 0x4D, 0x00, 0x2A])
}

pub(super) fn looks_like_svg(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    text.trim_start().starts_with("<svg") || text.contains("<svg")
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    fn tiny_png() -> Vec<u8> {
        let image = ImageBuffer::from_pixel(2, 2, Rgba([0_u8, 128, 255, 255]));
        let mut output = Cursor::new(Vec::new());
        image
            .write_to(&mut output, ImageFormat::Png)
            .expect("write png");
        output.into_inner()
    }

    #[test]
    fn normalize_favicon_bytes_resizes_decodable_images_to_png() {
        let original = tiny_png();
        let (normalized, mime_type) =
            normalize_favicon_bytes(&original, Some("image/png".to_string()));

        assert_eq!(mime_type, "image/png");
        assert!(normalized.starts_with(&[0x89, b'P', b'N', b'G']));
        assert!(
            image::load_from_memory(&normalized).is_ok(),
            "normalized favicon should remain a decodable image"
        );
    }

    #[test]
    fn normalize_favicon_bytes_preserves_svg_with_content_type() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg"></svg>"#;
        let (normalized, mime_type) =
            normalize_favicon_bytes(svg, Some("image/svg+xml".to_string()));

        assert_eq!(normalized, svg);
        assert_eq!(mime_type, "image/svg+xml");
    }
}

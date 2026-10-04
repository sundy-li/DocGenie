//! Read native clipboard only on an explicit paste in the document editor.
use image::ImageEncoder;
use project_store::library::{MAX_IMAGE_BYTES, MAX_IMAGE_PIXELS};

pub fn pasted_image() -> Result<Option<Vec<u8>>, &'static str> {
    let mut clipboard = arboard::Clipboard::new().map_err(|_| "无法读取剪贴板")?;
    // The platform already delivers text paste; do not replace mixed HTML/text.
    if clipboard.get_text().is_ok_and(|s| !s.is_empty()) {
        return Ok(None);
    }
    let data = match clipboard.get_image() {
        Ok(data) => data,
        Err(arboard::Error::ContentNotAvailable) => return Ok(None),
        Err(_) => return Err("无法读取剪贴板图片"),
    };
    encode_image(data.width, data.height, &data.bytes).map(Some)
}
fn encode_image(width: usize, height: usize, rgba: &[u8]) -> Result<Vec<u8>, &'static str> {
    let pixels = width.checked_mul(height).ok_or("图片过大")?;
    if pixels == 0 || pixels as u64 > MAX_IMAGE_PIXELS || pixels.checked_mul(4) != Some(rgba.len())
    {
        return Err("图片尺寸无效或超过 1600 万像素限制");
    }
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(
            rgba,
            width as u32,
            height as u32,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|_| "图片编码失败")?;
    if png.len() > MAX_IMAGE_BYTES {
        return Err("图片超过 16 MiB 限制");
    }
    Ok(png)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encoding_roundtrip_and_limits() {
        let png = encode_image(2, 1, &[255, 0, 0, 255, 0, 255, 0, 255]).unwrap();
        let decoded = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), (2, 1));
        assert!(encode_image(0, 1, &[]).is_err());
        assert!(encode_image(usize::MAX, 2, &[]).is_err());
        assert!(encode_image(20_000, 20_000, &[]).is_err());
        assert!(encode_image(2, 1, &[0; 4]).is_err());
    }
}

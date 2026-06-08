use crate::CoreResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ImageEncoding {
    Rgb8 = 0,
    Bgr8 = 1,
    Mono8 = 2,
    Rgba8 = 3,
    Bgra8 = 4,
    Unknown = 255,
}

impl ImageEncoding {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(encoding: &str) -> Self {
        match encoding {
            "rgb8" => Self::Rgb8,
            "bgr8" => Self::Bgr8,
            "mono8" => Self::Mono8,
            "8UC1" => Self::Mono8,
            "rgba8" => Self::Rgba8,
            "bgra8" => Self::Bgra8,
            _ => Self::Unknown,
        }
    }
}

pub fn convert_image_to_rgba(cdr_payload: &[u8]) -> CoreResult<Vec<u8>> {
    if cdr_payload.len() < 4 {
        return Err(crate::CoreError::Schema("image payload too short".into()));
    }

    let data = &cdr_payload[4..];
    let mut offset = 0;

    if data.len() < 12 {
        return Err(crate::CoreError::Schema("image header too short".into()));
    }
    offset += 8;

    let frame_id_len = read_u32_le(data, offset)? as usize;
    offset += 4 + frame_id_len;
    offset = align4(offset);

    let height = read_u32_le(data, offset)?;
    offset += 4;
    let width = read_u32_le(data, offset)?;
    offset += 4;

    let encoding_len = read_u32_le(data, offset)? as usize;
    offset += 4;
    if offset + encoding_len > data.len() {
        return Err(crate::CoreError::Schema("encoding field overflows".into()));
    }
    let encoding_bytes = &data[offset..offset + encoding_len.saturating_sub(1)];
    let encoding_str = std::str::from_utf8(encoding_bytes)
        .map_err(|_| crate::CoreError::Schema("invalid encoding string".into()))?;
    let encoding = ImageEncoding::from_str(encoding_str);
    offset += encoding_len;

    if encoding == ImageEncoding::Unknown {
        return Err(crate::CoreError::Schema(format!(
            "unsupported image encoding: {encoding_str}"
        )));
    }

    offset += 1;
    offset = align4(offset);

    let step = read_u32_le(data, offset)? as usize;
    offset += 4;

    let data_len = read_u32_le(data, offset)? as usize;
    offset += 4;

    if offset + data_len > data.len() {
        return Err(crate::CoreError::Schema(
            "image data field overflows".into(),
        ));
    }
    let pixel_data = &data[offset..offset + data_len];

    let width_usize = width as usize;
    let height_usize = height as usize;
    let rgba_len = width_usize * height_usize * 4;

    let mut output = Vec::with_capacity(16 + rgba_len);
    output.extend_from_slice(&width.to_le_bytes());
    output.extend_from_slice(&height.to_le_bytes());
    output.extend_from_slice(&(width_usize as u32 * 4).to_le_bytes());
    output.push(encoding as u8);
    output.extend_from_slice(&[0u8; 3]);

    match encoding {
        ImageEncoding::Rgb8 => {
            for row in 0..height_usize {
                let row_start = row * step;
                for col in 0..width_usize {
                    let si = row_start + col * 3;
                    if si + 2 >= pixel_data.len() {
                        break;
                    }
                    output.push(pixel_data[si]);
                    output.push(pixel_data[si + 1]);
                    output.push(pixel_data[si + 2]);
                    output.push(255);
                }
            }
        }
        ImageEncoding::Bgr8 => {
            for row in 0..height_usize {
                let row_start = row * step;
                for col in 0..width_usize {
                    let si = row_start + col * 3;
                    if si + 2 >= pixel_data.len() {
                        break;
                    }
                    output.push(pixel_data[si + 2]);
                    output.push(pixel_data[si + 1]);
                    output.push(pixel_data[si]);
                    output.push(255);
                }
            }
        }
        ImageEncoding::Mono8 => {
            for row in 0..height_usize {
                let row_start = row * step;
                for col in 0..width_usize {
                    let si = row_start + col;
                    if si >= pixel_data.len() {
                        break;
                    }
                    let v = pixel_data[si];
                    output.push(v);
                    output.push(v);
                    output.push(v);
                    output.push(255);
                }
            }
        }
        ImageEncoding::Rgba8 => {
            if step == width_usize * 4 {
                let copy_len = rgba_len.min(pixel_data.len());
                output.extend_from_slice(&pixel_data[..copy_len]);
            } else {
                for row in 0..height_usize {
                    let row_start = row * step;
                    let row_end = (row_start + width_usize * 4).min(pixel_data.len());
                    output.extend_from_slice(&pixel_data[row_start..row_end]);
                }
            }
        }
        ImageEncoding::Bgra8 => {
            for row in 0..height_usize {
                let row_start = row * step;
                for col in 0..width_usize {
                    let si = row_start + col * 4;
                    if si + 3 >= pixel_data.len() {
                        break;
                    }
                    output.push(pixel_data[si + 2]);
                    output.push(pixel_data[si + 1]);
                    output.push(pixel_data[si]);
                    output.push(pixel_data[si + 3]);
                }
            }
        }
        ImageEncoding::Unknown => unreachable!(),
    }

    Ok(output)
}

#[derive(Debug)]
pub struct ImageRgba {
    pub width: u32,
    pub height: u32,
    pub encoding: ImageEncoding,
    pub rgba: Vec<u8>,
}

pub fn image_value_to_rgba(value: &rw_canonical::CanonicalValue) -> Option<ImageRgba> {
    use rw_canonical::CanonicalValue as V;
    let fields = match value {
        V::Struct(map) => map,
        _ => return None,
    };

    let height = canonical_as_u32(fields.get("height")?)?;
    let width = canonical_as_u32(fields.get("width")?)?;
    let encoding_str = match fields.get("encoding")? {
        V::String(s) => s.as_str(),
        _ => return None,
    };
    let encoding = ImageEncoding::from_str(encoding_str);
    if encoding == ImageEncoding::Unknown {
        return None;
    }
    let step = canonical_as_u32(fields.get("step")?)? as usize;
    let pixel_data: &[u8] = match fields.get("data")? {
        V::Bytes(b) => b.as_slice(),
        V::Array(items) => {
            let bytes: Option<Vec<u8>> = items
                .iter()
                .map(|item| match item {
                    V::Uint(b) if *b <= u8::MAX as u64 => Some(*b as u8),
                    V::Int(b) if *b >= 0 && *b <= u8::MAX as i64 => Some(*b as u8),
                    _ => None,
                })
                .collect();
            return convert_pixels(width, height, step, encoding, &bytes?);
        }
        _ => return None,
    };
    convert_pixels(width, height, step, encoding, pixel_data)
}

fn canonical_as_u32(value: &rw_canonical::CanonicalValue) -> Option<u32> {
    use rw_canonical::CanonicalValue as V;
    match value {
        V::Uint(v) => u32::try_from(*v).ok(),
        V::Int(v) if *v >= 0 => u32::try_from(*v).ok(),
        _ => None,
    }
}

fn convert_pixels(
    width: u32,
    height: u32,
    step: usize,
    encoding: ImageEncoding,
    pixel_data: &[u8],
) -> Option<ImageRgba> {
    let width_usize = width as usize;
    let height_usize = height as usize;
    let rgba_len = width_usize.checked_mul(height_usize)?.checked_mul(4)?;
    let mut rgba = Vec::with_capacity(rgba_len);

    match encoding {
        ImageEncoding::Rgb8 => {
            for row in 0..height_usize {
                let row_start = row * step;
                for col in 0..width_usize {
                    let si = row_start + col * 3;
                    if si + 2 >= pixel_data.len() {
                        break;
                    }
                    rgba.push(pixel_data[si]);
                    rgba.push(pixel_data[si + 1]);
                    rgba.push(pixel_data[si + 2]);
                    rgba.push(255);
                }
            }
        }
        ImageEncoding::Bgr8 => {
            for row in 0..height_usize {
                let row_start = row * step;
                for col in 0..width_usize {
                    let si = row_start + col * 3;
                    if si + 2 >= pixel_data.len() {
                        break;
                    }
                    rgba.push(pixel_data[si + 2]);
                    rgba.push(pixel_data[si + 1]);
                    rgba.push(pixel_data[si]);
                    rgba.push(255);
                }
            }
        }
        ImageEncoding::Mono8 => {
            for row in 0..height_usize {
                let row_start = row * step;
                for col in 0..width_usize {
                    let si = row_start + col;
                    if si >= pixel_data.len() {
                        break;
                    }
                    let v = pixel_data[si];
                    rgba.push(v);
                    rgba.push(v);
                    rgba.push(v);
                    rgba.push(255);
                }
            }
        }
        ImageEncoding::Rgba8 => {
            if step == width_usize * 4 {
                let copy_len = rgba_len.min(pixel_data.len());
                rgba.extend_from_slice(&pixel_data[..copy_len]);
            } else {
                for row in 0..height_usize {
                    let row_start = row * step;
                    let row_end = (row_start + width_usize * 4).min(pixel_data.len());
                    rgba.extend_from_slice(&pixel_data[row_start..row_end]);
                }
            }
        }
        ImageEncoding::Bgra8 => {
            for row in 0..height_usize {
                let row_start = row * step;
                for col in 0..width_usize {
                    let si = row_start + col * 4;
                    if si + 3 >= pixel_data.len() {
                        break;
                    }
                    rgba.push(pixel_data[si + 2]);
                    rgba.push(pixel_data[si + 1]);
                    rgba.push(pixel_data[si]);
                    rgba.push(pixel_data[si + 3]);
                }
            }
        }
        ImageEncoding::Unknown => return None,
    }

    Some(ImageRgba {
        width,
        height,
        encoding,
        rgba,
    })
}

fn read_u32_le(data: &[u8], offset: usize) -> CoreResult<u32> {
    if offset + 4 > data.len() {
        return Err(crate::CoreError::Schema(
            "unexpected end of image data".into(),
        ));
    }
    Ok(u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

fn align4(offset: usize) -> usize {
    (offset + 3) & !3
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_rgb8_image_cdr(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.push(0u8);
        buf.extend_from_slice(&[0u8; 3]);
        buf.extend_from_slice(&height.to_le_bytes());
        buf.extend_from_slice(&width.to_le_bytes());
        buf.extend_from_slice(&5u32.to_le_bytes());
        buf.extend_from_slice(b"rgb8\0");
        buf.push(0u8);
        buf.extend_from_slice(&[0u8; 2]);
        let step = width * 3;
        buf.extend_from_slice(&step.to_le_bytes());
        let data_len = pixels.len() as u32;
        buf.extend_from_slice(&data_len.to_le_bytes());
        buf.extend_from_slice(pixels);
        buf
    }

    #[test]
    fn converts_2x2_rgb8_to_rgba() {
        let pixels: Vec<u8> = vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 128, 128, 128];
        let cdr = build_rgb8_image_cdr(2, 2, &pixels);
        let output = convert_image_to_rgba(&cdr).unwrap();

        assert_eq!(&output[0..4], &2u32.to_le_bytes());
        assert_eq!(&output[4..8], &2u32.to_le_bytes());
        assert_eq!(&output[8..12], &8u32.to_le_bytes());
        assert_eq!(output[12], ImageEncoding::Rgb8 as u8);

        let rgba = &output[16..];
        assert_eq!(rgba.len(), 2 * 2 * 4);
        assert_eq!(&rgba[0..4], &[255, 0, 0, 255]);
        assert_eq!(&rgba[4..8], &[0, 255, 0, 255]);
        assert_eq!(&rgba[8..12], &[0, 0, 255, 255]);
        assert_eq!(&rgba[12..16], &[128, 128, 128, 255]);
    }

    #[test]
    fn rejects_unsupported_encoding() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.push(0u8);
        buf.extend_from_slice(&[0u8; 3]);
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&7u32.to_le_bytes());
        buf.extend_from_slice(b"yuv422\0");
        buf.push(0u8);
        buf.push(0u8);
        buf.extend_from_slice(&[0u8; 3]);
        buf.extend_from_slice(&3u32.to_le_bytes());
        buf.extend_from_slice(&3u32.to_le_bytes());
        buf.extend_from_slice(&[0u8; 3]);

        let result = convert_image_to_rgba(&buf);
        assert!(result.is_err());
    }
}

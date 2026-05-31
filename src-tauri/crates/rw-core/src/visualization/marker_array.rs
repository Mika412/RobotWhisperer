use crate::CoreResult;

const MARKER_CUBE_LIST: i32 = 6;
const MARKER_SPHERE_LIST: i32 = 7;
const MARKER_POINTS: i32 = 8;

pub fn convert_marker_array_to_positions(cdr_payload: &[u8]) -> CoreResult<Vec<u8>> {
    if cdr_payload.len() < 4 {
        return Err(crate::CoreError::Schema(
            "marker array payload too short".into(),
        ));
    }

    let data = &cdr_payload[4..];
    let mut offset = 0;

    let marker_count = read_u32_le(data, offset)? as usize;
    offset += 4;

    let mut output = Vec::with_capacity(4 + marker_count * 64);
    let mut valid_marker_count: u32 = 0;
    output.extend_from_slice(&0u32.to_le_bytes());

    for _ in 0..marker_count {
        if offset >= data.len() {
            break;
        }

        let marker_start = offset;
        let parsed = parse_marker(data, &mut offset);

        match parsed {
            Ok(marker) => {
                let is_point_cloud = marker.marker_type == MARKER_POINTS
                    || marker.marker_type == MARKER_SPHERE_LIST
                    || marker.marker_type == MARKER_CUBE_LIST;

                if is_point_cloud && !marker.points.is_empty() {
                    valid_marker_count += 1;
                    output.extend_from_slice(&marker.id.to_le_bytes());
                    output.extend_from_slice(&marker.marker_type.to_le_bytes());
                    output.extend_from_slice(&marker.action.to_le_bytes());
                    output.extend_from_slice(&marker.scale[0].to_le_bytes());
                    output.extend_from_slice(&marker.scale[1].to_le_bytes());
                    output.extend_from_slice(&marker.scale[2].to_le_bytes());

                    let point_count = marker.points.len() as u32 / 3;
                    output.extend_from_slice(&point_count.to_le_bytes());
                    for val in &marker.points {
                        output.extend_from_slice(&val.to_le_bytes());
                    }

                    let color_count = marker.colors.len() as u32 / 4;
                    output.extend_from_slice(&color_count.to_le_bytes());
                    for val in &marker.colors {
                        output.extend_from_slice(&val.to_le_bytes());
                    }
                }
            }
            Err(_) => {
                let _ = marker_start;
                break;
            }
        }
    }

    output[0..4].copy_from_slice(&valid_marker_count.to_le_bytes());

    Ok(output)
}

struct ParsedMarker {
    id: i32,
    marker_type: i32,
    action: i32,
    scale: [f32; 3],
    points: Vec<f32>,
    colors: Vec<f32>,
}

fn parse_marker(data: &[u8], offset: &mut usize) -> CoreResult<ParsedMarker> {
    *offset = align4(*offset);
    ensure_remaining(data, *offset, 8)?;
    *offset += 8;

    let frame_id_len = read_u32_le(data, *offset)? as usize;
    *offset += 4 + frame_id_len;
    *offset = align4(*offset);

    let ns_len = read_u32_le(data, *offset)? as usize;
    *offset += 4 + ns_len;
    *offset = align4(*offset);

    ensure_remaining(data, *offset, 4)?;
    let id = read_i32_le(data, *offset)?;
    *offset += 4;

    let marker_type = read_i32_le(data, *offset)?;
    *offset += 4;

    let action = read_i32_le(data, *offset)?;
    *offset += 4;

    *offset = align8(*offset);
    ensure_remaining(data, *offset, 56)?;
    *offset += 56;

    *offset = align8(*offset);
    ensure_remaining(data, *offset, 24)?;
    let scale_x = read_f64_le(data, *offset)? as f32;
    let scale_y = read_f64_le(data, *offset + 8)? as f32;
    let scale_z = read_f64_le(data, *offset + 16)? as f32;
    *offset += 24;

    ensure_remaining(data, *offset, 16)?;
    *offset += 16;

    ensure_remaining(data, *offset, 8)?;
    *offset += 8;

    ensure_remaining(data, *offset, 1)?;
    *offset += 1;
    *offset = align4(*offset);

    let point_count = read_u32_le(data, *offset)? as usize;
    *offset += 4;
    *offset = align8(*offset);
    let point_bytes = point_count * 24;
    ensure_remaining(data, *offset, point_bytes)?;
    let mut points = Vec::with_capacity(point_count * 3);
    for i in 0..point_count {
        let base = *offset + i * 24;
        points.push(read_f64_le(data, base)? as f32);
        points.push(read_f64_le(data, base + 8)? as f32);
        points.push(read_f64_le(data, base + 16)? as f32);
    }
    *offset += point_bytes;

    *offset = align4(*offset);
    let color_count = read_u32_le(data, *offset)? as usize;
    *offset += 4;
    let color_bytes = color_count * 16;
    ensure_remaining(data, *offset, color_bytes)?;
    let mut colors = Vec::with_capacity(color_count * 4);
    for i in 0..color_count {
        let base = *offset + i * 16;
        colors.push(read_f32_le(data, base)?);
        colors.push(read_f32_le(data, base + 4)?);
        colors.push(read_f32_le(data, base + 8)?);
        colors.push(read_f32_le(data, base + 12)?);
    }
    *offset += color_bytes;

    *offset = align4(*offset);
    if *offset + 4 <= data.len() {
        let text_len = read_u32_le(data, *offset)? as usize;
        *offset += 4 + text_len;
    }

    *offset = align4(*offset);
    if *offset + 4 <= data.len() {
        let mesh_len = read_u32_le(data, *offset)? as usize;
        *offset += 4 + mesh_len;
    }

    if *offset < data.len() {
        *offset += 1;
    }
    *offset = align4(*offset);

    Ok(ParsedMarker {
        id,
        marker_type,
        action,
        scale: [scale_x, scale_y, scale_z],
        points,
        colors,
    })
}

fn read_u32_le(data: &[u8], offset: usize) -> CoreResult<u32> {
    ensure_remaining(data, offset, 4)?;
    Ok(u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

fn read_i32_le(data: &[u8], offset: usize) -> CoreResult<i32> {
    ensure_remaining(data, offset, 4)?;
    Ok(i32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

fn read_f32_le(data: &[u8], offset: usize) -> CoreResult<f32> {
    ensure_remaining(data, offset, 4)?;
    Ok(f32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

fn read_f64_le(data: &[u8], offset: usize) -> CoreResult<f64> {
    ensure_remaining(data, offset, 8)?;
    Ok(f64::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ]))
}

fn ensure_remaining(data: &[u8], offset: usize, needed: usize) -> CoreResult<()> {
    if offset + needed > data.len() {
        Err(crate::CoreError::Schema("marker data truncated".into()))
    } else {
        Ok(())
    }
}

fn align4(offset: usize) -> usize {
    (offset + 3) & !3
}

fn align8(offset: usize) -> usize {
    (offset + 7) & !7
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_marker_array_produces_zero_count() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        buf.extend_from_slice(&0u32.to_le_bytes());

        let output = convert_marker_array_to_positions(&buf).unwrap();
        assert_eq!(&output[0..4], &0u32.to_le_bytes());
    }

    #[test]
    fn rejects_truncated_payload() {
        let buf = vec![0x00, 0x01];
        assert!(convert_marker_array_to_positions(&buf).is_err());
    }
}

#![deny(missing_debug_implementations)]
#![deny(unused_must_use)]

use std::sync::atomic::{AtomicBool, Ordering};

pub const WIRE_VERSION: u8 = 4;

pub const PERF_TRACE_SIZE: usize = 5 * 8;

static PERF_TRACE_ENABLED: AtomicBool = AtomicBool::new(false);

#[inline]
pub fn perf_trace_enabled() -> bool {
    PERF_TRACE_ENABLED.load(Ordering::Relaxed)
}

pub fn set_perf_trace_enabled(enabled: bool) {
    PERF_TRACE_ENABLED.store(enabled, Ordering::Relaxed);
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PerfTrace {
    pub ws_recv_ns: u64,
    pub decode_start_ns: u64,
    pub decode_end_ns: u64,
    pub pack_start_ns: u64,
    pub channel_send_ns: u64,
}

impl PerfTrace {
    pub fn on_ws_recv() -> Self {
        Self {
            ws_recv_ns: now_ns(),
            ..Self::default()
        }
    }
}

#[cfg(not(target_family = "wasm"))]
#[inline]
pub fn now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(target_family = "wasm")]
#[inline]
pub fn now_ns() -> u64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map(|perf| ((perf.time_origin() + perf.now()) * 1.0e6) as u64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameKind {
    Value = 1,
    Image = 2,
    PointCloud = 3,
    Packed = 4,
    Error = 5,
}

impl FrameKind {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Value),
            2 => Some(Self::Image),
            3 => Some(Self::PointCloud),
            4 => Some(Self::Packed),
            5 => Some(Self::Error),
            _ => None,
        }
    }
}

pub mod flags {
    pub const STALE_REPLAY: u16 = 1 << 0;
    pub const LOSSY_DROPPED: u16 = 1 << 1;
    pub const IMAGE_COMPRESSED: u16 = 1 << 2;
    pub const PERF_TRACE: u16 = 1 << 3;
    pub const PAYLOAD_CBOR: u16 = 1 << 4;
}

fn write_field(buffer: &mut Vec<u8>, bytes: &[u8]) {
    buffer.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    buffer.extend_from_slice(bytes);
}

#[derive(Debug, thiserror::Error)]
pub enum CborPackError {
    #[error("cbor serialize: {0}")]
    Serialize(String),
}

#[allow(clippy::too_many_arguments)]
pub fn pack_frame_with_cbor_perf<T: serde::Serialize + ?Sized>(
    handle: &str,
    timestamp_ns: u64,
    frame_kind: FrameKind,
    flags: u16,
    payload: &T,
    payload_size_hint: usize,
    perf: Option<PerfTrace>,
) -> Result<Vec<u8>, CborPackError> {
    let handle_bytes = handle.as_bytes();

    let perf_tail = if perf.is_some() { PERF_TRACE_SIZE } else { 0 };
    let mut effective_flags = flags | flags::PAYLOAD_CBOR;
    if perf.is_some() {
        effective_flags |= flags::PERF_TRACE;
    } else {
        effective_flags &= !flags::PERF_TRACE;
    }

    let prelude = 1 + 1 + 2 + 8 + 4 + handle_bytes.len() + 4;

    let mut buffer = Vec::with_capacity(prelude + payload_size_hint + perf_tail);
    buffer.push(WIRE_VERSION);
    buffer.push(frame_kind as u8);
    buffer.extend_from_slice(&effective_flags.to_le_bytes());
    buffer.extend_from_slice(&timestamp_ns.to_le_bytes());

    write_field(&mut buffer, handle_bytes);

    let len_offset = buffer.len();
    buffer.extend_from_slice(&[0u8; 4]);
    let payload_start = buffer.len();
    ciborium::ser::into_writer(payload, &mut buffer)
        .map_err(|err| CborPackError::Serialize(err.to_string()))?;
    let payload_len = (buffer.len() - payload_start) as u32;
    buffer[len_offset..len_offset + 4].copy_from_slice(&payload_len.to_le_bytes());

    if let Some(trace) = perf {
        append_perf(&mut buffer, &trace);
    }
    Ok(buffer)
}

pub fn pack_frame_raw(
    handle: &str,
    timestamp_ns: u64,
    frame_kind: FrameKind,
    flags: u16,
    payload: &[u8],
    perf: Option<PerfTrace>,
) -> Vec<u8> {
    let handle_bytes = handle.as_bytes();
    let perf_tail = if perf.is_some() { PERF_TRACE_SIZE } else { 0 };
    let mut effective_flags = flags & !flags::PAYLOAD_CBOR;
    if perf.is_some() {
        effective_flags |= flags::PERF_TRACE;
    } else {
        effective_flags &= !flags::PERF_TRACE;
    }

    let total = 1 + 1 + 2 + 8 + 4 + handle_bytes.len() + 4 + payload.len() + perf_tail;
    let mut buffer = Vec::with_capacity(total);
    buffer.push(WIRE_VERSION);
    buffer.push(frame_kind as u8);
    buffer.extend_from_slice(&effective_flags.to_le_bytes());
    buffer.extend_from_slice(&timestamp_ns.to_le_bytes());
    write_field(&mut buffer, handle_bytes);
    write_field(&mut buffer, payload);
    if let Some(trace) = perf {
        append_perf(&mut buffer, &trace);
    }
    buffer
}

fn append_perf(buffer: &mut Vec<u8>, trace: &PerfTrace) {
    buffer.extend_from_slice(&trace.ws_recv_ns.to_le_bytes());
    buffer.extend_from_slice(&trace.decode_start_ns.to_le_bytes());
    buffer.extend_from_slice(&trace.decode_end_ns.to_le_bytes());
    buffer.extend_from_slice(&trace.pack_start_ns.to_le_bytes());
    buffer.extend_from_slice(&trace.channel_send_ns.to_le_bytes());
}

#[derive(Debug)]
pub struct UnpackedFrame<'a> {
    pub version: u8,
    pub frame_kind: FrameKind,
    pub flags: u16,
    pub timestamp_ns: u64,
    pub handle: &'a str,
    pub payload: &'a [u8],
    pub perf: Option<PerfTrace>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum UnpackError {
    #[error("buffer too short: {0} bytes")]
    TooShort(usize),
    #[error("unknown wire version {0}")]
    UnknownVersion(u8),
    #[error("unknown frame kind {0}")]
    UnknownFrameKind(u8),
    #[error("malformed utf-8 in {field}")]
    InvalidUtf8 { field: &'static str },
}

pub fn unpack_frame(buffer: &[u8]) -> Result<UnpackedFrame<'_>, UnpackError> {
    if buffer.len() < 12 {
        return Err(UnpackError::TooShort(buffer.len()));
    }
    let version = buffer[0];
    if version != WIRE_VERSION {
        return Err(UnpackError::UnknownVersion(version));
    }
    let kind_byte = buffer[1];
    let frame_kind =
        FrameKind::from_u8(kind_byte).ok_or(UnpackError::UnknownFrameKind(kind_byte))?;
    let flags = u16::from_le_bytes([buffer[2], buffer[3]]);
    let timestamp_ns = u64::from_le_bytes([
        buffer[4], buffer[5], buffer[6], buffer[7], buffer[8], buffer[9], buffer[10], buffer[11],
    ]);

    let cursor = 12usize;
    let (handle, cursor) = read_str(buffer, cursor, "handle")?;
    let (payload, next) = read_bytes(buffer, cursor)?;

    let perf = if (flags & flags::PERF_TRACE) != 0 && buffer.len() >= next + PERF_TRACE_SIZE {
        let read_u64 = |offset: usize| {
            u64::from_le_bytes([
                buffer[offset],
                buffer[offset + 1],
                buffer[offset + 2],
                buffer[offset + 3],
                buffer[offset + 4],
                buffer[offset + 5],
                buffer[offset + 6],
                buffer[offset + 7],
            ])
        };
        Some(PerfTrace {
            ws_recv_ns: read_u64(next),
            decode_start_ns: read_u64(next + 8),
            decode_end_ns: read_u64(next + 16),
            pack_start_ns: read_u64(next + 24),
            channel_send_ns: read_u64(next + 32),
        })
    } else {
        None
    };

    Ok(UnpackedFrame {
        version,
        frame_kind,
        flags,
        timestamp_ns,
        handle,
        payload,
        perf,
    })
}

fn read_str<'a>(
    buffer: &'a [u8],
    offset: usize,
    field: &'static str,
) -> Result<(&'a str, usize), UnpackError> {
    let (bytes, next) = read_bytes(buffer, offset)?;
    std::str::from_utf8(bytes)
        .map(|s| (s, next))
        .map_err(|_| UnpackError::InvalidUtf8 { field })
}

fn read_bytes(buffer: &[u8], offset: usize) -> Result<(&[u8], usize), UnpackError> {
    if buffer.len() < offset + 4 {
        return Err(UnpackError::TooShort(buffer.len()));
    }
    let len = u32::from_le_bytes([
        buffer[offset],
        buffer[offset + 1],
        buffer[offset + 2],
        buffer[offset + 3],
    ]) as usize;
    let start = offset + 4;
    let end = start + len;
    if buffer.len() < end {
        return Err(UnpackError::TooShort(buffer.len()));
    }
    Ok((&buffer[start..end], end))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cbor(value: &serde_json::Value, flags: u16, perf: Option<PerfTrace>) -> Vec<u8> {
        pack_frame_with_cbor_perf(
            "handle-1",
            1_700_000_000_000_000_000,
            FrameKind::Value,
            flags,
            value,
            64,
            perf,
        )
        .unwrap()
    }

    #[test]
    fn pack_unpack_roundtrip() {
        let payload = serde_json::json!({ "kind": "f64", "value": 1.5 });
        let packed = cbor(&payload, 0, None);
        let unpacked = unpack_frame(&packed).unwrap();
        assert_eq!(unpacked.version, WIRE_VERSION);
        assert_eq!(unpacked.frame_kind, FrameKind::Value);
        assert_eq!(unpacked.timestamp_ns, 1_700_000_000_000_000_000);
        assert_eq!(unpacked.handle, "handle-1");
        assert_ne!(unpacked.flags & flags::PAYLOAD_CBOR, 0);
        let decoded: serde_json::Value =
            ciborium::de::from_reader(unpacked.payload).expect("ciborium decode");
        assert_eq!(decoded, payload);
    }

    #[test]
    fn caller_flags_survive() {
        let packed = cbor(&serde_json::json!(1u32), flags::STALE_REPLAY, None);
        let unpacked = unpack_frame(&packed).unwrap();
        assert_ne!(unpacked.flags & flags::STALE_REPLAY, 0);
        assert_eq!(unpacked.flags & flags::PERF_TRACE, 0);
        assert_eq!(unpacked.perf, None);
    }

    #[test]
    fn perf_trace_flag_sets_when_some_passed() {
        let packed = cbor(&serde_json::json!(1u32), 0, Some(PerfTrace::default()));
        let unpacked = unpack_frame(&packed).unwrap();
        assert_ne!(unpacked.flags & flags::PERF_TRACE, 0);
        assert_eq!(unpacked.perf, Some(PerfTrace::default()));
    }

    #[test]
    fn perf_trace_tail_roundtrips() {
        let trace = PerfTrace {
            ws_recv_ns: 100,
            decode_start_ns: 200,
            decode_end_ns: 300,
            pack_start_ns: 400,
            channel_send_ns: 500,
        };
        let packed = cbor(
            &serde_json::json!({ "kind": "int", "value": -42 }),
            0,
            Some(trace),
        );
        let unpacked = unpack_frame(&packed).unwrap();
        assert_ne!(unpacked.flags & flags::PAYLOAD_CBOR, 0);
        assert_eq!(unpacked.perf, Some(trace));
        let value: serde_json::Value = ciborium::de::from_reader(unpacked.payload).unwrap();
        assert_eq!(value["value"], -42);
    }

    #[test]
    fn unknown_version_rejected() {
        let mut packed = cbor(&serde_json::json!(0u32), 0, None);
        packed[0] = 99;
        assert!(matches!(
            unpack_frame(&packed),
            Err(UnpackError::UnknownVersion(99))
        ));
    }

    #[test]
    fn too_short_rejected() {
        assert!(matches!(
            unpack_frame(&[WIRE_VERSION, 1, 0]),
            Err(UnpackError::TooShort(3))
        ));
    }

    #[test]
    fn raw_pack_unpack_roundtrip() {
        let payload = [9u8, 8, 7, 6, 5];
        let packed = pack_frame_raw(
            "img",
            42,
            FrameKind::Image,
            flags::IMAGE_COMPRESSED,
            &payload,
            None,
        );
        let unpacked = unpack_frame(&packed).unwrap();
        assert_eq!(unpacked.frame_kind, FrameKind::Image);
        assert_eq!(unpacked.handle, "img");
        assert_eq!(unpacked.payload, &payload);
        assert_eq!(unpacked.flags & flags::PAYLOAD_CBOR, 0);
        assert_ne!(unpacked.flags & flags::IMAGE_COMPRESSED, 0);
    }
}

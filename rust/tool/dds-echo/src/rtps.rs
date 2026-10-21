//! A minimal, bounds-safe RTPS (DDSI-RTPS 2.x) header and submessage parser.
//!
//! It decodes just enough of each submessage to describe it in the inspector's
//! detail pane: the submessage id, the reader/writer entity ids, sequence
//! numbers, and the submessage length. It is deliberately defensive - it parses
//! arbitrary bytes off the wire, so every read is length-checked and it never
//! panics on malformed or truncated input.

/// The 20-byte RTPS message header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    /// Protocol version (major, minor).
    pub version: (u8, u8),
    /// Vendor id.
    pub vendor: [u8; 2],
    /// The sender participant's GUID prefix.
    pub guid_prefix: [u8; 12],
}

/// One decoded submessage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    /// The submessage id (`0x15` DATA, `0x07` HEARTBEAT, ...).
    pub id: u8,
    /// The submessage name, or `"UNKNOWN"` for an id we do not name.
    pub kind: &'static str,
    /// The writer entity id, when the submessage carries one.
    pub writer: Option<[u8; 4]>,
    /// The reader entity id, when the submessage carries one.
    pub reader: Option<[u8; 4]>,
    /// The writer sequence number, or a heartbeat's first sequence number.
    pub sn: Option<u64>,
    /// A heartbeat's last sequence number.
    pub sn_last: Option<u64>,
    /// A heartbeat/acknack count.
    pub count: Option<u32>,
    /// The submessage length (resolved to the end of the message when `0`).
    pub len: usize,
}

/// The result of parsing one datagram.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parsed {
    /// The RTPS header, or `None` when the input is not RTPS.
    pub header: Option<Header>,
    /// The decoded submessages, in order.
    pub frames: Vec<Frame>,
    /// The source participant GUID prefix: the header's, updated by any
    /// `INFO_SRC` submessage.
    pub src_guid_prefix: Option<[u8; 12]>,
}

const RTPS_HEADER_LEN: usize = 20;

pub fn is_rtps(data: &[u8]) -> bool {
    data.starts_with(b"RTPS")
}

/// The submessage id's name.
pub fn kind(id: u8) -> &'static str {
    match id {
        0x01 => "PAD",
        0x06 => "ACKNACK",
        0x07 => "HEARTBEAT",
        0x08 => "GAP",
        0x09 => "INFO_TS",
        0x0c => "INFO_SRC",
        0x0e => "INFO_DST",
        0x15 => "DATA",
        0x16 => "DATA_FRAG",
        _ => "UNKNOWN",
    }
}

/// Lowercase hex pairs for a byte slice.
pub fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Parse a datagram into its header and submessages. Non-RTPS input yields an
/// empty [`Parsed`] ([`Parsed::header`] is `None`).
pub fn parse(data: &[u8]) -> Parsed {
    if data.len() < RTPS_HEADER_LEN || !is_rtps(data) {
        return Parsed {
            header: None,
            frames: Vec::new(),
            src_guid_prefix: None,
        };
    }
    let header = Header {
        version: (data[4], data[5]),
        vendor: [data[6], data[7]],
        guid_prefix: array12(&data[8..20]),
    };
    let mut src = header.guid_prefix;
    let mut frames = Vec::new();

    let mut off = RTPS_HEADER_LEN;
    while off + 4 <= data.len() {
        let id = data[off];
        let flags = data[off + 1];
        let le = flags & 0x01 != 0;
        let Some(length) = read_u16(data, off + 2, le) else {
            break;
        };
        let length = length as usize;
        let body = off + 4;
        let end = if length == 0 {
            data.len()
        } else {
            body.saturating_add(length).min(data.len())
        };
        let sub = &data[body..end];

        let mut frame = Frame {
            id,
            kind: kind(id),
            writer: None,
            reader: None,
            sn: None,
            sn_last: None,
            count: None,
            len: sub.len(),
        };

        match id {
            0x15 | 0x16 | 0x06 | 0x07 | 0x08 => {
                // DATA/DATA_FRAG begin with `extraFlags` (present only when Q
                // is set) followed by `octetsToInlineQos`, before the entity
                // ids. CycloneDDS always emits `extraFlags` even when Q is
                // clear, so locate `octetsToInlineQos` - it is always 16
                // (readerId + writerId + writerSN) and sits just before the
                // reader id - rather than trusting the Q flag.
                let base = if matches!(id, 0x15 | 0x16) {
                    if read_u16(sub, 2, le) == Some(16) {
                        4
                    } else {
                        2
                    }
                } else {
                    0
                };
                frame.reader = read_entity(sub, base);
                frame.writer = read_entity(sub, base + 4);
                frame.sn = read_seq(sub, base + 8, le);
                if id == 0x07 {
                    frame.sn_last = read_seq(sub, base + 16, le);
                    frame.count = read_u32(sub, base + 24, le);
                }
            }
            0x0c => {
                if let Some(prefix) = read_array12(sub, 8) {
                    src = prefix;
                }
            }
            _ => {}
        }

        frames.push(frame);
        if length == 0 || end <= off {
            break;
        }
        off = end;
    }

    Parsed {
        header: Some(header),
        frames,
        src_guid_prefix: Some(src),
    }
}

fn read_u16(data: &[u8], at: usize, le: bool) -> Option<u16> {
    let b = data.get(at..at + 2)?;
    let a = [b[0], b[1]];
    Some(if le {
        u16::from_le_bytes(a)
    } else {
        u16::from_be_bytes(a)
    })
}

fn read_u32(data: &[u8], at: usize, le: bool) -> Option<u32> {
    let b = data.get(at..at + 4)?;
    let a = [b[0], b[1], b[2], b[3]];
    Some(if le {
        u32::from_le_bytes(a)
    } else {
        u32::from_be_bytes(a)
    })
}

/// A 64-bit RTPS sequence number (an `i32` high word and a `u32` low word).
fn read_seq(data: &[u8], at: usize, le: bool) -> Option<u64> {
    let high = read_u32(data, at, le)? as u64;
    let low = read_u32(data, at + 4, le)? as u64;
    Some((high << 32) | low)
}

fn read_entity(data: &[u8], at: usize) -> Option<[u8; 4]> {
    let b = data.get(at..at + 4)?;
    Some([b[0], b[1], b[2], b[3]])
}

fn array12(data: &[u8]) -> [u8; 12] {
    let mut out = [0u8; 12];
    out.copy_from_slice(&data[..12]);
    out
}

fn read_array12(data: &[u8], at: usize) -> Option<[u8; 12]> {
    let b = data.get(at..at + 12)?;
    Some(array12(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"RTPS");
        v.extend_from_slice(&[2, 4]);
        v.extend_from_slice(&[0x01, 0x0f]);
        v.extend_from_slice(&[0u8; 12]);
        v
    }

    #[test]
    fn parses_header() {
        let mut d = header();
        d[8..20].copy_from_slice(&[0xaa; 12]);
        let p = parse(&d);
        let h = p.header.expect("header");
        assert_eq!(h.version, (2, 4));
        assert_eq!(h.vendor, [0x01, 0x0f]);
        assert_eq!(h.guid_prefix, [0xaa; 12]);
        assert_eq!(p.src_guid_prefix, Some([0xaa; 12]));
        assert!(p.frames.is_empty());
    }

    #[test]
    fn parses_data_submessage() {
        let mut d = header();
        d.extend_from_slice(&[0x15, 0x05]); // id, flags E|D
        d.extend_from_slice(&24u16.to_le_bytes());
        d.extend_from_slice(&0u16.to_le_bytes()); // extraFlags
        d.extend_from_slice(&16u16.to_le_bytes()); // octetsToInlineQos
        d.extend_from_slice(&[0, 0, 0, 0]); // readerId
        d.extend_from_slice(&[0, 0, 0x03, 0xc2]); // writerId
        d.extend_from_slice(&0u32.to_le_bytes()); // writerSN high
        d.extend_from_slice(&42u32.to_le_bytes()); // writerSN low
        d.extend_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd]); // payload
        let p = parse(&d);
        assert_eq!(p.frames.len(), 1);
        let f = &p.frames[0];
        assert_eq!(f.id, 0x15);
        assert_eq!(f.kind, "DATA");
        assert_eq!(f.reader, Some([0, 0, 0, 0]));
        assert_eq!(f.writer, Some([0, 0, 0x03, 0xc2]));
        assert_eq!(f.sn, Some(42));
        assert_eq!(f.len, 24);
    }

    #[test]
    fn parses_heartbeat() {
        let mut d = header();
        d.extend_from_slice(&[0x07, 0x01]); // id, flags E
        d.extend_from_slice(&28u16.to_le_bytes());
        d.extend_from_slice(&[0, 0, 0, 0]); // readerId
        d.extend_from_slice(&[0, 0, 0x03, 0xc2]); // writerId
        d.extend_from_slice(&0u32.to_le_bytes()); // firstSN high
        d.extend_from_slice(&1u32.to_le_bytes()); // firstSN low -> 1
        d.extend_from_slice(&0u32.to_le_bytes()); // lastSN high
        d.extend_from_slice(&5u32.to_le_bytes()); // lastSN low -> 5
        d.extend_from_slice(&7u32.to_le_bytes()); // count
        let p = parse(&d);
        let f = &p.frames[0];
        assert_eq!(f.kind, "HEARTBEAT");
        assert_eq!(f.sn, Some(1));
        assert_eq!(f.sn_last, Some(5));
        assert_eq!(f.count, Some(7));
    }

    #[test]
    fn parses_big_endian_lengths_and_sn() {
        let mut d = header();
        d.extend_from_slice(&[0x15, 0x04]); // E = 0 (big-endian), D
        d.extend_from_slice(&24u16.to_be_bytes());
        d.extend_from_slice(&0u16.to_be_bytes()); // extraFlags
        d.extend_from_slice(&16u16.to_be_bytes()); // octetsToInlineQos
        d.extend_from_slice(&[0, 0, 0, 0]);
        d.extend_from_slice(&[0, 0, 0x03, 0xc2]);
        d.extend_from_slice(&0u32.to_be_bytes());
        d.extend_from_slice(&9u32.to_be_bytes());
        d.extend_from_slice(&[0, 0, 0, 0]);
        let p = parse(&d);
        assert_eq!(p.frames[0].sn, Some(9));
        assert_eq!(p.frames[0].len, 24);
    }

    #[test]
    fn parses_spec_data_without_extra_flags() {
        // A spec-style DATA (no extraFlags when Q is clear): octetsToInlineQos
        // is the first field, so readerId follows it directly.
        let mut d = header();
        d.extend_from_slice(&[0x15, 0x05]); // E|D
        d.extend_from_slice(&20u16.to_le_bytes());
        d.extend_from_slice(&16u16.to_le_bytes()); // octetsToInlineQos
        d.extend_from_slice(&[0, 0, 0, 0]); // readerId
        d.extend_from_slice(&[0, 0, 0x03, 0xc2]); // writerId
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&3u32.to_le_bytes());
        d.extend_from_slice(&[1, 2]); // payload
        let p = parse(&d);
        assert_eq!(p.frames[0].reader, Some([0, 0, 0, 0]));
        assert_eq!(p.frames[0].writer, Some([0, 0, 0x03, 0xc2]));
        assert_eq!(p.frames[0].sn, Some(3));
    }

    #[test]
    fn info_src_updates_prefix() {
        let mut d = header();
        d.extend_from_slice(&[0x0c, 0x01]);
        d.extend_from_slice(&20u16.to_le_bytes()); // unused(4)+version(2)+vendor(2)+prefix(12)
        d.extend_from_slice(&[0, 0, 0, 0]); // unused
        d.extend_from_slice(&[2, 3]); // version
        d.extend_from_slice(&[0x01, 0x0f]); // vendor
        d.extend_from_slice(&[0xaa; 12]); // prefix
        let p = parse(&d);
        assert_eq!(p.header.unwrap().guid_prefix, [0; 12]);
        assert_eq!(p.src_guid_prefix, Some([0xaa; 12]));
    }

    #[test]
    fn length_zero_extends_to_end() {
        let mut d = header();
        d.extend_from_slice(&[0x15, 0x05]);
        d.extend_from_slice(&0u16.to_le_bytes());
        d.extend_from_slice(&0u16.to_le_bytes()); // extraFlags
        d.extend_from_slice(&16u16.to_le_bytes()); // octetsToInlineQos
        d.extend_from_slice(&[0, 0, 0, 0]);
        d.extend_from_slice(&[0, 0, 0x03, 0xc2]);
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&1u32.to_le_bytes());
        let p = parse(&d);
        assert_eq!(p.frames.len(), 1);
        assert_eq!(p.frames[0].len, d.len() - 24);
    }

    #[test]
    fn short_data_fields_are_none() {
        let mut d = header();
        d.extend_from_slice(&[0x15, 0x05]);
        d.extend_from_slice(&8u16.to_le_bytes()); // extraFlags + octetsToInlineQos + readerId
        d.extend_from_slice(&0u16.to_le_bytes()); // extraFlags
        d.extend_from_slice(&16u16.to_le_bytes()); // octetsToInlineQos
        d.extend_from_slice(&[0, 0, 0, 0]); // readerId only
        let p = parse(&d);
        assert_eq!(p.frames[0].reader, Some([0, 0, 0, 0]));
        assert_eq!(p.frames[0].writer, None);
        assert_eq!(p.frames[0].sn, None);
    }

    #[test]
    fn truncated_and_unknown_do_not_panic() {
        let mut d = header();
        d.extend_from_slice(&[0x99, 0x01]); // unknown id
        d.extend_from_slice(&100u16.to_le_bytes()); // length beyond the end
        d.extend_from_slice(&[1, 2, 3]);
        let p = parse(&d);
        assert_eq!(p.frames.len(), 1);
        assert_eq!(p.frames[0].kind, "UNKNOWN");
        assert_eq!(p.frames[0].id, 0x99);

        assert!(parse(b"RTPS").header.is_none()); // too short
        assert!(parse(b"not rtps at all......").header.is_none());
    }

    #[test]
    fn hex_is_lowercase_pairs() {
        assert_eq!(hex(&[0x0a, 0xbc, 0xde]), "0abcde");
    }
}

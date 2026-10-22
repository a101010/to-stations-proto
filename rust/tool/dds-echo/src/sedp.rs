//! A minimal, bounds-safe parser for DDSI-RTPS discovery payloads.
//!
//! SPDP (participants) and SEDP (endpoints/topics) carry a PL_CDR serialized
//! payload: a 4-byte encapsulation header followed by a parameter list. This
//! decodes only the parameters the inspector needs - participant/endpoint GUIDs
//! and topic/type names - and is deliberately defensive (the input comes off
//! the wire, so every read is length-checked and it never panics).

/// Which an announced endpoint is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndpointKind {
    Writer,
    Reader,
    Other,
}

/// One discovery announcement decoded from a DATA payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Discovered {
    /// A participant announcement (SPDP).
    Participant { guid_prefix: [u8; 12] },
    /// An endpoint announcement (SEDP).
    Endpoint {
        guid: [u8; 16],
        kind: EndpointKind,
        topic: Option<String>,
        type_name: Option<String>,
    },
}

const PID_SENTINEL: u16 = 0x0001;
const PID_TOPIC_NAME: u16 = 0x0005;
const PID_TYPE_NAME: u16 = 0x0007;
const PID_PARTICIPANT_GUID: u16 = 0x0050;
const PID_ENDPOINT_GUID: u16 = 0x005a;

/// Parse a PL_CDR serialized payload (encapsulation header + parameter list).
///
/// Non-parameter-list payloads (CDR/CDR2), malformed, or truncated input yield
/// `None`.
pub fn parse(payload: &[u8]) -> Option<Discovered> {
    // The representation identifier is two octets, big-endian on the wire:
    // 0x0002/0x000a is PL_CDR_BE/PL_CDR2_BE, 0x0003/0x000b PL_CDR_LE/PL_CDR2_LE.
    let header = payload.get(0..4)?;
    let identifier = u16::from_be_bytes([header[0], header[1]]);
    let le = match identifier {
        0x0002 | 0x000a => false,
        0x0003 | 0x000b => true,
        _ => return None,
    };

    let mut participant = None;
    let mut endpoint = None;
    let mut topic = None;
    let mut type_name = None;

    let mut off = 4;
    while off + 4 <= payload.len() {
        let pid = read_u16(payload, off, le)?;
        let len = read_u16(payload, off + 2, le)? as usize;
        off += 4;
        if pid == PID_SENTINEL {
            break;
        }
        let value = payload.get(off..off + len)?;
        match pid {
            PID_PARTICIPANT_GUID => participant = value.get(0..12).map(array12),
            PID_ENDPOINT_GUID => endpoint = value.get(0..16).map(array16),
            PID_TOPIC_NAME => topic = read_string(value, le),
            PID_TYPE_NAME => type_name = read_string(value, le),
            _ => {}
        }
        off += len;
    }

    if let Some(guid) = endpoint {
        Some(Discovered::Endpoint {
            guid,
            kind: kind_of(&guid[12..16]),
            topic,
            type_name,
        })
    } else {
        participant.map(|guid_prefix| Discovered::Participant { guid_prefix })
    }
}

/// A CDR string: a 4-byte length (including the NUL) then that many octets,
/// with the trailing NUL dropped.
fn read_string(value: &[u8], le: bool) -> Option<String> {
    let len = read_u32(value, 0, le)? as usize;
    let bytes = value.get(4..4 + len)?;
    let bytes = match bytes.split_last() {
        Some((0, rest)) => rest,
        _ => bytes,
    };
    Some(String::from_utf8_lossy(bytes).into_owned())
}

/// The kind of an entity id (its last octet's low 6 bits).
fn kind_of(entity: &[u8]) -> EndpointKind {
    match entity.get(3).copied().unwrap_or(0) & 0x3f {
        0x02 | 0x03 => EndpointKind::Writer,
        0x04 | 0x07 => EndpointKind::Reader,
        _ => EndpointKind::Other,
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

fn array12(data: &[u8]) -> [u8; 12] {
    let mut out = [0u8; 12];
    out.copy_from_slice(&data[..12]);
    out
}

fn array16(data: &[u8]) -> [u8; 16] {
    let mut out = [0u8; 16];
    out.copy_from_slice(&data[..16]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A parameter with the length padded to a multiple of 4, as on the wire.
    fn param(pid: u16, value: &[u8], le: bool) -> Vec<u8> {
        let padded = (value.len() + 3) & !3;
        let mut v = Vec::new();
        v.extend_from_slice(&u16_bytes(pid, le));
        v.extend_from_slice(&u16_bytes(padded as u16, le));
        v.extend_from_slice(value);
        v.resize(v.len() + (padded - value.len()), 0);
        v
    }

    fn u16_bytes(x: u16, le: bool) -> [u8; 2] {
        if le {
            x.to_le_bytes()
        } else {
            x.to_be_bytes()
        }
    }

    fn cdr_string(s: &str, le: bool) -> Vec<u8> {
        let n = s.len() as u32 + 1;
        let bytes = if le { n.to_le_bytes() } else { n.to_be_bytes() };
        let mut v = Vec::new();
        v.extend_from_slice(&bytes);
        v.extend_from_slice(s.as_bytes());
        v.push(0);
        v
    }

    fn encapsulation(le: bool) -> Vec<u8> {
        if le {
            vec![0x00, 0x03, 0x00, 0x00]
        } else {
            vec![0x00, 0x02, 0x00, 0x00]
        }
    }

    #[test]
    fn parses_endpoint_le() {
        let guid = {
            let mut g = [0u8; 16];
            g[0..12].copy_from_slice(&[0xaa; 12]);
            g[12..16].copy_from_slice(&[0, 0, 0x01, 0x02]); // writer, no key
            g
        };
        let mut pl = encapsulation(true);
        pl.extend(param(PID_ENDPOINT_GUID, &guid, true));
        pl.extend(param(PID_TOPIC_NAME, &cdr_string("HelloWorldTopic", true), true));
        pl.extend(param(
            PID_TYPE_NAME,
            &cdr_string("hello_world::HelloWorldTopic", true),
            true,
        ));
        pl.extend(param(PID_SENTINEL, &[], true));

        match parse(&pl).expect("discovered") {
            Discovered::Endpoint {
                guid: g,
                kind,
                topic,
                type_name,
            } => {
                assert_eq!(g, guid);
                assert_eq!(kind, EndpointKind::Writer);
                assert_eq!(topic.as_deref(), Some("HelloWorldTopic"));
                assert_eq!(type_name.as_deref(), Some("hello_world::HelloWorldTopic"));
            }
            other => panic!("expected endpoint, got {other:?}"),
        }
    }

    #[test]
    fn parses_endpoint_be_and_reader_kind() {
        let guid = {
            let mut g = [0u8; 16];
            g[0..12].copy_from_slice(&[0xbb; 12]);
            g[12..16].copy_from_slice(&[0, 0, 0x01, 0x07]); // reader, with key
            g
        };
        let mut pl = encapsulation(false);
        pl.extend(param(PID_ENDPOINT_GUID, &guid, false));
        pl.extend(param(PID_SENTINEL, &[], false));

        match parse(&pl).expect("discovered") {
            Discovered::Endpoint { kind, topic, .. } => {
                assert_eq!(kind, EndpointKind::Reader);
                assert_eq!(topic, None);
            }
            other => panic!("expected endpoint, got {other:?}"),
        }
    }

    #[test]
    fn parses_participant() {
        let prefix = [0xcc; 12];
        let mut pl = encapsulation(true);
        pl.extend(param(PID_PARTICIPANT_GUID, &prefix, true));
        pl.extend(param(PID_SENTINEL, &[], true));

        match parse(&pl).expect("discovered") {
            Discovered::Participant { guid_prefix } => assert_eq!(guid_prefix, prefix),
            other => panic!("expected participant, got {other:?}"),
        }
    }

    #[test]
    fn padded_string_is_read() {
        // 4-char name plus NUL is exactly 5 bytes, padded to 8 on the wire.
        let mut pl = encapsulation(true);
        pl.extend(param(PID_TOPIC_NAME, &cdr_string("abcd", true), true));
        // No endpoint/participant GUID -> still None (nothing to report).
        assert!(parse(&pl).is_none());
    }

    #[test]
    fn rejects_non_parameter_list_payloads() {
        assert!(parse(&[0x00, 0x01, 0x00, 0x00]).is_none()); // CDR_LE
        assert!(parse(&[0x00, 0x00, 0x00, 0x00]).is_none()); // CDR_BE
        assert!(parse(&[]).is_none());
        assert!(parse(&[0x00, 0x03]).is_none()); // shorter than the header
    }

    #[test]
    fn truncated_parameter_does_not_panic() {
        let mut pl = encapsulation(true);
        // A parameter claiming 100 bytes with none present.
        pl.extend_from_slice(&u16_bytes(PID_ENDPOINT_GUID, true));
        pl.extend_from_slice(&u16_bytes(100, true));
        assert!(parse(&pl).is_none());
    }

    /// The SEDP endpoint vector from CycloneDDS's own test
    /// (`src/core/ddsi/tests/plist_leasedur.c`, the subscriptions-writer DATA):
    /// PL_CDR_BE, `PID_ENDPOINT_GUID(16)` + `PID_TOPIC_NAME(12)` +
    /// `PID_TYPE_NAME(12)`, where a string is `SER32BE(len)` then the bytes.
    #[test]
    fn parses_cyclonedds_endpoint_vector() {
        let prefix = [7u8, 7, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let mut guid = [0u8; 16];
        guid[..12].copy_from_slice(&prefix);
        guid[12..].copy_from_slice(&[0, 0, 0x01, 0x07]); // reader, with key

        let mut pl = encapsulation(false); // PL_CDR_BE
        pl.extend(param(PID_ENDPOINT_GUID, &guid, false));
        pl.extend(param(PID_TOPIC_NAME, &cdr_string("topic", false), false));
        pl.extend(param(PID_TYPE_NAME, &cdr_string("type", false), false));
        // PID_LIVELINESS (0x1b), 12 bytes: kind + count + duration.
        pl.extend(param(
            0x001b,
            &[0, 0, 0, 1, 0, 0, 0, 2, 0x0a, 0xbc, 0xde, 0xfb],
            false,
        ));
        pl.extend(param(PID_SENTINEL, &[], false));

        match parse(&pl).expect("discovered") {
            Discovered::Endpoint {
                guid: g,
                kind,
                topic,
                type_name,
            } => {
                assert_eq!(g, guid);
                assert_eq!(kind, EndpointKind::Reader);
                assert_eq!(topic.as_deref(), Some("topic"));
                assert_eq!(type_name.as_deref(), Some("type"));
            }
            other => panic!("expected endpoint, got {other:?}"),
        }
    }

    /// The SPDP participant vector from the same CycloneDDS test (`PID_PARTICIPANT_GUID`).
    #[test]
    fn parses_cyclonedds_participant_vector() {
        let prefix = [7u8, 7, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let mut guid = [0u8; 16];
        guid[..12].copy_from_slice(&prefix);
        guid[12..].copy_from_slice(&[0, 0, 0x01, 0xc1]); // ENTITYID_PARTICIPANT

        let mut pl = encapsulation(false);
        pl.extend(param(PID_PARTICIPANT_GUID, &guid, false));
        pl.extend(param(PID_SENTINEL, &[], false));

        match parse(&pl).expect("discovered") {
            Discovered::Participant { guid_prefix } => assert_eq!(guid_prefix, prefix),
            other => panic!("expected participant, got {other:?}"),
        }
    }

    #[test]
    fn unknown_pids_are_skipped() {
        let prefix = [0xdd; 12];
        let mut pl = encapsulation(true);
        pl.extend(param(0x1234, &[1, 2, 3, 4, 5, 6, 7, 8], true)); // vendor pid
        pl.extend(param(PID_PARTICIPANT_GUID, &prefix, true));
        pl.extend(param(PID_SENTINEL, &[], true));
        match parse(&pl).expect("discovered") {
            Discovered::Participant { guid_prefix } => assert_eq!(guid_prefix, prefix),
            other => panic!("expected participant, got {other:?}"),
        }
    }
}

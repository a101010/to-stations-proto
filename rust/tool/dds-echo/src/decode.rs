//! Decode known-type DATA samples with the DDS CDR deserializer.
//!
//! A DATA sample's serialized payload is a 4-byte encapsulation header followed
//! by the CDR body. `CdrDeserializer::deserialize` expects the CDR body in
//! native endianness without the header, so this strips it and only accepts a
//! little-endian (native) encapsulation. Field-level display is interim: the
//! sample is pretty-printed; a proper field model is deferred to
//! `topic-field-metadata`.

use stations_dds::cyclonedds::{CdrDeserializer, CdrEncoding, DdsType};
use stations_dds::hello_world::{HelloWorldTopic, HelloWorldViewTopic};

/// Decode a known-type DATA payload (encapsulation header + CDR) into a
/// human-readable field dump. Returns `None` for an unknown type, a
/// non-native-endian encapsulation, or malformed data.
pub fn decode(idl_type: &str, payload: &[u8]) -> Option<String> {
    match idl_type {
        "hello_world::HelloWorldTopic" => {
            let sample = deserialize::<HelloWorldTopic>(payload)?;
            Some(format!("{sample:#?}"))
        }
        "hello_world::HelloWorldViewTopic" => {
            let sample = deserialize::<HelloWorldViewTopic>(payload)?;
            Some(format!("{sample:#?}"))
        }
        _ => None,
    }
}

/// Strip the encapsulation header and deserialize the native-endian CDR body.
fn deserialize<T: DdsType>(payload: &[u8]) -> Option<T> {
    let header = payload.get(0..4)?;
    // The representation identifier is two octets, big-endian on the wire:
    // 0x0001 CDR_LE, 0x0007 CDR2_LE (native here); BE variants are not handled.
    let identifier = u16::from_be_bytes([header[0], header[1]]);
    let encoding = match identifier {
        0x0001 => CdrEncoding::Xcdr1,
        0x0007 => CdrEncoding::Xcdr2,
        _ => return None,
    };
    CdrDeserializer::<T>::deserialize(&payload[4..], encoding).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use stations_dds::cyclonedds::CdrSerializer;
    use stations_dds::math::Quaternion;

    const HELLO: &str = "hello_world::HelloWorldTopic";

    fn sample() -> HelloWorldTopic {
        HelloWorldTopic {
            greeting: "Hello, world!".to_string(),
            attitude: Quaternion {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
        }
    }

    /// The sample's CDR body behind a little-endian CDR encapsulation header.
    fn framed(sample: &HelloWorldTopic) -> Vec<u8> {
        let cdr = CdrSerializer::serialize(sample, CdrEncoding::Xcdr1).unwrap();
        let mut payload = vec![0x00, 0x01, 0x00, 0x00];
        payload.extend_from_slice(&cdr);
        payload
    }

    #[test]
    fn decodes_hello_world_fields() {
        let dump = decode(HELLO, &framed(&sample())).expect("decoded");
        assert!(dump.contains("greeting"));
        assert!(dump.contains("Hello, world!"));
        assert!(dump.contains("attitude"));
        assert!(dump.contains("w: 1.0"));
    }

    /// Mirrors `scripts/traffic-fake-topic.ps1`'s `CdrHelloWorldTopic` so the
    /// decoder is checked against the stimulus's hand-built layout.
    #[test]
    fn decodes_stimulus_style_payload() {
        let mut b = vec![0x00, 0x01, 0x00, 0x00]; // CDR_LE
        b.extend_from_slice(&14u32.to_le_bytes()); // string length incl NUL
        b.extend_from_slice(b"Hello, world!");
        b.push(0);
        while (b.len() - 4) % 8 != 0 {
            b.push(0); // align the CDR body to the doubles
        }
        for d in [0.0f64, 0.0, 0.0, 1.0] {
            b.extend_from_slice(&d.to_le_bytes());
        }
        let dump = decode(HELLO, &b).expect("decoded");
        assert!(dump.contains("Hello, world!"));
        assert!(dump.contains("w: 1.0"));
    }

    #[test]
    fn decodes_hello_world_view_fields() {
        let sample = HelloWorldViewTopic {
            greeting: "Hello, world!".to_string(),
            attitude: Quaternion {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
        };
        let cdr = CdrSerializer::serialize(&sample, CdrEncoding::Xcdr1).unwrap();
        let mut payload = vec![0x00, 0x01, 0x00, 0x00];
        payload.extend_from_slice(&cdr);
        let dump = decode("hello_world::HelloWorldViewTopic", &payload).expect("decoded");
        assert!(dump.contains("Hello, world!"));
        assert!(dump.contains("attitude"));
    }

    #[test]
    fn unknown_type_is_none() {
        assert!(decode("weather::WeatherTopic", &framed(&sample())).is_none());
    }

    #[test]
    fn non_native_encapsulation_is_none() {
        let mut payload = framed(&sample());
        payload[1] = 0x00; // CDR_BE
        assert!(decode(HELLO, &payload).is_none());
        assert!(decode(HELLO, &[]).is_none());
        assert!(decode(HELLO, &[0x00, 0x01]).is_none());
    }

    #[test]
    fn truncated_body_is_none() {
        let payload = framed(&sample());
        assert!(decode(HELLO, &payload[..6]).is_none());
    }
}

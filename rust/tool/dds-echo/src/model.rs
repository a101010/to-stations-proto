//! The `dds-echo` data model: records, statistics, and the topic counts.
//!
//! UI-agnostic (no Slint types) so it is unit-testable.

use std::collections::{BTreeMap, BTreeSet};

/// The datagram table cap.
pub const MAX_RECORDS: usize = 1000;
/// Bytes shown in the detail pane's hex view before truncating.
pub const HEX_PREVIEW: usize = 4096;
/// Bytes per line in the pre-wrapped hex view.
pub const HEX_BYTES_PER_LINE: usize = 16;
/// Known topics; the capture/SEDP stories read `contracts/topics.toml`.
pub const KNOWN_TOPICS: &[&str] = &["HelloWorldTopic"];

/// How much is understood about a message.
#[allow(dead_code)] // Unregistered/Known are produced by later stories
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// Not RTPS (no `RTPS` magic).
    Udp,
    /// RTPS, but no topic identified.
    Rtps,
    /// RTPS with an identified topic that is not in the registry.
    Unregistered,
    /// RTPS with a registered topic (decodable).
    Known,
}

/// One decoded RTPS submessage, formatted for display.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    /// The submessage id (`0x15` DATA, `0x07` HEARTBEAT, ...).
    pub id: u8,
    /// The submessage name, or `"0xNN"` for an id we do not name.
    pub kind: String,
    /// The writer entity id (hex) when present.
    pub writer: String,
    /// The reader entity id (hex) when present.
    pub reader: String,
    /// The sequence number, `"a..b"` for a range, else `""`.
    pub seq: String,
    /// The submessage length in bytes.
    pub len: usize,
    /// The discovery announcement this submessage's payload decoded, if any.
    pub announcement: Option<Announcement>,
}

/// Which a discovered endpoint is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndpointKind {
    Writer,
    Reader,
    Other,
}

impl EndpointKind {
    pub fn label(self) -> &'static str {
        match self {
            EndpointKind::Writer => "writer",
            EndpointKind::Reader => "reader",
            EndpointKind::Other => "other",
        }
    }
}

/// A discovered endpoint (from SEDP), formatted for display.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoint {
    /// The full endpoint GUID (hex, 32 chars): participant prefix + entity id.
    pub guid: String,
    /// The participant GUID prefix (hex, 24 chars).
    pub prefix: String,
    pub kind: EndpointKind,
    /// The topic the endpoint uses; empty for a builtin endpoint.
    pub topic: String,
    /// The IDL type name; empty when absent.
    pub type_name: String,
}

/// What a submessage's payload decoded to (SPDP/SEDP discovery).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Announcement {
    /// A participant announcement (SPDP): the announced participant prefix.
    Participant { prefix: String },
    /// An endpoint announcement (SEDP).
    Endpoint(Endpoint),
}

/// One captured datagram (or a decoded sample).
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub elapsed_ms: u64,
    pub class: Class,
    /// Sender address (`host:port`) — the table's `source`.
    pub src_addr: String,
    /// RTPS participant GUID prefix (hex), when known — detail only.
    pub guid_prefix: Option<String>,
    /// RTPS entity id (hex), when a topic is identified — detail only.
    pub entity_id: Option<String>,
    pub dest_port: u16,
    pub size: usize,
    pub topic: Option<String>,
    /// The submessage kinds in order (the table's `Submessages` column).
    pub submessages: String,
    pub hex: Vec<u8>,
    /// Rendered fields for a decoded [`Class::Known`] message; empty otherwise.
    pub decoded: String,
    /// The decoded RTPS submessages (empty for non-RTPS datagrams).
    pub frames: Vec<Frame>,
}

impl Record {
    /// `MM:SS.mmm` since capture started.
    pub fn time_label(&self) -> String {
        let ms = self.elapsed_ms;
        format!("{:02}:{:02}.{:03}", ms / 60_000, (ms / 1000) % 60, ms % 1000)
    }

    /// Lowercase hex pairs, first [`HEX_PREVIEW`] bytes, pre-wrapped at
    /// [`HEX_BYTES_PER_LINE`] bytes per line, with a note when truncated.
    pub fn hex_string(&self) -> String {
        let n = self.hex.len().min(HEX_PREVIEW);
        let mut out = String::with_capacity(n * 3 + n / HEX_BYTES_PER_LINE + 16);
        for (i, chunk) in self.hex[..n].chunks(HEX_BYTES_PER_LINE).enumerate() {
            if i > 0 {
                out.push('\n');
            }
            for (j, byte) in chunk.iter().enumerate() {
                if j > 0 {
                    out.push(' ');
                }
                out.push_str(&format!("{byte:02x}"));
            }
        }
        if self.hex.len() > n {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&format!("… (+{} bytes)", self.hex.len() - n));
        }
        out
    }

    pub fn is_known(&self) -> bool {
        self.topic
            .as_deref()
            .is_some_and(|t| KNOWN_TOPICS.contains(&t))
    }

    /// The transport protocol for the table's `Protocol` column.
    pub fn protocol(&self) -> &'static str {
        match self.class {
            Class::Udp => "UDP",
            _ => "RTPS",
        }
    }

    /// The datagram writer's full GUID (hex), when both its prefix and entity
    /// id are known.
    pub fn writer_guid(&self) -> Option<String> {
        match (&self.guid_prefix, &self.entity_id) {
            (Some(prefix), Some(entity)) => Some(format!("{prefix}{entity}")),
            _ => None,
        }
    }

    /// The submessage kinds in order, e.g. `"INFO_TS, DATA, DATA, HEARTBEAT"`
    /// (empty when there are none).
    pub fn submessage_list(&self) -> String {
        self.frames
            .iter()
            .map(|f| f.kind.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Cumulative counters.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stats {
    pub packets: usize,
    pub bytes: usize,
    pub rtps: usize,
}

impl Stats {
    pub fn observe(&mut self, r: &Record) {
        self.packets += 1;
        self.bytes += r.size;
        if r.class != Class::Udp {
            self.rtps += 1;
        }
    }

    pub fn reset(&mut self) {
        *self = Stats::default();
    }
}

/// A row in the topics view.
#[derive(Clone, Debug, PartialEq)]
pub struct TopicRow {
    pub name: String,
    pub known: bool,
    pub packets: usize,
}

/// Per-topic packet counts (known and unregistered topics) plus a count of
/// datagrams with no identified topic.
#[derive(Clone, Debug, Default)]
pub struct TopicCounts {
    counts: BTreeMap<String, usize>,
    unidentified: usize,
}

impl TopicCounts {
    pub fn observe(&mut self, r: &Record) {
        match &r.topic {
            Some(topic) => *self.counts.entry(topic.clone()).or_insert(0) += 1,
            None => self.unidentified += 1,
        }
    }

    /// Known topics (0 if unseen), then unregistered topics seen and an
    /// `unidentified` row when there are unattributed datagrams, sorted.
    pub fn rows(&self) -> Vec<TopicRow> {
        let mut rows = Vec::new();
        for name in KNOWN_TOPICS {
            rows.push(TopicRow {
                name: (*name).to_string(),
                known: true,
                packets: self.counts.get(*name).copied().unwrap_or(0),
            });
        }
        for (name, packets) in &self.counts {
            if !KNOWN_TOPICS.contains(&name.as_str()) {
                rows.push(TopicRow {
                    name: name.clone(),
                    known: false,
                    packets: *packets,
                });
            }
        }
        if self.unidentified > 0 {
            rows.push(TopicRow {
                name: "unidentified".to_string(),
                known: false,
                packets: self.unidentified,
            });
        }
        rows.sort_by(|a, b| b.known.cmp(&a.known).then_with(|| a.name.cmp(&b.name)));
        rows
    }

    pub fn reset(&mut self) {
        self.counts.clear();
        self.unidentified = 0;
    }
}

/// The participants, endpoints, and writer->topic map learned from SPDP/SEDP.
///
/// Accumulated on the UI thread; every item is keyed by GUID so repeated
/// announcements are deduplicated.
#[derive(Clone, Debug, Default)]
pub struct Discovery {
    participants: BTreeSet<String>,
    endpoints: BTreeMap<String, Endpoint>,
}

impl Discovery {
    /// Learn this datagram's announcements, then attribute its topic: when the
    /// record's topic is still unknown and its writer GUID matches a learned
    /// endpoint that carries a topic, set `record.topic` from it.
    pub fn observe_and_attr(&mut self, record: &mut Record) {
        for frame in &record.frames {
            match &frame.announcement {
                Some(Announcement::Participant { prefix }) => {
                    self.participants.insert(prefix.clone());
                }
                Some(Announcement::Endpoint(endpoint)) => {
                    self.participants.insert(endpoint.prefix.clone());
                    self.endpoints.insert(endpoint.guid.clone(), endpoint.clone());
                }
                None => {}
            }
        }

        if record.topic.is_none() {
            if let Some(guid) = record.writer_guid() {
                if let Some(endpoint) = self.endpoints.get(&guid) {
                    if !endpoint.topic.is_empty() {
                        record.topic = Some(endpoint.topic.clone());
                    }
                }
            }
        }
    }

    /// The number of distinct participants observed.
    pub fn participants(&self) -> usize {
        self.participants.len()
    }

    /// The number of distinct endpoints observed.
    pub fn endpoints(&self) -> usize {
        self.endpoints.len()
    }

    pub fn reset(&mut self) {
        self.participants.clear();
        self.endpoints.clear();
    }
}

/// Per-second rates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rates {
    pub pps: usize,
    pub bps: usize,
}

/// Computes per-second rates from successive [`Stats`] snapshots.
#[derive(Clone, Debug, Default)]
pub struct RateTracker {
    prev_packets: usize,
    prev_bytes: usize,
    prev_ms: u64,
    initialized: bool,
}

impl RateTracker {
    /// Rates since the previous sample; the first sample returns zeros.
    pub fn sample(&mut self, stats: &Stats, now_ms: u64) -> Rates {
        let dt = now_ms.saturating_sub(self.prev_ms) as usize;
        if !self.initialized || dt == 0 {
            self.prev_packets = stats.packets;
            self.prev_bytes = stats.bytes;
            self.prev_ms = now_ms;
            self.initialized = true;
            return Rates::default();
        }
        let pps = (stats.packets.saturating_sub(self.prev_packets)) * 1000 / dt;
        let bps = (stats.bytes.saturating_sub(self.prev_bytes)) * 1000 / dt;
        self.prev_packets = stats.packets;
        self.prev_bytes = stats.bytes;
        self.prev_ms = now_ms;
        Rates { pps, bps }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(class: Class, size: usize, topic: Option<&str>) -> Record {
        Record {
            elapsed_ms: 0,
            class,
            src_addr: "[::1]:7400".to_string(),
            guid_prefix: None,
            entity_id: None,
            dest_port: 7400,
            size,
            topic: topic.map(str::to_string),
            submessages: String::new(),
            hex: vec![0],
            decoded: String::new(),
            frames: Vec::new(),
        }
    }

    #[test]
    fn stats_accumulate_rtps_only() {
        let mut s = Stats::default();
        s.observe(&record(Class::Udp, 10, None));
        s.observe(&record(Class::Known, 5, Some("HelloWorldTopic")));
        assert_eq!(s.packets, 2);
        assert_eq!(s.bytes, 15);
        assert_eq!(s.rtps, 1);
    }

    #[test]
    fn topic_counts_include_known_unregistered_and_unidentified() {
        let mut c = TopicCounts::default();
        assert!(c
            .rows()
            .iter()
            .any(|r| r.name == "HelloWorldTopic" && r.known && r.packets == 0));
        c.observe(&record(Class::Known, 1, Some("HelloWorldTopic")));
        c.observe(&record(Class::Unregistered, 1, Some("WeatherTopic")));
        c.observe(&record(Class::Udp, 1, None));
        c.observe(&record(Class::Rtps, 1, None));
        let rows = c.rows();
        assert!(rows
            .iter()
            .any(|r| r.name == "HelloWorldTopic" && r.known && r.packets == 1));
        assert!(rows
            .iter()
            .any(|r| r.name == "WeatherTopic" && !r.known && r.packets == 1));
        assert!(rows
            .iter()
            .any(|r| r.name == "unidentified" && !r.known && r.packets == 2));
    }

    #[test]
    fn rate_tracker_rates() {
        let mut rt = RateTracker::default();
        assert_eq!(rt.sample(&Stats::default(), 0), Rates::default());
        let stats = Stats {
            packets: 100,
            bytes: 1000,
            rtps: 0,
        };
        assert_eq!(rt.sample(&stats, 1000), Rates { pps: 100, bps: 1000 });
    }

    #[test]
    fn hex_string_is_lowercase() {
        let mut r = record(Class::Udp, 3, None);
        r.hex = vec![0x0a, 0xbc, 0xde];
        assert_eq!(r.hex_string(), "0a bc de");
    }

    #[test]
    fn hex_string_notes_truncation() {
        let mut r = record(Class::Udp, HEX_PREVIEW + 1, None);
        r.hex = vec![0xaa; HEX_PREVIEW + 1];
        assert!(r.hex_string().ends_with("(+1 bytes)"));
    }

    #[test]
    fn hex_string_wraps_lines() {
        let mut r = record(Class::Udp, 0, None);
        r.hex = (0..17u8).collect();
        let s = r.hex_string();
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].split(' ').count(), HEX_BYTES_PER_LINE);
        assert_eq!(lines[1].split(' ').count(), 1);
    }

    fn frame(id: u8, kind: &str, writer: &str, seq: &str, len: usize) -> Frame {
        Frame {
            id,
            kind: kind.into(),
            writer: writer.into(),
            reader: String::new(),
            seq: seq.into(),
            len,
            announcement: None,
        }
    }

    #[test]
    fn submessage_list_lists_kinds() {
        let mut r = record(Class::Rtps, 0, None);
        assert_eq!(r.submessage_list(), "");
        r.frames = vec![
            frame(0x15, "DATA", "000003c2", "42", 20),
            frame(0x15, "DATA", "000003c2", "43", 20),
            frame(0x07, "HEARTBEAT", "000003c2", "1..5", 28),
            frame(0x09, "INFO_TS", "", "", 8),
        ];
        assert_eq!(r.submessage_list(), "DATA, DATA, HEARTBEAT, INFO_TS");
    }

    #[test]
    fn discovery_learns_endpoints_and_attributes_data() {
        let mut d = Discovery::default();

        // An SEDP endpoint announcement carried by a DATA submessage.
        let mut ann = record(Class::Rtps, 0, None);
        ann.frames.push(Frame {
            announcement: Some(Announcement::Endpoint(Endpoint {
                guid: "aaaaaaaaaaaaaaaaaaaaaaaa00000102".into(),
                prefix: "aaaaaaaaaaaaaaaaaaaaaaaa".into(),
                kind: EndpointKind::Writer,
                topic: "Throughput".into(),
                type_name: "Throughput".into(),
            })),
            ..frame(0x15, "DATA", "000003c2", "1", 20)
        });
        ann.frames.push(Frame {
            announcement: Some(Announcement::Participant {
                prefix: "aaaaaaaaaaaaaaaaaaaaaaaa".into(),
            }),
            ..frame(0x15, "DATA", "000001c1", "1", 20)
        });
        d.observe_and_attr(&mut ann);
        assert_eq!(d.participants(), 1);
        assert_eq!(d.endpoints(), 1);
        assert!(ann.topic.is_none()); // the announcement is not itself data

        // Data from the announced writer is attributed to its topic.
        let mut data = record(Class::Rtps, 0, None);
        data.guid_prefix = Some("aaaaaaaaaaaaaaaaaaaaaaaa".into());
        data.entity_id = Some("00000102".into());
        d.observe_and_attr(&mut data);
        assert_eq!(data.topic.as_deref(), Some("Throughput"));

        // Data from an unknown writer stays untitled.
        let mut other = record(Class::Rtps, 0, None);
        other.guid_prefix = Some("bbbbbbbbbbbbbbbbbbbbbbbb".into());
        other.entity_id = Some("00000102".into());
        d.observe_and_attr(&mut other);
        assert!(other.topic.is_none());

        // Re-announcing the same endpoint does not double-count.
        d.observe_and_attr(&mut ann.clone());
        assert_eq!(d.participants(), 1);
        assert_eq!(d.endpoints(), 1);

        d.reset();
        assert_eq!(d.participants(), 0);
        assert_eq!(d.endpoints(), 0);
    }

    #[test]
    fn writer_guid_needs_prefix_and_entity() {
        let mut r = record(Class::Rtps, 0, None);
        assert_eq!(r.writer_guid(), None);
        r.guid_prefix = Some("111111111111111111111111".into());
        assert_eq!(r.writer_guid(), None);
        r.entity_id = Some("000003c2".into());
        assert_eq!(
            r.writer_guid().as_deref(),
            Some("111111111111111111111111000003c2")
        );
    }
}

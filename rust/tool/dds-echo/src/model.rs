//! The `dds-echo` data model: records, statistics, and the topic counts.
//!
//! UI-agnostic (no Slint types) so it is unit-testable.

use std::collections::BTreeMap;

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

impl Class {
    pub fn label(self) -> &'static str {
        match self {
            Class::Udp => "UDP",
            Class::Rtps => "RTPS",
            Class::Unregistered => "unregistered",
            Class::Known => "known",
        }
    }
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
    pub summary: String,
    pub hex: Vec<u8>,
    /// Rendered fields for a decoded [`Class::Known`] message; empty otherwise.
    pub decoded: String,
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
            summary: String::new(),
            hex: vec![0],
            decoded: String::new(),
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
}

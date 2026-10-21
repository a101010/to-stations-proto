//! The `dds-echo` data model: records, statistics, and the simulated feed.
//!
//! UI-agnostic (no Slint types) so it is unit-testable and the real capture
//! backend can plug in unchanged.

use std::collections::BTreeMap;

/// The datagram table cap.
pub const MAX_RECORDS: usize = 1000;
/// Bytes shown in the detail pane's hex view.
pub const HEX_PREVIEW: usize = 512;
/// Known topics, hardcoded for the mockup; the capture/SEDP stories read
/// `contracts/topics.toml`.
pub const KNOWN_TOPICS: &[&str] = &["HelloWorldTopic"];

/// How much is understood about a message.
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
    /// `MM:SS.mmm` since the feed started.
    pub fn time_label(&self) -> String {
        let ms = self.elapsed_ms;
        format!("{:02}:{:02}.{:03}", ms / 60_000, (ms / 1000) % 60, ms % 1000)
    }

    /// Lowercase hex pairs, first [`HEX_PREVIEW`] bytes.
    pub fn hex_string(&self) -> String {
        let n = self.hex.len().min(HEX_PREVIEW);
        let mut out = String::with_capacity(n * 3 + 1);
        for byte in &self.hex[..n] {
            out.push_str(&format!("{byte:02x} "));
        }
        if self.hex.len() > n {
            out.push('…');
        } else {
            out.pop();
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

/// Per-topic packet counts (known and unregistered topics).
#[derive(Clone, Debug, Default)]
pub struct TopicCounts {
    counts: BTreeMap<String, usize>,
}

impl TopicCounts {
    pub fn observe(&mut self, r: &Record) {
        if let Some(topic) = &r.topic {
            *self.counts.entry(topic.clone()).or_insert(0) += 1;
        }
    }

    /// Known topics (0 if unseen) then unregistered topics seen, sorted.
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
        rows.sort_by(|a, b| b.known.cmp(&a.known).then_with(|| a.name.cmp(&b.name)));
        rows
    }

    pub fn reset(&mut self) {
        self.counts.clear();
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

/// Deterministic xorshift64* PRNG, so the feed needs no dependency.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }

    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        if hi <= lo {
            lo
        } else {
            lo + self.below(hi - lo + 1)
        }
    }
}

/// Lowercase hex, no separators.
fn hex_str(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

const UNREGISTERED_TOPICS: &[&str] = &["WeatherTopic", "LogTopic", "NavTopic"];

/// Emits a mix of unknown UDP, unknown RTPS, unregistered RTPS, and known-topic
/// messages.
#[derive(Clone, Debug)]
pub struct SimFeed {
    rng: Rng,
    seq: u64,
}

impl SimFeed {
    pub fn new(seed: u64) -> Self {
        SimFeed {
            rng: Rng::new(seed),
            seq: 0,
        }
    }

    /// Records for one tick: 1..=4.
    pub fn burst(&mut self) -> usize {
        (1 + self.rng.below(4)) as usize
    }

    pub fn next(&mut self, now_ms: u64) -> Record {
        self.seq += 1;
        match self.rng.below(100) {
            0..=14 => self.udp(now_ms),
            15..=39 => self.rtps(now_ms),
            40..=59 => self.unregistered(now_ms),
            _ => self.known(now_ms),
        }
    }

    fn src_addr(&mut self) -> String {
        format!("[::1]:{}", 7400 + self.rng.below(4))
    }

    fn guid_prefix(&mut self) -> [u8; 12] {
        let mut guid = [0u8; 12];
        for byte in &mut guid {
            *byte = self.rng.below(256) as u8;
        }
        guid
    }

    fn entity_id(&mut self) -> [u8; 4] {
        let mut id = [0u8; 4];
        for byte in &mut id {
            *byte = self.rng.below(256) as u8;
        }
        id
    }

    fn rtps_header(&mut self, guid: &[u8; 12]) -> Vec<u8> {
        let mut header = Vec::with_capacity(20);
        header.extend_from_slice(b"RTPS");
        header.extend_from_slice(&[2, 4, 0x01, 0x0f]);
        header.extend_from_slice(guid);
        header
    }

    fn udp(&mut self, now_ms: u64) -> Record {
        let len = self.rng.range(8, 200) as usize;
        let mut hex = Vec::with_capacity(len);
        hex.push(0x00); // never the RTPS magic
        while hex.len() < len {
            hex.push(self.rng.below(256) as u8);
        }
        let src_addr = self.src_addr();
        Record {
            elapsed_ms: now_ms,
            class: Class::Udp,
            src_addr,
            guid_prefix: None,
            entity_id: None,
            dest_port: 7400,
            size: hex.len(),
            topic: None,
            summary: "UDP".to_string(),
            hex,
            decoded: String::new(),
        }
    }

    fn rtps(&mut self, now_ms: u64) -> Record {
        let guid = self.guid_prefix();
        let len = self.rng.range(30, 300) as usize;
        let mut hex = self.rtps_header(&guid);
        while hex.len() < len {
            hex.push(self.rng.below(256) as u8);
        }
        let src_addr = self.src_addr();
        Record {
            elapsed_ms: now_ms,
            class: Class::Rtps,
            src_addr,
            guid_prefix: Some(hex_str(&guid)),
            entity_id: None,
            dest_port: 7400,
            size: hex.len(),
            topic: None,
            summary: "RTPS".to_string(),
            hex,
            decoded: String::new(),
        }
    }

    fn unregistered(&mut self, now_ms: u64) -> Record {
        let guid = self.guid_prefix();
        let entity = self.entity_id();
        let topic = UNREGISTERED_TOPICS[self.rng.below(UNREGISTERED_TOPICS.len() as u64) as usize];
        let mut hex = self.rtps_header(&guid);
        hex.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        for _ in 0..self.rng.range(8, 40) {
            hex.push(self.rng.below(256) as u8);
        }
        let src_addr = self.src_addr();
        Record {
            elapsed_ms: now_ms,
            class: Class::Unregistered,
            src_addr,
            guid_prefix: Some(hex_str(&guid)),
            entity_id: Some(hex_str(&entity)),
            dest_port: 7401,
            size: hex.len(),
            topic: Some(topic.to_string()),
            summary: "RTPS DATA".to_string(),
            hex,
            decoded: String::new(),
        }
    }

    fn known(&mut self, now_ms: u64) -> Record {
        let guid = self.guid_prefix();
        let entity = self.entity_id();
        let mut hex = self.rtps_header(&guid);
        hex.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        for _ in 0..24 {
            hex.push(self.rng.below(256) as u8);
        }
        let src_addr = self.src_addr();
        Record {
            elapsed_ms: now_ms,
            class: Class::Known,
            src_addr,
            guid_prefix: Some(hex_str(&guid)),
            entity_id: Some(hex_str(&entity)),
            dest_port: 7401,
            size: hex.len(),
            topic: Some("HelloWorldTopic".to_string()),
            summary: "Hello, world!".to_string(),
            hex,
            decoded: "greeting: \"Hello, world!\"\nattitude: (0.0, 0.0, 0.0, 1.0)".to_string(),
        }
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
    fn feed_yields_all_classes() {
        let mut feed = SimFeed::new(1);
        let mut seen = std::collections::HashSet::new();
        for i in 0..200 {
            let r = feed.next(i);
            assert!(!r.hex.is_empty());
            if r.class == Class::Udp {
                assert!(!r.hex.starts_with(b"RTPS"));
                assert!(r.topic.is_none());
            } else {
                assert!(r.hex.starts_with(b"RTPS"), "{:?}", r.class);
            }
            if r.class == Class::Known {
                assert_eq!(r.topic.as_deref(), Some("HelloWorldTopic"));
                assert!(!r.decoded.is_empty());
            }
            if r.class == Class::Unregistered {
                let topic = r.topic.as_deref().unwrap();
                assert!(!KNOWN_TOPICS.contains(&topic));
                assert!(r.entity_id.is_some());
            }
            seen.insert(r.class.label());
        }
        assert_eq!(seen.len(), 4, "classes seen: {seen:?}");
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
    fn topic_counts_include_known_and_unregistered() {
        let mut c = TopicCounts::default();
        assert!(c
            .rows()
            .iter()
            .any(|r| r.name == "HelloWorldTopic" && r.known && r.packets == 0));
        c.observe(&record(Class::Known, 1, Some("HelloWorldTopic")));
        c.observe(&record(Class::Unregistered, 1, Some("WeatherTopic")));
        let rows = c.rows();
        assert!(rows
            .iter()
            .any(|r| r.name == "HelloWorldTopic" && r.known && r.packets == 1));
        assert!(rows
            .iter()
            .any(|r| r.name == "WeatherTopic" && !r.known && r.packets == 1));
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
    fn rng_is_deterministic() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..10 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }
}

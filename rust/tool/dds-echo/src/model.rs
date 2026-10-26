//! The `dds-echo` data model: records, statistics, and the topic counts.
//!
//! UI-agnostic (no Slint types) so it is unit-testable.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

/// The datagram table cap.
pub const MAX_RECORDS: usize = 1000;
/// Bytes shown in the detail pane's hex view before truncating.
pub const HEX_PREVIEW: usize = 4096;
/// Bytes per line in the pre-wrapped hex view.
pub const HEX_BYTES_PER_LINE: usize = 16;

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
    /// For a DATA submessage, the range of its serialized payload within the
    /// datagram (`record.hex`); `None` for non-DATA or empty payloads.
    pub payload: Option<std::ops::Range<usize>>,
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

    pub fn is_known(&self, known: &KnownTopics) -> bool {
        self.topic.as_deref().is_some_and(|t| known.contains(t))
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

    /// The key used to group records for retention. The topic is used when
    /// known; otherwise the writer GUID, so an unattributed stream still forms
    /// its own group; otherwise the source address.
    pub fn retention_key(&self) -> String {
        if let Some(topic) = self.topic.as_deref().filter(|t| !t.is_empty()) {
            return topic.to_string();
        }
        if let Some(guid) = self.writer_guid() {
            return format!("@writer:{guid}");
        }
        format!("@source:{}", self.src_addr)
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

/// The registry's topics: wire name -> scoped IDL/DDS type name. Built from
/// `contracts/topics.toml`; UI-agnostic so `model` need not depend on the DDS
/// binding.
#[derive(Clone, Debug, Default)]
pub struct KnownTopics {
    by_name: BTreeMap<String, String>,
}

impl KnownTopics {
    pub fn insert(&mut self, name: &str, idl_type: &str) {
        self.by_name.insert(name.to_string(), idl_type.to_string());
    }

    pub fn contains(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }

    /// The scoped IDL/DDS type name registered for `name`.
    pub fn idl_type(&self, name: &str) -> Option<&str> {
        self.by_name.get(name).map(String::as_str)
    }

    /// The registered wire names, sorted.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.by_name.keys().map(String::as_str)
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
    pub fn rows(&self, known: &KnownTopics) -> Vec<TopicRow> {
        let mut rows = Vec::new();
        for name in known.names() {
            rows.push(TopicRow {
                name: name.to_string(),
                known: true,
                packets: self.counts.get(name).copied().unwrap_or(0),
            });
        }
        for (name, packets) in &self.counts {
            if !known.contains(name) {
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

    /// Move one observed record from `old` to `new` after discovery learns its
    /// endpoint. `None` means the `unidentified` bucket.
    pub fn reattribute(&mut self, old: Option<&str>, new: &str) {
        if new.is_empty() {
            return;
        }
        match old {
            Some(name) => {
                let remove = match self.counts.get_mut(name) {
                    Some(count) => {
                        *count = count.saturating_sub(1);
                        *count == 0
                    }
                    None => false,
                };
                if remove {
                    self.counts.remove(name);
                }
            }
            None => self.unidentified = self.unidentified.saturating_sub(1),
        }
        *self.counts.entry(new.to_string()).or_insert(0) += 1;
    }

    pub fn reset(&mut self) {
        self.counts.clear();
        self.unidentified = 0;
    }
}

/// The participants, endpoints, and writer->topic map learned from SPDP/SEDP.
///
/// Accumulated on the UI thread; every item is keyed by GUID so repeated
/// announcements are deduplicated. A passive sniffer only learns endpoints
/// whose SEDP it observes; announcements sent before capture started are not
/// replayed, so their data stays unattributed.
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
                    self.learn_participant(prefix.clone());
                }
                Some(Announcement::Endpoint(endpoint)) => {
                    self.learn_endpoint(endpoint.clone());
                }
                None => {}
            }
        }

        if record.topic.is_none() {
            if let Some(guid) = record.writer_guid() {
                if let Some(topic) = self.topic_for_writer(&guid) {
                    record.topic = Some(topic.to_string());
                }
            }
        }
    }

    /// Learn a participant prefix; returns false when already known.
    pub fn learn_participant(&mut self, prefix: String) -> bool {
        self.participants.insert(prefix)
    }

    /// Learn an endpoint; returns true when it was not already present.
    pub fn learn_endpoint(&mut self, endpoint: Endpoint) -> bool {
        let is_new = !self.endpoints.contains_key(&endpoint.guid);
        self.participants.insert(endpoint.prefix.clone());
        self.endpoints.insert(endpoint.guid.clone(), endpoint);
        is_new
    }

    /// The topic a writer GUID is known to use, when it has one.
    pub fn topic_for_writer(&self, guid: &str) -> Option<&str> {
        self.endpoints
            .get(guid)
            .map(|endpoint| endpoint.topic.as_str())
            .filter(|topic| !topic.is_empty())
    }

    /// The number of distinct participants observed.
    pub fn participants(&self) -> usize {
        self.participants.len()
    }

    /// The number of distinct endpoints observed.
    pub fn endpoints(&self) -> usize {
        self.endpoints.len()
    }
}

/// The set of topic rows the user has selected in the topics pane. An empty
/// selection matches every datagram.
#[derive(Clone, Debug, Default)]
pub struct TopicSelection {
    names: BTreeSet<String>,
}

impl TopicSelection {
    /// Add `name` if absent, else remove it.
    pub fn toggle(&mut self, name: &str) {
        if !self.names.remove(name) {
            self.names.insert(name.to_string());
        }
    }

    pub fn clear(&mut self) {
        self.names.clear();
    }

    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    /// Whether a datagram with `topic` passes the filter. An empty selection
    /// matches everything; a `None`/empty topic is the `unidentified` row.
    pub fn matches(&self, topic: Option<&str>) -> bool {
        if self.names.is_empty() {
            return true;
        }
        let name = topic.filter(|t| !t.is_empty()).unwrap_or("unidentified");
        self.names.contains(name)
    }
}

/// A bounded buffer that evicts from the most frequent key first.
///
/// Records keep their arrival order by id. When a cap is exceeded, the oldest
/// record of the key with the most buffered records is dropped. A key with one
/// buffered record is never chosen while another key has two or more. If every
/// key has one record and a cap is still exceeded, the globally oldest record
/// is dropped; the hard cap wins.
#[derive(Clone, Debug)]
pub struct FrequencyBuffer<T> {
    entries: BTreeMap<u64, Slot<T>>,
    by_key: HashMap<String, VecDeque<u64>>,
    next_id: u64,
    max_records: usize,
    max_bytes: usize,
    bytes: usize,
}

#[derive(Clone, Debug)]
struct Slot<T> {
    key: String,
    size: usize,
    value: T,
}

impl<T> FrequencyBuffer<T> {
    pub fn new(max_records: usize, max_bytes: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            by_key: HashMap::new(),
            next_id: 0,
            max_records,
            max_bytes,
            bytes: 0,
        }
    }

    /// Insert `value`; returns the entries evicted to respect the caps.
    pub fn push(&mut self, key: String, size: usize, value: T) -> Vec<(u64, T)> {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.bytes += size;
        self.by_key.entry(key.clone()).or_default().push_back(id);
        self.entries.insert(
            id,
            Slot {
                key,
                size,
                value,
            },
        );

        let mut evicted = Vec::new();
        while self.over_cap() {
            if let Some((id, value)) = self.evict_one() {
                evicted.push((id, value));
            } else {
                break;
            }
        }
        evicted
    }

    /// Remove and return up to `max` of the oldest records.
    pub fn drain_batch(&mut self, max: usize) -> Vec<(u64, T)> {
        let mut out = Vec::with_capacity(max.min(self.entries.len()));
        while out.len() < max {
            let Some((&id, _)) = self.entries.iter().next() else {
                break;
            };
            if let Some(slot) = self.remove(id) {
                out.push((id, slot));
            } else {
                break;
            }
        }
        out
    }

    /// Retained records in arrival order.
    pub fn iter(&self) -> impl Iterator<Item = (u64, &T)> {
        self.entries.iter().map(|(&id, slot)| (id, &slot.value))
    }

    pub fn find(&self, id: u64) -> Option<&T> {
        self.entries.get(&id).map(|slot| &slot.value)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.by_key.clear();
        self.next_id = 0;
        self.bytes = 0;
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    fn over_cap(&self) -> bool {
        self.entries.len() > self.max_records
            || (self.max_bytes != usize::MAX && self.bytes > self.max_bytes)
    }

    /// Evict the oldest record of the most frequent key; fall back to the
    /// globally oldest record when every key has one buffered record.
    fn evict_one(&mut self) -> Option<(u64, T)> {
        let victim = self
            .by_key
            .iter()
            .filter(|(_, ids)| ids.len() >= 2)
            .max_by_key(|(_, ids)| {
                (
                    ids.len(),
                    Reverse(ids.front().copied().unwrap_or(u64::MAX)),
                )
            })
            .map(|(key, _)| key.clone());

        let id = match victim {
            Some(key) => self.by_key.get_mut(&key)?.pop_front()?,
            None => *self.entries.keys().next()?,
        };
        self.remove(id).map(|value| (id, value))
    }

    /// Remove `id` from both indexes, returning its value.
    fn remove(&mut self, id: u64) -> Option<T> {
        let slot = self.entries.remove(&id)?;
        self.bytes = self.bytes.saturating_sub(slot.size);
        if let Some(ids) = self.by_key.get_mut(&slot.key) {
            if ids.front() == Some(&id) {
                ids.pop_front();
            } else {
                ids.retain(|&x| x != id);
            }
            if ids.is_empty() {
                self.by_key.remove(&slot.key);
            }
        }
        Some(slot.value)
    }
}

/// The display history cap.
pub const HISTORY_MAX_BYTES: usize = 64 * 1024 * 1024;

/// The retained records shown in the table, evicted frequency-first.
#[derive(Clone, Debug)]
pub struct History {
    buffer: FrequencyBuffer<Record>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            buffer: FrequencyBuffer::new(MAX_RECORDS, HISTORY_MAX_BYTES),
        }
    }
}

impl History {
    /// Append `record`; returns the entries evicted to respect the caps.
    pub fn push(&mut self, record: Record) -> Vec<(u64, Record)> {
        let key = record.retention_key();
        let size = record.size;
        self.buffer.push(key, size, record)
    }

    /// Retained records that pass `selection`, in arrival order.
    pub fn matching<'a>(
        &'a self,
        selection: &'a TopicSelection,
    ) -> impl Iterator<Item = (u64, &'a Record)> + 'a {
        self.buffer
            .iter()
            .filter(move |(_, record)| selection.matches(record.topic.as_deref()))
    }

    pub fn find(&self, id: u64) -> Option<&Record> {
        self.buffer.find(id)
    }

    /// Attribute retained records whose writer is now known. Returns the
    /// `(old, new)` topic changes so the caller can adjust `TopicCounts`.
    pub fn reattribute(&mut self, discovery: &Discovery) -> Vec<(Option<String>, String)> {
        let mut changes = Vec::new();
        for slot in self.buffer.entries.values_mut() {
            let record = &mut slot.value;
            if record.topic.is_some() {
                continue;
            }
            let Some(guid) = record.writer_guid() else {
                continue;
            };
            let Some(topic) = discovery.topic_for_writer(&guid) else {
                continue;
            };
            let new_topic = topic.to_string();
            record.topic = Some(new_topic.clone());
            changes.push((None, new_topic));
        }
        changes
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.buffer.len()
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
    fn known_topics_insert_and_lookup() {
        let k = known();
        assert!(k.contains("HelloWorldTopic"));
        assert_eq!(k.idl_type("HelloWorldTopic"), Some("hello_world::HelloWorldTopic"));
        assert_eq!(k.names().collect::<Vec<_>>(), vec!["HelloWorldTopic"]);
        assert!(!k.contains("Nope"));
        assert_eq!(k.idl_type("Nope"), None);
    }

    #[test]
    fn record_is_known_uses_the_registry() {
        let k = known();
        assert!(record(Class::Rtps, 0, Some("HelloWorldTopic")).is_known(&k));
        assert!(!record(Class::Rtps, 0, Some("WeatherTopic")).is_known(&k));
        assert!(!record(Class::Rtps, 0, None).is_known(&k));
    }

    #[test]
    fn topic_counts_include_known_unregistered_and_unidentified() {
        let k = known();
        let mut c = TopicCounts::default();
        assert!(c
            .rows(&k)
            .iter()
            .any(|r| r.name == "HelloWorldTopic" && r.known && r.packets == 0));
        c.observe(&record(Class::Known, 1, Some("HelloWorldTopic")));
        c.observe(&record(Class::Unregistered, 1, Some("WeatherTopic")));
        c.observe(&record(Class::Udp, 1, None));
        c.observe(&record(Class::Rtps, 1, None));
        let rows = c.rows(&k);
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
            payload: None,
        }
    }

    fn known() -> KnownTopics {
        let mut k = KnownTopics::default();
        k.insert("HelloWorldTopic", "hello_world::HelloWorldTopic");
        k
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
    }

    #[test]
    fn topic_selection_toggle_and_match() {
        let mut s = TopicSelection::default();
        assert!(s.matches(None)); // empty selection = all
        assert!(s.matches(Some("A")));

        s.toggle("A");
        assert!(s.contains("A"));
        assert!(s.matches(Some("A")));
        assert!(!s.matches(Some("B")));
        assert!(!s.matches(None));

        s.toggle("unidentified");
        assert!(s.matches(None));
        assert!(s.matches(Some(""))); // empty topic is the unidentified row

        s.toggle("A"); // toggles off
        assert!(!s.contains("A"));
        assert!(s.matches(None));
        assert!(!s.matches(Some("B")));

        s.clear();
        assert!(!s.contains("A"));
        assert!(s.matches(Some("B")));
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

    #[test]
    fn retention_key_prefers_topic_then_writer_then_source() {
        let mut r = record(Class::Rtps, 0, Some("A"));
        assert_eq!(r.retention_key(), "A");
        r.topic = None;
        r.guid_prefix = Some("111111111111111111111111".into());
        r.entity_id = Some("000003c2".into());
        assert_eq!(
            r.retention_key(),
            "@writer:111111111111111111111111000003c2"
        );
        r.guid_prefix = None;
        r.entity_id = None;
        assert_eq!(r.retention_key(), "@source:[::1]:7400");
    }

    #[test]
    fn frequency_buffer_evicts_high_frequency_key_first() {
        let mut buf = FrequencyBuffer::new(4, usize::MAX);
        for i in 0..4 {
            assert!(buf.push("fast".into(), 1, i).is_empty());
        }

        let first = buf.push("rare-a".into(), 1, 100);
        let second = buf.push("rare-b".into(), 1, 200);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].1, 0); // oldest fast record
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].1, 1); // the next fast record
        assert_eq!(buf.len(), 4);
        let values: Vec<i32> = buf.iter().map(|(_, v)| *v).collect();
        assert!(values.contains(&100) && values.contains(&200));
    }

    #[test]
    fn frequency_buffer_keeps_singletons_until_all_keys_are_singletons() {
        let mut buf = FrequencyBuffer::new(3, usize::MAX);
        assert!(buf.push("a".into(), 1, 1).is_empty());
        assert!(buf.push("b".into(), 1, 2).is_empty());
        assert!(buf.push("a".into(), 1, 3).is_empty());

        let evicted = buf.push("c".into(), 1, 4);
        assert_eq!(evicted.len(), 1);
        assert_eq!(evicted[0].1, 1); // oldest a, not the singleton b
        let values: Vec<i32> = buf.iter().map(|(_, v)| *v).collect();
        assert!(values.contains(&2) && values.contains(&3) && values.contains(&4));
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn frequency_buffer_evicts_oldest_when_all_keys_are_singletons() {
        let mut buf = FrequencyBuffer::new(2, usize::MAX);
        buf.push("a".into(), 1, 1);
        buf.push("b".into(), 1, 2);

        let evicted = buf.push("c".into(), 1, 3);
        assert_eq!(evicted.len(), 1);
        assert_eq!(evicted[0].1, 1); // globally oldest
        let values: Vec<i32> = buf.iter().map(|(_, v)| *v).collect();
        assert_eq!(values, vec![2, 3]);
    }

    #[test]
    fn frequency_buffer_byte_cap_evicts_high_frequency_first() {
        let mut buf = FrequencyBuffer::new(100, 35);
        for i in 0..3 {
            buf.push("fast".into(), 10, i); // 30 bytes
        }

        let evicted = buf.push("rare".into(), 10, 99); // 40 > 35
        assert_eq!(evicted.len(), 1);
        assert_eq!(evicted[0].1, 0); // oldest fast
        assert_eq!(buf.bytes(), 30);
        assert!(buf.iter().any(|(_, v)| *v == 99));
    }

    #[test]
    fn history_keeps_a_one_shot_topic_while_trimming_fast_records() {
        let mut history = History::default();
        for _ in 0..MAX_RECORDS {
            assert!(history.push(record(Class::Known, 10, Some("fast"))).is_empty());
        }

        let evicted = history.push(record(Class::Known, 10, Some("rare")));
        assert_eq!(evicted.len(), 1);
        assert_eq!(history.len(), MAX_RECORDS);

        let mut selection = TopicSelection::default();
        selection.toggle("rare");
        assert_eq!(history.matching(&selection).count(), 1);
    }

    #[test]
    fn history_matching_applies_the_topic_filter() {
        let mut history = History::default();
        history.push(record(Class::Known, 0, Some("A")));
        history.push(record(Class::Known, 0, Some("B")));
        history.push(record(Class::Rtps, 0, None));

        let mut selection = TopicSelection::default();
        selection.toggle("A");
        let topics: Vec<Option<String>> = history
            .matching(&selection)
            .map(|(_, r)| r.topic.clone())
            .collect();
        assert_eq!(topics, vec![Some("A".to_string())]);
        assert_eq!(history.len(), 3);
    }

    fn view_endpoint() -> Endpoint {
        Endpoint {
            guid: "aaaaaaaaaaaaaaaaaaaaaaaa00000102".to_string(),
            prefix: "aaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            kind: EndpointKind::Writer,
            topic: "HelloWorldViewTopic".to_string(),
            type_name: "hello_world::HelloWorldViewTopic".to_string(),
        }
    }

    #[test]
    fn discovery_learns_builtin_endpoints_and_looks_up_writers() {
        let mut discovery = Discovery::default();
        assert!(discovery.learn_endpoint(view_endpoint()));
        assert!(!discovery.learn_endpoint(view_endpoint()));
        assert_eq!(discovery.endpoints(), 1);
        assert_eq!(discovery.participants(), 1);
        assert_eq!(
            discovery.topic_for_writer("aaaaaaaaaaaaaaaaaaaaaaaa00000102"),
            Some("HelloWorldViewTopic")
        );
        assert_eq!(discovery.topic_for_writer("missing"), None);
    }

    #[test]
    fn history_reattributes_unattributed_records() {
        let mut history = History::default();
        let mut unattributed = record(Class::Rtps, 0, None);
        unattributed.guid_prefix = Some("aaaaaaaaaaaaaaaaaaaaaaaa".to_string());
        unattributed.entity_id = Some("00000102".to_string());
        history.push(unattributed);

        let mut discovery = Discovery::default();
        discovery.learn_endpoint(view_endpoint());
        let changes = history.reattribute(&discovery);
        assert_eq!(
            changes,
            vec![(None, "HelloWorldViewTopic".to_string())]
        );

        let selection = TopicSelection::default();
        let records: Vec<_> = history.matching(&selection).collect();
        assert_eq!(
            records[0].1.topic.as_deref(),
            Some("HelloWorldViewTopic")
        );
        assert!(history.reattribute(&discovery).is_empty());
    }

    #[test]
    fn topic_counts_reattribute_moves_records_between_buckets() {
        let mut counts = TopicCounts::default();
        let known = KnownTopics::default();

        counts.observe(&record(Class::Rtps, 0, None));
        counts.reattribute(None, "A");
        let rows = counts.rows(&known);
        assert!(rows.iter().any(|r| r.name == "A" && r.packets == 1));
        assert!(!rows.iter().any(|r| r.name == "unidentified"));

        counts.observe(&record(Class::Known, 0, Some("B")));
        counts.reattribute(Some("B"), "C");
        let rows = counts.rows(&known);
        assert!(!rows.iter().any(|r| r.name == "B"));
        assert!(rows.iter().any(|r| r.name == "C" && r.packets == 1));
    }
}

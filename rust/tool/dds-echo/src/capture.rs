//! The UDP capture backend: bind multicast/broadcast/unicast sockets, receive
//! datagrams on background threads, and hand classified [`Record`]s to the UI.
//!
//! The sockets are standalone (no CycloneDDS participant); each datagram is
//! classified as `UDP` or `RTPS` and forwarded over a channel. Later stories
//! parse the RTPS/SEDP payloads.
//!
//! Topic attribution depends on observing SEDP during capture. SEDP is directed
//! to matched readers and is not replayed to a late-joining passive sniffer, so
//! data from endpoints discovered before capture starts stays `unidentified`.
//! The proper fix is the `dds-echo-discovery-attribution` story.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use socket2::{Domain, Protocol, Socket, Type};

use crate::model::{self, Class, Record};
use crate::rtps;
use crate::sedp;

const READ_TIMEOUT: Duration = Duration::from_millis(200);
const BUF_SIZE: usize = 65_535;

/// How an interface is named on the command line / in the toolbar.
#[derive(Clone, Debug)]
pub enum IfSpec {
    Name(String),
    Index(u32),
}

/// What to capture.
#[derive(Clone, Debug)]
pub struct CaptureConfig {
    pub interfaces: Vec<IfSpec>,
    pub groups: Vec<IpAddr>,
    pub ports: Vec<u16>,
    pub broadcast: bool,
}

impl CaptureConfig {
    /// The DDS defaults for `domain`: interface `StationsDDS`, group
    /// `ff02::ffff:239.255.0.1`, ports `7400 + 250*domain` and `+1`.
    pub fn for_domain(domain: u32) -> Self {
        let base = 7400 + 250 * domain;
        CaptureConfig {
            interfaces: vec![IfSpec::Name("StationsDDS".to_string())],
            groups: vec![IpAddr::V6(Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0xffff, 0xefff, 0x0001))],
            ports: vec![base as u16, (base + 1) as u16],
            broadcast: false,
        }
    }

    /// Build a config from the toolbar fields.
    pub fn from_fields(iface: &str, group: &str, ports: &str) -> Result<Self, ConfigError> {
        let interfaces = parse_interfaces(iface);
        if interfaces.is_empty() {
            return Err(ConfigError::BadInterface(iface.to_string()));
        }
        let groups = parse_groups(group)?;
        if groups.is_empty() {
            return Err(ConfigError::BadGroup(group.to_string()));
        }
        let port_list = parse_ports(ports)?;
        if port_list.is_empty() {
            return Err(ConfigError::BadPort(ports.to_string()));
        }
        Ok(CaptureConfig {
            interfaces,
            groups,
            ports: port_list,
            broadcast: false,
        })
    }
}

/// A capture configuration that could not be parsed.
#[derive(Debug)]
pub enum ConfigError {
    BadInterface(String),
    BadGroup(String),
    BadPort(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::BadInterface(s) => write!(f, "bad interface: {s}"),
            ConfigError::BadGroup(s) => write!(f, "bad group: {s}"),
            ConfigError::BadPort(s) => write!(f, "bad port: {s}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Maximum datagrams pending between the capture threads and the UI.
pub const QUEUE_MAX_RECORDS: usize = 4096;
/// Maximum pending datagram bytes between the capture threads and the UI.
pub const QUEUE_MAX_BYTES: usize = 32 * 1024 * 1024;
/// Maximum datagrams the UI processes per tick, keeping the event loop moving.
pub const DRAIN_MAX_PER_TICK: usize = 2048;

struct QueueState {
    buffer: model::FrequencyBuffer<Record>,
    /// Writer GUID -> topic, learned from SEDP so the queue can group data
    /// before the UI attributes it.
    writer_topic: HashMap<String, String>,
}

/// The bounded handoff from the capture threads to the UI.
///
/// Eviction is frequency-first: the oldest record of the most frequent key is
/// dropped, and a key with one buffered record is kept while any other key has
/// two or more. The `dropped` count records evictions the UI never observed.
pub struct CaptureQueue {
    inner: Mutex<QueueState>,
    dropped: AtomicU64,
}

impl Default for CaptureQueue {
    fn default() -> Self {
        Self {
            inner: Mutex::new(QueueState {
                buffer: model::FrequencyBuffer::new(QUEUE_MAX_RECORDS, QUEUE_MAX_BYTES),
                writer_topic: HashMap::new(),
            }),
            dropped: AtomicU64::new(0),
        }
    }
}

impl CaptureQueue {
    pub fn push(&self, record: Record) {
        let mut state = self.lock();
        for frame in &record.frames {
            if let Some(model::Announcement::Endpoint(endpoint)) = &frame.announcement {
                if endpoint.kind == model::EndpointKind::Writer && !endpoint.topic.is_empty() {
                    state
                        .writer_topic
                        .insert(endpoint.guid.clone(), endpoint.topic.clone());
                }
            }
        }
        let key = queue_key(&state, &record);
        let evicted = state.buffer.push(key, record.size, record);
        let dropped = evicted.len() as u64;
        drop(state);
        if dropped > 0 {
            self.dropped.fetch_add(dropped, Ordering::Relaxed);
        }
    }

    /// Remove and return up to `max` of the oldest pending records.
    pub fn drain(&self, max: usize) -> Vec<Record> {
        self.lock()
            .buffer
            .drain_batch(max)
            .into_iter()
            .map(|(_, record)| record)
            .collect()
    }

    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Discard pending records without counting them as dropped and forget the
    /// learned writer topics.
    pub fn clear(&self) {
        let mut state = self.lock();
        state.buffer.clear();
        state.writer_topic.clear();
        drop(state);
        self.dropped.store(0, Ordering::Relaxed);
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.lock().buffer.len()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, QueueState> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn queue_key(state: &QueueState, record: &Record) -> String {
    if let Some(topic) = record.topic.as_deref().filter(|t| !t.is_empty()) {
        return topic.to_string();
    }
    if let Some(guid) = record.writer_guid() {
        if let Some(topic) = state.writer_topic.get(&guid) {
            return topic.clone();
        }
        return format!("@writer:{guid}");
    }
    format!("@source:{}", record.src_addr)
}

pub fn is_rtps(data: &[u8]) -> bool {
    rtps::is_rtps(data)
}

pub fn classify(data: &[u8]) -> Class {
    if is_rtps(data) {
        Class::Rtps
    } else {
        Class::Udp
    }
}

/// Build a record from a received datagram.
pub fn record_from(data: &[u8], src: SocketAddr, local_port: u16, elapsed_ms: u64) -> Record {
    let class = classify(data);
    let mut record = Record {
        elapsed_ms,
        class,
        src_addr: src.to_string(),
        guid_prefix: None,
        entity_id: None,
        dest_port: local_port,
        size: data.len(),
        topic: None,
        submessages: if class == Class::Rtps { "RTPS" } else { "UDP" }.to_string(),
        hex: data.to_vec(),
        frames: Vec::new(),
    };
    if class == Class::Rtps {
        let parsed = rtps::parse(data);
        record.guid_prefix = parsed.src_guid_prefix.map(|p| rtps::hex(&p));
        record.frames = parsed.frames.iter().map(frame_from).collect();
        record.entity_id = parsed
            .frames
            .iter()
            .rev()
            .find_map(|f| f.writer)
            .map(|e| rtps::hex(&e));

        // Decode each submessage's SPDP/SEDP parameter-list payload, attaching
        // the announcement to that submessage.
        for (i, frame) in parsed.frames.iter().enumerate() {
            let Some(range) = &frame.payload else {
                continue;
            };
            let Some(found) = data.get(range.clone()).and_then(sedp::parse) else {
                continue;
            };
            record.frames[i].announcement = Some(match found {
                sedp::Discovered::Participant { guid_prefix } => model::Announcement::Participant {
                    prefix: rtps::hex(&guid_prefix),
                },
                sedp::Discovered::Endpoint {
                    guid,
                    kind,
                    topic,
                    type_name,
                } => model::Announcement::Endpoint(model::Endpoint {
                    guid: rtps::hex(&guid),
                    prefix: rtps::hex(&guid[..12]),
                    kind: kind.into(),
                    topic: topic.unwrap_or_default(),
                    type_name: type_name.unwrap_or_default(),
                }),
            });
        }

        if !record.frames.is_empty() {
            record.submessages = record.submessage_list();
        }
    }
    record
}

impl From<sedp::EndpointKind> for model::EndpointKind {
    fn from(kind: sedp::EndpointKind) -> Self {
        match kind {
            sedp::EndpointKind::Writer => model::EndpointKind::Writer,
            sedp::EndpointKind::Reader => model::EndpointKind::Reader,
            sedp::EndpointKind::Other => model::EndpointKind::Other,
        }
    }
}

/// Map a parsed RTPS [`rtps::Frame`] to the display [`model::Frame`].
fn frame_from(f: &rtps::Frame) -> model::Frame {
    let seq = match (f.sn, f.sn_last) {
        (Some(a), Some(b)) => format!("{a}..{b}"),
        (Some(a), None) => format!("{a}"),
        _ => String::new(),
    };
    model::Frame {
        id: f.id,
        kind: if f.kind == "UNKNOWN" {
            format!("0x{:02x}", f.id)
        } else {
            f.kind.to_string()
        },
        writer: f.writer.map(|e| rtps::hex(&e)).unwrap_or_default(),
        reader: f.reader.map(|e| rtps::hex(&e)).unwrap_or_default(),
        seq,
        len: f.len,
        announcement: None,
        payload: f.payload.clone(),
    }
}

/// A running capture: sockets plus one receive thread each.
pub struct Capture {
    stop: Arc<AtomicBool>,
    threads: Vec<std::thread::JoinHandle<()>>,
    // Kept open for the threads' lifetime (and to report bound ports in tests).
    #[allow(dead_code)]
    sockets: Vec<UdpSocket>,
}

impl Capture {
    /// Bind the sockets, join the multicast groups, and start receiving.
    pub fn start(
        config: &CaptureConfig,
        queue: Arc<CaptureQueue>,
    ) -> std::io::Result<Self> {
        let resolved = resolve_interfaces(&config.interfaces);
        let families = families(config);

        let mut sockets = Vec::new();
        for &port in &config.ports {
            for &is_v6 in &families {
                let socket = bind_socket(is_v6, port, config.broadcast)?;
                join_groups(&socket, is_v6, &config.groups, &resolved);
                sockets.push(socket);
            }
        }

        let stop = Arc::new(AtomicBool::new(false));
        let mut threads = Vec::with_capacity(sockets.len());
        for socket in &sockets {
            let udp = socket.try_clone()?;
            let port = socket.local_addr().map(|a| a.port()).unwrap_or(0);
            let queue = queue.clone();
            let stop = stop.clone();
            threads.push(std::thread::spawn(move || recv_loop(udp, port, queue, stop)));
        }

        Ok(Capture {
            stop,
            threads,
            sockets,
        })
    }

    /// The local ports the sockets are bound to.
    #[allow(dead_code)] // used by the loopback test
    pub fn local_ports(&self) -> Vec<u16> {
        self.sockets
            .iter()
            .filter_map(|s| s.local_addr().ok().map(|a| a.port()))
            .collect()
    }

    /// Signal the threads to stop and wait for them.
    pub fn stop(self) {
        self.stop.store(true, Ordering::Relaxed);
        for thread in self.threads {
            let _ = thread.join();
        }
    }
}

fn recv_loop(udp: UdpSocket, port: u16, queue: Arc<CaptureQueue>, stop: Arc<AtomicBool>) {
    let epoch = Instant::now();
    let mut buf = vec![0u8; BUF_SIZE];
    loop {
        match udp.recv_from(&mut buf) {
            Ok((n, src)) => {
                let elapsed = epoch.elapsed().as_millis() as u64;
                queue.push(record_from(&buf[..n], src, port, elapsed));
            }
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
            }
            Err(e) => {
                eprintln!("dds-echo capture: {e}");
                break;
            }
        }
    }
}

/// Which socket families to bind.
fn families(config: &CaptureConfig) -> Vec<bool> {
    let mut families = Vec::new();
    if config.groups.iter().any(IpAddr::is_ipv6) {
        families.push(true);
    }
    if config.groups.iter().any(IpAddr::is_ipv4) {
        families.push(false);
    }
    if config.broadcast && !families.contains(&false) {
        families.push(false);
    }
    if families.is_empty() {
        families.push(false);
        families.push(true);
    }
    families
}

fn bind_socket(is_v6: bool, port: u16, broadcast: bool) -> std::io::Result<UdpSocket> {
    let domain = if is_v6 { Domain::IPV6 } else { Domain::IPV4 };
    let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    if is_v6 {
        socket.set_only_v6(true)?;
    }
    let addr = if is_v6 {
        SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port)
    } else {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port)
    };
    socket.bind(&addr.into())?;
    if broadcast {
        socket.set_broadcast(true)?;
    }
    let udp: UdpSocket = socket.into();
    udp.set_read_timeout(Some(READ_TIMEOUT))?;
    Ok(udp)
}

#[derive(Clone, Copy, Debug, Default)]
struct ResolvedIface {
    v6_index: Option<u32>,
    v4_addr: Option<Ipv4Addr>,
}

/// The IPv6 interface index for `name` (Windows), via `GetAdaptersAddresses`.
///
/// `if-addrs` drops adapters whose addresses it doesn't consider preferred, and
/// `if_nametoindex` doesn't accept the friendly name, so we query the adapters
/// ourselves with `GAA_FLAG_INCLUDE_ALL_INTERFACES` (which includes the KM-TEST
/// loopback adapter).
#[cfg(windows)]
fn if_index_by_name(name: &str) -> Option<u32> {
    use windows_sys::Win32::Foundation::NO_ERROR;
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GAA_FLAG_INCLUDE_ALL_INTERFACES, GAA_FLAG_INCLUDE_PREFIX,
        IP_ADAPTER_ADDRESSES_LH,
    };
    use windows_sys::Win32::Networking::WinSock::AF_UNSPEC;

    let flags = GAA_FLAG_INCLUDE_ALL_INTERFACES | GAA_FLAG_INCLUDE_PREFIX;
    unsafe {
        let mut size: u32 = 0;
        let _ = GetAdaptersAddresses(
            AF_UNSPEC as u32,
            flags,
            std::ptr::null(),
            std::ptr::null_mut(),
            &mut size,
        );
        if size == 0 {
            return None;
        }
        let mut buffer: Vec<u64> = vec![0; (size as usize).div_ceil(8)];
        if GetAdaptersAddresses(
            AF_UNSPEC as u32,
            flags,
            std::ptr::null(),
            buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH,
            &mut size,
        ) != NO_ERROR
        {
            return None;
        }

        let mut adapter = buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
        while !adapter.is_null() {
            let entry = &*adapter;
            if !entry.FriendlyName.is_null() {
                let mut len = 0usize;
                while *entry.FriendlyName.add(len) != 0 {
                    len += 1;
                }
                let friendly =
                    String::from_utf16_lossy(std::slice::from_raw_parts(entry.FriendlyName, len));
                if friendly == name && entry.Ipv6IfIndex != 0 {
                    return Some(entry.Ipv6IfIndex);
                }
            }
            adapter = entry.Next;
        }
    }
    None
}

#[cfg(not(windows))]
fn if_index_by_name(_name: &str) -> Option<u32> {
    None
}

fn resolve_interfaces(specs: &[IfSpec]) -> Vec<ResolvedIface> {
    let list = if_addrs::get_if_addrs().unwrap_or_default();
    specs
        .iter()
        .map(|spec| match spec {
            IfSpec::Index(index) => ResolvedIface {
                v6_index: Some(*index),
                v4_addr: None,
            },
            IfSpec::Name(name) => {
                // `if_nametoindex` resolves the scope id even for adapters that
                // `if-addrs` drops (e.g. the KM-TEST loopback).
                let mut resolved = ResolvedIface {
                    v6_index: if_index_by_name(name),
                    v4_addr: None,
                };
                for iface in &list {
                    if &iface.name != name {
                        continue;
                    }
                    match &iface.addr {
                        if_addrs::IfAddr::V6(_) => {
                            if resolved.v6_index.is_none() {
                                resolved.v6_index = iface.index;
                            }
                        }
                        if_addrs::IfAddr::V4(v4) => resolved.v4_addr = Some(v4.ip),
                    }
                }
                resolved
            }
        })
        .collect()
}

fn join_groups(udp: &UdpSocket, is_v6: bool, groups: &[IpAddr], resolved: &[ResolvedIface]) {
    for group in groups {
        match group {
            IpAddr::V6(g) if is_v6 => {
                for iface in resolved {
                    if let Some(index) = iface.v6_index {
                        if let Err(e) = udp.join_multicast_v6(g, index) {
                            eprintln!("dds-echo: join {g} on interface {index} failed: {e}");
                        }
                    }
                }
            }
            IpAddr::V4(g) if !is_v6 => {
                for iface in resolved {
                    if let Some(addr) = iface.v4_addr {
                        if let Err(e) = udp.join_multicast_v4(g, &addr) {
                            eprintln!("dds-echo: join {g} on {addr} failed: {e}");
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn parse_interfaces(field: &str) -> Vec<IfSpec> {
    field
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| match s.parse::<u32>() {
            Ok(index) => IfSpec::Index(index),
            Err(_) => IfSpec::Name(s.to_string()),
        })
        .collect()
}

fn parse_groups(field: &str) -> Result<Vec<IpAddr>, ConfigError> {
    field
        .split([',', ' '])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<IpAddr>().map_err(|_| ConfigError::BadGroup(s.to_string())))
        .collect()
}

fn parse_ports(field: &str) -> Result<Vec<u16>, ConfigError> {
    field
        .split([',', ' '])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<u16>().map_err(|_| ConfigError::BadPort(s.to_string())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn next_record(queue: &CaptureQueue, timeout: Duration) -> Record {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(record) = queue.drain(1).into_iter().next() {
                return record;
            }
            assert!(Instant::now() < deadline, "a record arrives");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn detects_rtps() {
        assert!(is_rtps(b"RTPS\x02\x04"));
        assert!(!is_rtps(b"\x00\x01\x02\x03"));
        assert_eq!(classify(b"RTPS...."), Class::Rtps);
        assert_eq!(classify(b"xxxx"), Class::Udp);
    }

    #[test]
    fn multicast_receives_on_adapter() {
        let index = if_index_by_name("StationsDDS").expect("StationsDDS adapter index");
        let group: Ipv6Addr = "ff02::ffff:239.255.0.1".parse().unwrap();
        let config = CaptureConfig {
            interfaces: vec![IfSpec::Name("StationsDDS".to_string())],
            groups: vec![IpAddr::V6(group)],
            ports: vec![7400],
            broadcast: false,
        };
        let queue = Arc::new(CaptureQueue::default());
        let capture = Capture::start(&config, queue.clone()).expect("start capture");

        let sender = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP)).unwrap();
        sender.set_multicast_if_v6(index).unwrap();
        let sender: UdpSocket = sender.into();
        let dest = SocketAddr::V6(std::net::SocketAddrV6::new(group, 7400, 0, index));
        sender.send_to(b"RTPS\x02\x04hello", dest).unwrap();

        let received = next_record(&queue, Duration::from_secs(3));
        assert_eq!(received.class, Class::Rtps);
        capture.stop();
    }

    #[test]
    fn for_domain_defaults() {
        let c = CaptureConfig::for_domain(0);
        assert_eq!(c.ports, vec![7400, 7401]);
        assert_eq!(
            c.groups,
            vec!["ff02::ffff:239.255.0.1".parse::<IpAddr>().unwrap()]
        );
        assert_eq!(CaptureConfig::for_domain(1).ports, vec![7650, 7651]);
    }

    #[test]
    fn parses_fields() {
        let c = CaptureConfig::from_fields("StationsDDS", "ff02::1, 239.1.1.1", "7400, 7401").unwrap();
        assert_eq!(c.ports, vec![7400, 7401]);
        assert_eq!(c.groups.len(), 2);
        assert!(CaptureConfig::from_fields("", "239.1.1.1", "7400").is_err());
        assert!(CaptureConfig::from_fields("StationsDDS", "not-an-ip", "7400").is_err());
        assert!(CaptureConfig::from_fields("StationsDDS", "239.1.1.1", "99999").is_err());
    }

    #[test]
    fn record_from_classifies() {
        let src: SocketAddr = "127.0.0.1:1234".parse().unwrap();
        let r = record_from(b"RTPS\x02\x04abcdef", src, 7400, 42);
        assert_eq!(r.class, Class::Rtps);
        assert_eq!(r.size, 12);
        assert_eq!(r.hex, b"RTPS\x02\x04abcdef");
        assert_eq!(r.submessages, "RTPS");
        assert!(r.frames.is_empty());
        assert!(r.guid_prefix.is_none());
        assert_eq!(r.src_addr, "127.0.0.1:1234");
        assert_eq!(record_from(b"junk", src, 7400, 0).class, Class::Udp);
    }

    #[test]
    fn record_from_parses_frames() {
        let src: SocketAddr = "127.0.0.1:1234".parse().unwrap();
        let mut d = Vec::new();
        d.extend_from_slice(b"RTPS");
        d.extend_from_slice(&[2, 4]);
        d.extend_from_slice(&[0x01, 0x0f]);
        d.extend_from_slice(&[0x11; 12]); // guid prefix
        d.extend_from_slice(&[0x15, 0x05]); // DATA, E|D
        d.extend_from_slice(&20u16.to_le_bytes());
        d.extend_from_slice(&0u16.to_le_bytes()); // extraFlags
        d.extend_from_slice(&16u16.to_le_bytes()); // octetsToInlineQos
        d.extend_from_slice(&[0, 0, 0, 0]); // reader
        d.extend_from_slice(&[0, 0, 0x03, 0xc2]); // writer
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&7u32.to_le_bytes()); // sn = 7
        let r = record_from(&d, src, 7400, 5);
        assert_eq!(r.class, Class::Rtps);
        assert_eq!(r.guid_prefix.as_deref(), Some("111111111111111111111111"));
        assert_eq!(r.entity_id.as_deref(), Some("000003c2"));
        assert_eq!(r.frames.len(), 1);
        assert_eq!(r.frames[0].kind, "DATA");
        assert_eq!(r.frames[0].seq, "7");
        assert_eq!(r.submessages, "DATA");
    }

    /// A parameter with its length padded to a multiple of 4, little-endian.
    fn param(pid: u16, value: &[u8]) -> Vec<u8> {
        let padded = (value.len() + 3) & !3;
        let mut v = Vec::new();
        v.extend_from_slice(&pid.to_le_bytes());
        v.extend_from_slice(&(padded as u16).to_le_bytes());
        v.extend_from_slice(value);
        v.resize(v.len() + (padded - value.len()), 0);
        v
    }

    fn cdr_string(s: &str) -> Vec<u8> {
        let n = s.len() as u32 + 1;
        let mut v = Vec::new();
        v.extend_from_slice(&n.to_le_bytes());
        v.extend_from_slice(s.as_bytes());
        v.push(0);
        v
    }

    /// An RTPS datagram with one DATA submessage carrying `payload`.
    fn rtps_data(prefix: [u8; 12], payload: &[u8]) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(b"RTPS");
        d.extend_from_slice(&[2, 4]);
        d.extend_from_slice(&[0x01, 0x0f]);
        d.extend_from_slice(&prefix);
        d.extend_from_slice(&[0x15, 0x05]); // DATA, E|D
        d.extend_from_slice(&((20 + payload.len()) as u16).to_le_bytes());
        d.extend_from_slice(&0u16.to_le_bytes()); // extraFlags
        d.extend_from_slice(&16u16.to_le_bytes()); // octetsToInlineQos
        d.extend_from_slice(&[0, 0, 0, 0]); // reader
        d.extend_from_slice(&[0, 0, 0x03, 0xc2]); // writer (SEDP publications)
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&1u32.to_le_bytes()); // writerSN
        d.extend_from_slice(payload);
        d
    }

    #[test]
    fn record_from_parses_sedp_endpoint() {
        let src: SocketAddr = "127.0.0.1:1234".parse().unwrap();
        let mut guid = [0u8; 16];
        guid[0..12].copy_from_slice(&[0xaa; 12]);
        guid[12..16].copy_from_slice(&[0, 0, 0x01, 0x02]); // writer

        let mut pl = vec![0x00, 0x03, 0x00, 0x00]; // PL_CDR_LE
        pl.extend(param(0x005a, &guid));
        pl.extend(param(0x0005, &cdr_string("HelloWorldTopic")));
        pl.extend(param(0x0007, &cdr_string("hello_world::HelloWorldTopic")));
        pl.extend(param(0x0001, &[])); // sentinel

        let d = rtps_data([0x11; 12], &pl);
        let r = record_from(&d, src, 7400, 5);
        assert_eq!(r.class, Class::Rtps);
        assert_eq!(r.frames.len(), 1);
        match r.frames[0].announcement.as_ref().expect("announcement") {
            model::Announcement::Endpoint(e) => {
                assert_eq!(e.guid, "aaaaaaaaaaaaaaaaaaaaaaaa00000102");
                assert_eq!(e.prefix, "aaaaaaaaaaaaaaaaaaaaaaaa");
                assert_eq!(e.kind, model::EndpointKind::Writer);
                assert_eq!(e.topic, "HelloWorldTopic");
                assert_eq!(e.type_name, "hello_world::HelloWorldTopic");
            }
            other => panic!("expected endpoint, got {other:?}"),
        }
    }

    #[test]
    fn record_from_parses_spdp_participant() {
        let src: SocketAddr = "127.0.0.1:1234".parse().unwrap();
        let mut pl = vec![0x00, 0x03, 0x00, 0x00]; // PL_CDR_LE
        pl.extend(param(0x0050, &[0xcc; 12]));
        pl.extend(param(0x0001, &[])); // sentinel

        let d = rtps_data([0x11; 12], &pl);
        let r = record_from(&d, src, 7400, 5);
        match r.frames[0].announcement.as_ref().expect("announcement") {
            model::Announcement::Participant { prefix } => {
                assert_eq!(prefix, "cccccccccccccccccccccccc");
            }
            other => panic!("expected participant, got {other:?}"),
        }
    }

    #[test]
    fn receives_a_datagram_on_a_loopback_port() {
        // Plain unicast bind (no groups) on an ephemeral port; send to it.
        let config = CaptureConfig {
            interfaces: Vec::new(),
            groups: Vec::new(),
            ports: vec![0],
            broadcast: false,
        };
        let queue = Arc::new(CaptureQueue::default());
        let capture = Capture::start(&config, queue.clone()).expect("start capture");
        let port = capture.local_ports()[0];

        let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
        let mut d = Vec::new();
        d.extend_from_slice(b"RTPS\x02\x04\x01\x0f");
        d.extend_from_slice(&[0x22; 12]); // guid prefix
        d.extend_from_slice(&[0x15, 0x05]); // DATA, E|D
        d.extend_from_slice(&20u16.to_le_bytes());
        d.extend_from_slice(&0u16.to_le_bytes()); // extraFlags
        d.extend_from_slice(&16u16.to_le_bytes()); // octetsToInlineQos
        d.extend_from_slice(&[0, 0, 0, 0]); // reader
        d.extend_from_slice(&[0, 0, 0x03, 0xc2]); // writer
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&9u32.to_le_bytes()); // sn = 9
        sender.send_to(&d, ("127.0.0.1", port)).unwrap();

        let received = next_record(&queue, Duration::from_secs(3));
        assert_eq!(received.class, Class::Rtps);
        assert_eq!(received.size, d.len());
        assert_eq!(received.hex, d);
        assert_eq!(received.guid_prefix.as_deref(), Some("222222222222222222222222"));
        assert_eq!(received.entity_id.as_deref(), Some("000003c2"));
        assert_eq!(received.frames.len(), 1);
        assert_eq!(received.frames[0].kind, "DATA");
        assert_eq!(received.frames[0].seq, "9");

        capture.stop();
    }

    fn queued(topic: Option<&str>, size: usize) -> Record {
        Record {
            elapsed_ms: 0,
            class: Class::Rtps,
            src_addr: "[::1]:7400".to_string(),
            guid_prefix: None,
            entity_id: None,
            dest_port: 7401,
            size,
            topic: topic.map(str::to_string),
            submessages: "DATA".to_string(),
            hex: vec![0; size],
            frames: Vec::new(),
        }
    }

    #[test]
    fn queue_drops_high_frequency_records_before_a_one_shot_topic() {
        let queue = CaptureQueue::default();
        for _ in 0..QUEUE_MAX_RECORDS {
            queue.push(queued(Some("fast"), 1));
        }
        assert_eq!(queue.len(), QUEUE_MAX_RECORDS);
        assert_eq!(queue.dropped(), 0);

        queue.push(queued(Some("rare"), 1));
        assert_eq!(queue.len(), QUEUE_MAX_RECORDS);
        assert_eq!(queue.dropped(), 1);

        let drained = queue.drain(usize::MAX);
        assert!(drained
            .iter()
            .any(|record| record.topic.as_deref() == Some("rare")));
        assert_eq!(
            drained
                .iter()
                .filter(|record| record.topic.as_deref() == Some("fast"))
                .count(),
            QUEUE_MAX_RECORDS - 1
        );
    }

    #[test]
    fn queue_key_uses_learned_topic_then_writer_then_source() {
        let mut state = QueueState {
            buffer: model::FrequencyBuffer::new(QUEUE_MAX_RECORDS, QUEUE_MAX_BYTES),
            writer_topic: std::collections::HashMap::new(),
        };
        state.writer_topic.insert(
            "aaaaaaaaaaaaaaaaaaaaaaaa00000102".to_string(),
            "Learned".to_string(),
        );

        let mut record = queued(None, 1);
        record.guid_prefix = Some("aaaaaaaaaaaaaaaaaaaaaaaa".to_string());
        record.entity_id = Some("00000102".to_string());
        assert_eq!(queue_key(&state, &record), "Learned");

        record.topic = Some("Explicit".to_string());
        assert_eq!(queue_key(&state, &record), "Explicit");

        record.topic = None;
        record.entity_id = Some("00000999".to_string());
        assert_eq!(
            queue_key(&state, &record),
            "@writer:aaaaaaaaaaaaaaaaaaaaaaaa00000999"
        );

        record.guid_prefix = None;
        record.entity_id = None;
        assert_eq!(queue_key(&state, &record), "@source:[::1]:7400");
    }

    #[test]
    fn queue_clear_resets_pending_and_dropped() {
        let queue = CaptureQueue::default();
        for _ in 0..QUEUE_MAX_RECORDS + 1 {
            queue.push(queued(Some("fast"), 1));
        }
        assert_eq!(queue.dropped(), 1);

        queue.clear();
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.dropped(), 0);
    }
}

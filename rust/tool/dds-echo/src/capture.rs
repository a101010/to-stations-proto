//! The UDP capture backend: bind multicast/broadcast/unicast sockets, receive
//! datagrams on background threads, and hand classified [`Record`]s to the UI.
//!
//! The sockets are standalone (no CycloneDDS participant); each datagram is
//! classified as `UDP` or `RTPS` and forwarded over a channel. Later stories
//! parse the RTPS/SEDP payloads.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, Instant};

use socket2::{Domain, Protocol, Socket, Type};

use crate::model::{Class, Record};

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

pub fn is_rtps(data: &[u8]) -> bool {
    data.starts_with(b"RTPS")
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
    Record {
        elapsed_ms,
        class,
        src_addr: src.to_string(),
        guid_prefix: None,
        entity_id: None,
        dest_port: local_port,
        size: data.len(),
        topic: None,
        summary: if class == Class::Rtps { "RTPS" } else { "UDP" }.to_string(),
        hex: data.to_vec(),
        decoded: String::new(),
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
    pub fn start(config: &CaptureConfig, tx: Sender<Record>) -> std::io::Result<Self> {
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
            let tx = tx.clone();
            let stop = stop.clone();
            threads.push(std::thread::spawn(move || recv_loop(udp, port, tx, stop)));
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

fn recv_loop(udp: UdpSocket, port: u16, tx: Sender<Record>, stop: Arc<AtomicBool>) {
    let epoch = Instant::now();
    let mut buf = vec![0u8; BUF_SIZE];
    loop {
        match udp.recv_from(&mut buf) {
            Ok((n, src)) => {
                let elapsed = epoch.elapsed().as_millis() as u64;
                if tx.send(record_from(&buf[..n], src, port, elapsed)).is_err() {
                    break; // the receiver was dropped
                }
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
    use std::time::Duration;

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
        let (tx, rx) = std::sync::mpsc::channel();
        let capture = Capture::start(&config, tx).expect("start capture");

        let sender = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP)).unwrap();
        sender.set_multicast_if_v6(index).unwrap();
        let sender: UdpSocket = sender.into();
        let dest = SocketAddr::V6(std::net::SocketAddrV6::new(group, 7400, 0, index));
        sender.send_to(b"RTPS\x02\x04hello", dest).unwrap();

        let received = rx
            .recv_timeout(Duration::from_secs(3))
            .expect("a record arrives");
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
        assert_eq!(r.summary, "RTPS");
        assert_eq!(r.src_addr, "127.0.0.1:1234");
        assert_eq!(record_from(b"junk", src, 7400, 0).class, Class::Udp);
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
        let (tx, rx) = std::sync::mpsc::channel();
        let capture = Capture::start(&config, tx).expect("start capture");
        let port = capture.local_ports()[0];

        let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
        sender.send_to(b"RTPS\x02\x04hello", ("127.0.0.1", port)).unwrap();

        let received = rx
            .recv_timeout(Duration::from_secs(3))
            .expect("a record arrives");
        assert_eq!(received.class, Class::Rtps);
        assert_eq!(received.size, 11);
        assert_eq!(received.hex, b"RTPS\x02\x04hello");

        capture.stop();
    }
}

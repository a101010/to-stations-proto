//! The `dds-echo` traffic inspector (Slint).
//!
//! A real UDP capture backend (see [`capture`]) feeds the UI through a bounded
//! queue drained on the event loop, so Slint models are only touched on the UI
//! thread.

// Release builds are a GUI application (no console window); debug builds keep
// the console for diagnostics.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod capture;
mod decode;
mod model;
mod rtps;
mod sedp;
mod send;

mod ui {
    slint::include_modules!();
}

use std::cell::{Cell, RefCell};
use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use slint::{ComponentHandle as _, ModelRc, Timer, TimerMode, VecModel};

use crate::capture::{Capture, CaptureConfig, CaptureQueue, DRAIN_MAX_PER_TICK};
use crate::model::{
    Discovery, History, KnownTopics, RateTracker, Stats, TopicCounts, TopicSelection,
};
use crate::send::{schema_for, SendForm, TopicWriter};
use crate::ui::MainWindow;

use stations_dds::contract::{TopicDef, TopicRegistry};
use stations_dds::cyclonedds::{
    BuiltinEndpointSample, DataReader, DdsEntity, DomainParticipant,
};
use stations_dds::dds::Node;

use cyclonedds_rust_sys as cdds;

/// The UI refresh / capture-drain tick.
const TICK: Duration = Duration::from_millis(100);
/// Minimum interval between full table rebuilds.
const VIEW_REBUILD_MS: u64 = 250;
/// Minimum interval between discovery-reader polls.
const DISCOVERY_POLL_MS: u64 = 500;

#[derive(Parser, Debug)]
#[command(name = "dds-echo", about = "DDSI-RTPS multicast/UDP traffic inspector")]
struct Args {
    /// Interface name(s), comma-separated; a number is an interface index.
    #[arg(long, default_value = "StationsDDS")]
    interface: String,

    /// Multicast group(s), comma-separated. Empty uses the DDS group for --domain.
    #[arg(long, default_value = "")]
    group: String,

    /// Port(s), comma-separated. Empty uses 7400 + 250*domain and +1.
    #[arg(long, default_value = "")]
    ports: String,

    /// DDS domain id (sets the default group/ports).
    #[arg(long, default_value_t = 0)]
    domain: u32,

    /// Topic registry (`topics.toml`); defaults to the deploy config directory.
    #[arg(long)]
    registry: Option<PathBuf>,

    /// CycloneDDS configuration XML for the discovery participant.
    #[arg(long)]
    dds_config: Option<PathBuf>,

    /// Disable the discovery participant and use passive capture only.
    #[arg(long)]
    no_discovery: bool,

    /// Discovery diagnostic log path.
    #[arg(long)]
    discovery_log: Option<PathBuf>,
}

/// Append-only discovery diagnostics. dds-echo is a GUI app in release builds,
/// so double-click runs have no stderr to inspect.
struct DiscoveryLog {
    file: RefCell<Option<File>>,
}

impl DiscoveryLog {
    fn open(explicit: Option<&Path>) -> Self {
        let path = explicit
            .map(Path::to_path_buf)
            .or_else(default_log_path);
        let mut file = None;
        if let Some(path) = path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(opened) = OpenOptions::new().create(true).append(true).open(&path) {
                file = Some(opened);
                eprintln!("dds-echo discovery log: {}", path.display());
            }
        }
        if file.is_none() {
            let path = std::env::temp_dir().join("dds-echo-discovery.log");
            if let Ok(opened) = OpenOptions::new().create(true).append(true).open(&path) {
                file = Some(opened);
                eprintln!("dds-echo discovery log: {}", path.display());
            }
        }
        Self {
            file: RefCell::new(file),
        }
    }

    fn log(&self, message: impl std::fmt::Display) {
        if let Some(file) = self.file.borrow_mut().as_mut() {
            let _ = writeln!(file, "{message}");
            let _ = file.flush();
        }
    }
}

/// The deploy root derived from the executable location: `<exe_dir>/..`.
fn deploy_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let bin = exe.parent()?;
    bin.parent().map(Path::to_path_buf)
}

fn default_log_path() -> Option<PathBuf> {
    deploy_root()
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf))
        })
        .map(|dir| dir.join("dds-echo-discovery.log"))
}

fn resolve_config(explicit: Option<&Path>, log: &DiscoveryLog) -> Option<PathBuf> {
    if let Some(path) = explicit {
        if path.is_file() {
            return Some(path.to_path_buf());
        }
        log.log(format!(
            "explicit CycloneDDS config not found: {}",
            path.display()
        ));
    }
    if let Some(deploy) = deploy_root() {
        let path = deploy.join("config").join("cyclonedds-config.xml");
        if path.is_file() {
            return Some(path);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let path = cwd.join("DDS").join("cyclonedds-config.xml");
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn resolve_registry(explicit: Option<&Path>, log: &DiscoveryLog) -> Option<PathBuf> {
    if let Some(path) = explicit {
        if path.is_file() {
            return Some(path.to_path_buf());
        }
        log.log(format!("explicit registry not found: {}", path.display()));
    }
    if let Some(deploy) = deploy_root() {
        let path = deploy.join("config").join("topics.toml");
        if path.is_file() {
            return Some(path);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let path = cwd.join("contracts").join("topics.toml");
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn ignore_inherited_dds_env(log: &DiscoveryLog) {
    if let Some(inherited) = std::env::var_os("CYCLONEDDS_URI") {
        log.log(format!("ignoring inherited CYCLONEDDS_URI={inherited:?}"));
        std::env::remove_var("CYCLONEDDS_URI");
    }
}

/// Owns an explicitly created DDS domain so the participant uses the supplied
/// configuration without relying on `CYCLONEDDS_URI`.
struct DomainGuard(cdds::dds_entity_t);

impl Drop for DomainGuard {
    fn drop(&mut self) {
        unsafe {
            cdds::dds_delete(self.0);
        }
    }
}

fn create_domain(
    domain: u32,
    config: Option<&Path>,
    log: &DiscoveryLog,
) -> Result<Option<DomainGuard>, String> {
    match config {
        Some(path) => {
            let xml = std::fs::read_to_string(path)
                .map_err(|e| format!("read {}: {e}", path.display()))?;
            let xml = CString::new(xml).map_err(|e| format!("config contains NUL: {e}"))?;
            let handle =
                unsafe { cdds::dds_create_domain(domain as cdds::dds_domainid_t, xml.as_ptr()) };
            if handle <= 0 {
                return Err(format!("dds_create_domain failed: {handle}"));
            }
            log.log(format!(
                "created domain {domain} with config {}",
                path.display()
            ));
            Ok(Some(DomainGuard(handle)))
        }
        None => {
            log.log("no CycloneDDS config found; creating a default domain");
            Ok(None)
        }
    }
}

/// The shared DDS entities: one participant, its contract-aware Node (when a
/// registry was loaded), and the domain guard that must outlive both.
struct DdsResources {
    participant: DomainParticipant,
    node: Option<Node>,
    self_prefix: Option<String>,
    _domain: Option<DomainGuard>,
}

impl DdsResources {
    fn new(
        domain: u32,
        config: Option<&Path>,
        registry: Option<TopicRegistry>,
        log: &DiscoveryLog,
    ) -> Result<Self, String> {
        let domain_guard = create_domain(domain, config, log)?;
        let participant = DomainParticipant::new(domain).map_err(|e| e.to_string())?;
        let self_prefix = participant_prefix(&participant);
        let node = match registry {
            Some(registry) => Some(
                Node::from_participant(participant.clone(), registry).map_err(|e| e.to_string())?,
            ),
            None => None,
        };
        log.log(format!(
            "shared DDS participant created; self prefix {}",
            self_prefix.as_deref().unwrap_or("<unknown>")
        ));
        Ok(Self {
            participant,
            node,
            self_prefix,
            _domain: domain_guard,
        })
    }

    fn node(&self) -> Option<&Node> {
        self.node.as_ref()
    }

    fn self_prefix(&self) -> Option<&str> {
        self.self_prefix.as_deref()
    }
}

fn participant_prefix(participant: &DomainParticipant) -> Option<String> {
    let mut guid = cdds::dds_guid_t { v: [0; 16] };
    let rc = unsafe { cdds::dds_get_guid(participant.entity(), &mut guid) };
    if rc < 0 {
        None
    } else {
        Some(rtps::hex(&guid.v[..12]))
    }
}

/// Lazily created shared resources. Discovery and Send both use the same
/// participant; with `--no-discovery` Send creates it on first use.
struct AppResources {
    domain: u32,
    config: Option<PathBuf>,
    registry: Option<TopicRegistry>,
    slot: RefCell<Option<Rc<DdsResources>>>,
}

impl AppResources {
    fn existing(&self) -> Option<Rc<DdsResources>> {
        self.slot.borrow().clone()
    }

    fn ensure(&self, log: &DiscoveryLog) -> Result<Rc<DdsResources>, String> {
        if let Some(resources) = self.slot.borrow().as_ref() {
            return Ok(resources.clone());
        }
        let resources = Rc::new(DdsResources::new(
            self.domain,
            self.config.as_deref(),
            self.registry.clone(),
            log,
        )?);
        *self.slot.borrow_mut() = Some(resources.clone());
        Ok(resources)
    }
}

/// A discovery reader set over the shared participant, plus the resources that
/// keep its domain alive.
struct DiscoverySource {
    publications: DataReader<BuiltinEndpointSample>,
    subscriptions: DataReader<BuiltinEndpointSample>,
    _resources: Rc<DdsResources>,
    last_poll_ms: u64,
}

impl DiscoverySource {
    fn new(resources: Rc<DdsResources>) -> Result<Self, String> {
        let publications = resources
            .participant
            .create_builtin_publication_reader()
            .map_err(|e| e.to_string())?;
        let subscriptions = resources
            .participant
            .create_builtin_subscription_reader()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            publications,
            subscriptions,
            _resources: resources,
            last_poll_ms: 0,
        })
    }

    fn poll(
        &mut self,
        now_ms: u64,
        discovery: &mut Discovery,
        force: bool,
        log: &DiscoveryLog,
    ) -> usize {
        if !force
            && self.last_poll_ms != 0
            && now_ms.saturating_sub(self.last_poll_ms) < DISCOVERY_POLL_MS
        {
            return 0;
        }
        self.last_poll_ms = now_ms.max(1);

        let mut learned = 0;
        match self.publications.read() {
            Ok(samples) => {
                log.log(format!("poll t={now_ms}ms publications={}", samples.len()));
                for sample in &samples {
                    learned +=
                        learn_builtin_endpoint(discovery, sample, model::EndpointKind::Writer, log);
                }
            }
            Err(e) => log.log(format!("publications read failed: {e}")),
        }
        match self.subscriptions.read() {
            Ok(samples) => {
                log.log(format!("poll t={now_ms}ms subscriptions={}", samples.len()));
                for sample in &samples {
                    learned +=
                        learn_builtin_endpoint(discovery, sample, model::EndpointKind::Reader, log);
                }
            }
            Err(e) => log.log(format!("subscriptions read failed: {e}")),
        }
        log.log(format!(
            "poll learned={learned} total_endpoints={}",
            discovery.endpoints()
        ));
        learned
    }

    fn reset_poll_clock(&mut self) {
        self.last_poll_ms = 0;
    }
}

fn learn_builtin_endpoint(
    discovery: &mut Discovery,
    sample: &BuiltinEndpointSample,
    kind: model::EndpointKind,
    log: &DiscoveryLog,
) -> usize {
    let topic = sample.topic_name();
    let type_name = sample.type_name_value();
    let key = sample.key();
    let participant_key = sample.participant_key();
    if topic.is_empty() || topic.starts_with("DCPS") {
        return 0;
    }
    let endpoint = model::Endpoint {
        guid: rtps::hex(&key.v),
        prefix: rtps::hex(&participant_key.v[..12]),
        kind,
        topic,
        type_name,
    };
    let is_new = discovery.learn_endpoint(endpoint.clone());
    if is_new {
        log.log(format!(
            "learned {} topic={} type={} guid={} prefix={}",
            endpoint.kind.label(),
            endpoint.topic,
            endpoint.type_name,
            endpoint.guid,
            endpoint.prefix
        ));
    }
    usize::from(is_new)
}

/// Shared state behind the UI.
struct State {
    capture: RefCell<Option<Capture>>,
    queue: RefCell<Option<Arc<CaptureQueue>>>,
    discovery_source: RefCell<Option<DiscoverySource>>,
    stats: RefCell<Stats>,
    counts: RefCell<TopicCounts>,
    discovery: RefCell<Discovery>,
    rates: RefCell<RateTracker>,
    start: Cell<Instant>,
    last_view_ms: Cell<u64>,
    pending_view: Cell<bool>,
    /// The topic rows selected in the topics pane (the center filter).
    selection: RefCell<TopicSelection>,
    /// The retained raw records, evicted frequency-first.
    history: RefCell<History>,
    /// History ids aligned with the visible record rows.
    displayed_ids: RefCell<Vec<u64>>,
    /// The selected record's history id, preserved across view rebuilds.
    selected_id: Cell<Option<u64>>,
    /// Records evicted from the capture queue (never observed by the UI).
    dropped: Cell<u64>,
    /// Records evicted from the history (observed, then trimmed from the table).
    trimmed: Cell<u64>,
    /// Hide records originating from dds-echo's own participant.
    hide_self: Cell<bool>,
    /// Discovery diagnostics file.
    log: DiscoveryLog,
    /// The registry's topics (name -> IDL type), loaded at startup.
    known_topics: KnownTopics,
    /// The topics the Send tab can choose from.
    send_topics: Vec<TopicDef>,
    /// The shared DDS resources used by discovery and Send.
    resources: Rc<AppResources>,
    /// The Send tab's state and periodic timer.
    send: Rc<RefCell<SendRuntime>>,
    /// The Send tab's generated form rows.
    send_rows: Rc<VecModel<ui::SendField>>,
}

/// The Send tab's mutable state. The timer is kept here but its callback holds
/// only weak references, so the state and timer do not form a cycle.
struct SendRuntime {
    selected: Option<usize>,
    form: Option<SendForm>,
    writer: Option<TopicWriter>,
    /// Keeps the shared domain and participant alive while a writer exists.
    resources: Option<Rc<DdsResources>>,
    interval_ms: u64,
    repeating: bool,
    sent: u64,
    status: String,
    error: bool,
    timer: Rc<Timer>,
}

impl SendRuntime {
    fn new() -> Self {
        Self {
            selected: None,
            form: None,
            writer: None,
            resources: None,
            interval_ms: 100,
            repeating: false,
            sent: 0,
            status: "Select a topic".to_string(),
            error: false,
            timer: Rc::new(Timer::default()),
        }
    }

    fn ready(&self) -> bool {
        self.form
            .as_ref()
            .is_some_and(|form| !form.has_unsupported())
    }
}

fn to_send_field(row: &crate::send::FormRow) -> ui::SendField {
    ui::SendField {
        path: row.path.clone().into(),
        label: row.label.clone().into(),
        kind: row.kind.as_str().into(),
        indent: row.indent as i32,
        value: row.value.clone().into(),
        bool_value: row.bool_value,
        group: row.is_group(),
        unsupported: row.unsupported,
    }
}

fn update_send_props(app: &MainWindow, send: &SendRuntime) {
    app.set_send_ready(send.ready());
    app.set_send_repeating(send.repeating);
    app.set_send_count(send.sent.min(i32::MAX as u64) as i32);
    app.set_send_status(send.status.clone().into());
    app.set_send_error(send.error);
}

fn select_send_topic(app: &MainWindow, state: &Rc<State>, index: i32) {
    if index < 0 {
        return;
    }
    let index = index as usize;
    let Some(topic) = state.send_topics.get(index).cloned() else {
        return;
    };

    {
        let mut send = state.send.borrow_mut();
        send.timer.stop();
        send.repeating = false;
        send.selected = Some(index);
        send.form = None;
        send.writer = None;
        send.status = format!("Selected {}", topic.name);
        send.error = false;
        app.set_send_topic_index(index as i32);
        update_send_props(app, &send);
    }

    let Some(schema) = schema_for(&topic.idl_type) else {
        let mut send = state.send.borrow_mut();
        send.status = format!("Error: unsupported topic type `{}`", topic.idl_type);
        send.error = true;
        update_send_props(app, &send);
        return;
    };

    let form = SendForm::from_schema(&topic.idl_type, schema);
    let rows: Vec<ui::SendField> = form.rows().iter().map(to_send_field).collect();
    state.send_rows.set_vec(rows);

    let writer_result = match state.resources.existing() {
        Some(resources) => match resources.node() {
            Some(node) => Some((
                resources.clone(),
                TopicWriter::new(node, &topic.name, &topic.idl_type),
            )),
            None => Some((
                resources.clone(),
                Err("the topic registry was not loaded; sending is unavailable".to_string()),
            )),
        },
        None => None,
    };

    let unsupported = form.has_unsupported();
    let mut send = state.send.borrow_mut();
    send.form = Some(form);
    match writer_result {
        Some((resources, Ok(writer))) => {
            let matched = writer.matched_readers().unwrap_or(0);
            send.resources = Some(resources);
            send.writer = Some(writer);
            send.status = if matched > 0 {
                format!("Ready: {} ({matched} matched subscriber(s))", topic.name)
            } else {
                format!("Ready: {}; no matched subscribers", topic.name)
            };
            send.error = matched == 0;
        }
        Some((resources, Err(error))) => {
            send.resources = Some(resources);
            send.status = format!("Error: {error}");
            send.error = true;
        }
        None => {
            send.status = format!("Selected {}; press Send to start DDS", topic.name);
            send.error = false;
        }
    }
    if unsupported && !send.error {
        send.status = "This topic has unsupported field types".to_string();
        send.error = true;
    }
    update_send_props(app, &send);
}

/// Create the selected topic's writer on first use. With `--no-discovery` this
/// is the moment the shared participant is created.
fn ensure_writer(state: &Rc<State>) -> Result<(), String> {
    {
        let send = state.send.borrow();
        if send.writer.is_some() {
            return Ok(());
        }
        if send.form.is_none() {
            return Err("no topic selected".to_string());
        }
    }

    let topic = {
        let send = state.send.borrow();
        send.selected
            .and_then(|index| state.send_topics.get(index).cloned())
            .ok_or_else(|| "no topic selected".to_string())?
    };
    let resources = state.resources.ensure(&state.log)?;
    let node = resources
        .node()
        .ok_or_else(|| "the topic registry was not loaded; sending is unavailable".to_string())?;
    let writer = TopicWriter::new(node, &topic.name, &topic.idl_type)?;
    let matched = writer.matched_readers().unwrap_or(0);
    if let Some(prefix) = resources.self_prefix() {
        state
            .discovery
            .borrow_mut()
            .set_self_prefix(Some(prefix.to_string()));
    }

    let mut send = state.send.borrow_mut();
    send.resources = Some(resources);
    send.writer = Some(writer);
    send.status = if matched > 0 {
        format!("Ready: {} ({matched} matched subscriber(s))", topic.name)
    } else {
        format!("Ready: {}; no matched subscribers", topic.name)
    };
    send.error = matched == 0;
    Ok(())
}

fn send_now(app: &MainWindow, state: &Rc<State>, timer: &Weak<Timer>) {
    if let Err(error) = ensure_writer(state) {
        if let Some(timer) = timer.upgrade() {
            timer.stop();
        }
        let mut send = state.send.borrow_mut();
        send.repeating = false;
        send.error = true;
        send.status = format!("Error: {error}");
        update_send_props(app, &send);
        return;
    }

    let result: Result<usize, String> = (|| {
        let send = state.send.borrow();
        let form = send.form.as_ref().ok_or("no topic selected")?;
        let writer = send.writer.as_ref().ok_or("no writer available")?;
        let matched = writer.matched_readers()?;
        if matched == 0 {
            return Err("No matching subscriber; sample not transmitted".to_string());
        }
        let value = form.to_dynamic_value()?;
        writer.write_dynamic(&value)?;
        Ok(matched)
    })();

    match result {
        Ok(matched) => {
            let mut send = state.send.borrow_mut();
            send.sent += 1;
            send.error = false;
            send.status = format!(
                "Sent {} sample(s) to {} subscriber(s)",
                send.sent, matched
            );
            update_send_props(app, &send);
        }
        Err(error) => {
            if let Some(timer) = timer.upgrade() {
                timer.stop();
            }
            let mut send = state.send.borrow_mut();
            send.repeating = false;
            send.error = true;
            send.status = error;
            update_send_props(app, &send);
        }
    }
}

fn start_sending(app: &MainWindow, state: &Rc<State>) {
    if let Err(error) = ensure_writer(state) {
        let mut send = state.send.borrow_mut();
        send.repeating = false;
        send.error = true;
        send.status = format!("Error: {error}");
        update_send_props(app, &send);
        return;
    }

    let (ready, matched) = {
        let send = state.send.borrow();
        let matched = send
            .writer
            .as_ref()
            .and_then(|writer| writer.matched_readers().ok())
            .unwrap_or(0);
        (send.ready(), matched)
    };
    if !ready {
        let mut send = state.send.borrow_mut();
        send.repeating = false;
        send.error = true;
        send.status = "Select a supported topic before sending".to_string();
        update_send_props(app, &send);
        return;
    }
    if matched == 0 {
        let mut send = state.send.borrow_mut();
        send.repeating = false;
        send.error = true;
        send.status = "No matching subscriber; sample not transmitted".to_string();
        update_send_props(app, &send);
        return;
    }

    let (interval, timer) = {
        let send = state.send.borrow();
        (
            Duration::from_millis(send.interval_ms.max(1)),
            send.timer.clone(),
        )
    };

    {
        let mut send = state.send.borrow_mut();
        send.repeating = true;
        send.error = false;
        send.status = format!(
            "Sending every {} ms to {} subscriber(s)",
            interval.as_millis(),
            matched
        );
        update_send_props(app, &send);
    }

    let weak_state = Rc::downgrade(state);
    let weak_timer = Rc::downgrade(&timer);
    let app_weak = app.as_weak();
    timer.start(TimerMode::Repeated, interval, move || {
        if let (Some(app), Some(state)) = (app_weak.upgrade(), weak_state.upgrade()) {
            send_now(&app, &state, &weak_timer);
        }
    });
}

fn stop_sending(app: &MainWindow, state: &Rc<State>) {
    let mut send = state.send.borrow_mut();
    send.timer.stop();
    send.repeating = false;
    if !send.error {
        send.status = "Stopped".to_string();
    }
    update_send_props(app, &send);
}

/// A submessage's envelope row, e.g.
/// `ID 15   Kind DATA   Writer 000003c2   Reader 00000000   Seq 1   Len 92`.
fn frame_line(f: &model::Frame) -> String {
    let mut parts = vec![format!("ID {:02x}", f.id), format!("Kind {}", f.kind)];
    if !f.writer.is_empty() {
        parts.push(format!("Writer {}", f.writer));
    }
    if !f.reader.is_empty() {
        parts.push(format!("Reader {}", f.reader));
    }
    if !f.seq.is_empty() {
        parts.push(format!("Seq {}", f.seq));
    }
    parts.push(format!("Len {}", f.len));
    parts.join("   ")
}

/// The decoded row(s) beneath a submessage envelope (empty when nothing decoded).
fn frame_detail(f: &model::Frame) -> String {
    match &f.announcement {
        Some(model::Announcement::Participant { prefix }) => format!("Participant {prefix}"),
        Some(model::Announcement::Endpoint(e)) => format!(
            "Endpoint {} {}   Topic {}   Type {}",
            e.kind.label(),
            e.guid.get(24..).unwrap_or(&e.guid),
            e.topic,
            e.type_name
        ),
        None => String::new(),
    }
}

/// An empty detail model, used when no record is selected.
fn empty_record() -> ui::Record {
    ui::Record {
        time: "".into(),
        protocol: "".into(),
        source: "".into(),
        size: 0,
        topic: "".into(),
        submessages: "".into(),
        hex: "".into(),
        decoded: "".into(),
        guid: "".into(),
        entity: "".into(),
        known: false,
        frames: ModelRc::default(),
        text: "".into(),
    }
}

/// The lightweight table row; hex, decoded fields, and frames are deferred to
/// [`to_detail`].
fn to_row(r: &model::Record) -> ui::RecordRow {
    ui::RecordRow {
        time: r.time_label().into(),
        protocol: r.protocol().into(),
        source: if r.self_origin {
            "self".into()
        } else {
            r.src_addr.clone().into()
        },
        size: r.size as i32,
        topic: r.display_topic().unwrap_or_default().into(),
        submessages: r.submessages.clone().into(),
        known: r.kind == model::MessageKind::KnownTopic,
    }
}

/// The full detail model, built only when a row is selected.
fn to_detail(r: &model::Record, known: &KnownTopics) -> ui::Record {
    let frames: Vec<ui::Frame> = r
        .frames
        .iter()
        .map(|f| ui::Frame {
            line: frame_line(f).into(),
            detail: frame_detail(f).into(),
        })
        .collect();
    ui::Record {
        time: r.time_label().into(),
        protocol: r.protocol().into(),
        source: if r.self_origin {
            format!("self ({})", r.src_addr).into()
        } else {
            r.src_addr.clone().into()
        },
        size: r.size as i32,
        topic: r.display_topic().unwrap_or_default().into(),
        submessages: r.submessages.clone().into(),
        hex: r.hex_string().into(),
        decoded: decoded_text(r, known).into(),
        guid: r.guid_prefix.clone().unwrap_or_default().into(),
        entity: r.entity_id.clone().unwrap_or_default().into(),
        known: r.kind == model::MessageKind::KnownTopic,
        frames: ModelRc::from(frames.as_slice()),
        text: detail_text(r, known).into(),
    }
}

/// The plain-text rendering used by the selectable detail pane.
fn detail_text(r: &model::Record, known: &KnownTopics) -> String {
    let source = if r.self_origin {
        format!("self ({})", r.src_addr)
    } else {
        r.src_addr.clone()
    };
    let mut out = String::new();
    out.push_str(&format!("time: {}\n", r.time_label()));
    out.push_str(&format!("protocol: {}\n", r.protocol()));
    out.push_str(&format!("source: {source}\n"));
    out.push_str(&format!(
        "participant: {}\n",
        r.guid_prefix.as_deref().unwrap_or("")
    ));
    out.push_str(&format!(
        "writer: {}\n",
        r.entity_id.as_deref().unwrap_or("")
    ));
    out.push_str(&format!("size: {}\n", r.size));
    out.push_str(&format!(
        "kind: {}\n",
        if r.kind.is_topic() {
            "topic"
        } else {
            r.kind.label()
        }
    ));
    out.push_str(&format!(
        "topic: {}\n",
        r.display_topic().unwrap_or("")
    ));
    out.push_str(&format!("submessages: {}\n", r.submessages));
    out.push('\n');
    out.push_str(&format!("frames ({}):\n", r.frames.len()));
    for frame in &r.frames {
        out.push_str(&format!("  {}\n", frame_line(frame)));
        let detail = frame_detail(frame);
        if !detail.is_empty() {
            out.push_str(&format!("    {detail}\n"));
        }
    }

    let decoded = decoded_text(r, known);
    if !decoded.is_empty() {
        out.push('\n');
        out.push_str("decoded:\n");
        out.push_str(&decoded);
        if !decoded.ends_with('\n') {
            out.push('\n');
        }
    }

    out.push('\n');
    out.push_str("hex:\n");
    out.push_str(&r.hex_string());
    out.push('\n');
    out
}

/// Decode the sample's payload when the record's topic is registered; the
/// detail pane shows raw hex otherwise.
fn decoded_text(r: &model::Record, known: &KnownTopics) -> String {
    let Some(idl_type) = r.topic.as_deref().and_then(|t| known.idl_type(t)) else {
        return String::new();
    };
    let Some(range) = r
        .frames
        .iter()
        .find(|f| f.kind == "DATA" && f.payload.is_some())
        .and_then(|f| f.payload.clone())
    else {
        return String::new();
    };
    r.hex
        .get(range)
        .and_then(|bytes| decode::decode(idl_type, bytes))
        .unwrap_or_default()
}

/// Rebuild the visible rows from the retained history, in arrival order,
/// formatting only records that pass the topic filter. This runs at most once
/// per tick (and immediately on a filter change), never per record.
fn rebuild_view(app: &MainWindow, state: &State, records: &VecModel<ui::RecordRow>) {
    let history = state.history.borrow();
    let selection = state.selection.borrow();
    let selected = state.selected_id.get();
    let mut rows = Vec::new();
    let mut ids = Vec::new();
    let mut selected_index = -1i32;
    for (id, record) in history.matching(&selection, state.hide_self.get()) {
        if Some(id) == selected {
            selected_index = rows.len() as i32;
        }
        rows.push(to_row(record));
        ids.push(id);
    }
    records.set_vec(rows);
    *state.displayed_ids.borrow_mut() = ids;
    app.set_selected_index(selected_index);
    if selected.is_some() && selected_index < 0 {
        state.selected_id.set(None);
        app.set_selected(empty_record());
    }
}

/// The five topics-pane groups.
struct TopicModels {
    known: Rc<VecModel<ui::TopicRow>>,
    unknown: Rc<VecModel<ui::TopicRow>>,
    discovery: Rc<VecModel<ui::TopicRow>>,
    rtps_control: Rc<VecModel<ui::TopicRow>>,
    unidentified: Rc<VecModel<ui::TopicRow>>,
}

impl TopicModels {
    fn new() -> Self {
        Self {
            known: Rc::new(VecModel::default()),
            unknown: Rc::new(VecModel::default()),
            discovery: Rc::new(VecModel::default()),
            rtps_control: Rc::new(VecModel::default()),
            unidentified: Rc::new(VecModel::default()),
        }
    }
}

/// Re-apply the current selection: refresh the topics rows and rebuild the
/// record view.
fn apply_selection(
    app: &MainWindow,
    state: &State,
    records: &VecModel<ui::RecordRow>,
    topics: &TopicModels,
) {
    refresh_topics(
        app,
        &state
            .counts
            .borrow()
            .rows(&state.known_topics, state.hide_self.get()),
        &state.selection.borrow(),
        topics,
    );
    rebuild_view(app, state, records);
    state.pending_view.set(false);
    state
        .last_view_ms
        .set(state.start.get().elapsed().as_millis() as u64);
}

fn refresh_topics(
    app: &MainWindow,
    rows: &[model::TopicRow],
    selection: &TopicSelection,
    topics: &TopicModels,
) {
    topics.known.clear();
    topics.unknown.clear();
    topics.discovery.clear();
    topics.rtps_control.clear();
    topics.unidentified.clear();
    let mut known_total = 0i32;
    let mut unknown_total = 0i32;
    let mut discovery_total = 0i32;
    let mut rtps_control_total = 0i32;
    let mut unidentified_total = 0i32;
    for row in rows {
        let packets = row.packets as i32;
        let ui_row = ui::TopicRow {
            name: row.name.clone().into(),
            known: row.kind == model::MessageKind::KnownTopic,
            packets,
            selected: selection.contains(&row.name),
        };
        match row.kind {
            model::MessageKind::KnownTopic => {
                known_total += packets;
                topics.known.push(ui_row);
            }
            model::MessageKind::UnregisteredTopic => {
                unknown_total += packets;
                topics.unknown.push(ui_row);
            }
            model::MessageKind::Discovery => {
                discovery_total += packets;
                topics.discovery.push(ui_row);
            }
            model::MessageKind::RtpsControl => {
                rtps_control_total += packets;
                topics.rtps_control.push(ui_row);
            }
            model::MessageKind::UnidentifiedData | model::MessageKind::Udp => {
                unidentified_total += packets;
                topics.unidentified.push(ui_row);
            }
        }
    }
    app.set_known_total(known_total);
    app.set_unknown_total(unknown_total);
    app.set_discovery_total(discovery_total);
    app.set_rtps_control_total(rtps_control_total);
    app.set_unidentified_total(unidentified_total);
}

/// Poll the discovery readers and re-attribute retained records when new
/// endpoints are learned. Returns true when topic counts changed.
fn poll_discovery(
    app: &MainWindow,
    state: &State,
    topics: &TopicModels,
    force: bool,
) -> bool {
    let now_ms = state.start.get().elapsed().as_millis() as u64;
    let learned = {
        let mut source = state.discovery_source.borrow_mut();
        match source.as_mut() {
            Some(source) => {
                let mut discovery = state.discovery.borrow_mut();
                source.poll(now_ms, &mut discovery, force, &state.log)
            }
            None => 0,
        }
    };
    if learned == 0 {
        return false;
    }

    let changes = {
        let discovery = state.discovery.borrow();
        state
            .history
            .borrow_mut()
            .reattribute(&discovery, &state.known_topics)
    };
    if changes.is_empty() {
        return false;
    }

    {
        let mut counts = state.counts.borrow_mut();
        for change in &changes {
            counts.reattribute(change);
        }
    }
    state.pending_view.set(true);
    refresh_topics(
        app,
        &state
            .counts
            .borrow()
            .rows(&state.known_topics, state.hide_self.get()),
        &state.selection.borrow(),
        topics,
    );
    true
}

fn tick(
    app: &MainWindow,
    state: &State,
    records: &VecModel<ui::RecordRow>,
    topics: &TopicModels,
) {
    let now_ms = state.start.get().elapsed().as_millis() as u64;
    let self_prefix = state
        .resources
        .existing()
        .and_then(|resources| resources.self_prefix().map(str::to_string));
    state
        .discovery
        .borrow_mut()
        .set_self_prefix(self_prefix.clone());

    poll_discovery(app, state, topics, false);

    // Drain a bounded batch from the capture queue and append it to the
    // frequency-ranked history.
    let mut dirty = false;
    let mut trimmed = 0usize;
    {
        let queue = state.queue.borrow();
        if let Some(queue) = queue.as_ref() {
            let batch = queue.drain(DRAIN_MAX_PER_TICK);
            dirty = !batch.is_empty();
            let mut history = state.history.borrow_mut();
            for mut record in batch {
                state.discovery.borrow_mut().observe_and_attr(&mut record);
                if record.is_self(self_prefix.as_deref()) {
                    record.self_origin = true;
                }
                record.kind = record.classify(&state.known_topics);
                let topic = record.topic.clone();
                let first_for_topic = record.kind.is_topic()
                    && topic
                        .as_deref()
                        .is_some_and(|topic| state.counts.borrow().count(topic) == 0);
                state.stats.borrow_mut().observe(&record);
                state.counts.borrow_mut().observe(&record);
                if first_for_topic {
                    if let Some(topic) = &topic {
                        state.log.log(format!("first record attributed to topic {topic}"));
                    }
                }
                trimmed += history.push(record).len();
            }
            state.dropped.set(queue.dropped());
        }
    }
    if trimmed > 0 {
        state.trimmed.set(state.trimmed.get() + trimmed as u64);
    }
    if dirty {
        state.pending_view.set(true);
    }
    if app.get_follow()
        && state.pending_view.get()
        && now_ms.saturating_sub(state.last_view_ms.get()) >= VIEW_REBUILD_MS
    {
        rebuild_view(app, state, records);
        state.pending_view.set(false);
        state.last_view_ms.set(now_ms);
    }

    refresh_topics(
        app,
        &state
            .counts
            .borrow()
            .rows(&state.known_topics, state.hide_self.get()),
        &state.selection.borrow(),
        topics,
    );

    {
        let discovery = state.discovery.borrow();
        app.set_participants(discovery.participants() as i32);
        app.set_endpoints(discovery.endpoints() as i32);
    }

    let (stats, rates) = {
        let stats = state.stats.borrow();
        let visible = if state.hide_self.get() {
            stats.excluding_self()
        } else {
            stats.clone()
        };
        let rates = state.rates.borrow_mut().sample(&visible, now_ms);
        (visible, rates)
    };
    app.set_packets(stats.packets as i32);
    app.set_bytes(stats.bytes as i32);
    app.set_rtps(stats.rtps as i32);
    app.set_pps(rates.pps as i32);
    app.set_bps(rates.bps as i32);
    app.set_dropped(state.dropped.get().min(i32::MAX as u64) as i32);
    app.set_trimmed(state.trimmed.get().min(i32::MAX as u64) as i32);
}

/// Load the topic registry; on failure, log and return `None` so capture can
/// still run with an empty known-topic set.
fn load_registry(path: &Path, log: &DiscoveryLog) -> Option<TopicRegistry> {
    match TopicRegistry::load(path) {
        Ok(registry) => {
            log.log(format!("loaded registry {}", path.display()));
            Some(registry)
        }
        Err(e) => {
            log.log(format!(
                "failed to load registry {}: {e}",
                path.display()
            ));
            eprintln!("dds-echo: failed to load registry {}: {e}", path.display());
            None
        }
    }
}

fn known_from_registry(registry: &TopicRegistry) -> KnownTopics {
    let mut known = KnownTopics::default();
    for topic in &registry.topics {
        known.insert(&topic.name, &topic.idl_type);
    }
    known
}

fn main() -> Result<(), slint::PlatformError> {
    let args = Args::parse();

    let _instance = match single_instance::acquire("dds-echo") {
        single_instance::Acquire::Primary(guard) => guard,
        single_instance::Acquire::Duplicate => {
            single_instance::activate_existing_window("dds-echo");
            return Ok(());
        }
    };

    let log = DiscoveryLog::open(args.discovery_log.as_deref());
    log.log(format!("dds-echo starting; domain={}", args.domain));

    ignore_inherited_dds_env(&log);
    let config = resolve_config(args.dds_config.as_deref(), &log);
    match &config {
        Some(path) => log.log(format!("using CycloneDDS config {}", path.display())),
        None => log.log("no CycloneDDS config found; discovery uses defaults"),
    }

    let registry_path = resolve_registry(args.registry.as_deref(), &log);
    let registry = registry_path
        .as_deref()
        .and_then(|path| load_registry(path, &log));
    let known_topics = registry
        .as_ref()
        .map(known_from_registry)
        .unwrap_or_default();

    let resources = Rc::new(AppResources {
        domain: args.domain,
        config: config.clone(),
        registry: registry.clone(),
        slot: RefCell::new(None),
    });
    let discovery_source = if args.no_discovery {
        log.log("discovery disabled by --no-discovery");
        None
    } else {
        match resources.ensure(&log).and_then(DiscoverySource::new) {
            Ok(source) => {
                log.log("discovery participant created");
                Some(source)
            }
            Err(e) => {
                log.log(format!("discovery participant creation failed: {e}"));
                eprintln!("dds-echo: discovery participant disabled: {e}");
                None
            }
        }
    };
    let send_topics = registry
        .as_ref()
        .map(|registry| registry.topics.clone())
        .unwrap_or_default();

    let app = MainWindow::new()?;

    // Seed the toolbar fields from the CLI (group/ports default from --domain).
    let defaults = CaptureConfig::for_domain(args.domain);
    let group = if args.group.trim().is_empty() {
        defaults
            .groups
            .iter()
            .map(|g| g.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        args.group.clone()
    };
    let ports = if args.ports.trim().is_empty() {
        defaults
            .ports
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        args.ports.clone()
    };
    app.set_iface(args.interface.clone().into());
    app.set_group(group.into());
    app.set_ports(ports.into());

    let state = Rc::new(State {
        capture: RefCell::new(None),
        queue: RefCell::new(None),
        discovery_source: RefCell::new(discovery_source),
        stats: RefCell::new(Stats::default()),
        counts: RefCell::new(TopicCounts::default()),
        discovery: RefCell::new(Discovery::default()),
        rates: RefCell::new(RateTracker::default()),
        start: Cell::new(Instant::now()),
        last_view_ms: Cell::new(0),
        pending_view: Cell::new(false),
        selection: RefCell::new(TopicSelection::default()),
        history: RefCell::new(History::default()),
        displayed_ids: RefCell::new(Vec::new()),
        selected_id: Cell::new(None),
        dropped: Cell::new(0),
        trimmed: Cell::new(0),
        hide_self: Cell::new(false),
        log,
        known_topics,
        send_topics,
        resources,
        send: Rc::new(RefCell::new(SendRuntime::new())),
        send_rows: Rc::new(VecModel::<ui::SendField>::default()),
    });

    let records = Rc::new(VecModel::<ui::RecordRow>::default());
    let topics = Rc::new(TopicModels::new());
    app.set_records(ModelRc::from(records.clone()));
    app.set_known_topics(ModelRc::from(topics.known.clone()));
    app.set_unknown_topics(ModelRc::from(topics.unknown.clone()));
    app.set_discovery_topics(ModelRc::from(topics.discovery.clone()));
    app.set_rtps_control_topics(ModelRc::from(topics.rtps_control.clone()));
    app.set_unidentified_topics(ModelRc::from(topics.unidentified.clone()));
    refresh_topics(
        &app,
        &state
            .counts
            .borrow()
            .rows(&state.known_topics, state.hide_self.get()),
        &state.selection.borrow(),
        &topics,
    );

    app.set_send_fields(ModelRc::from(state.send_rows.clone()));
    let send_topic_names: Vec<slint::SharedString> = state
        .send_topics
        .iter()
        .map(|topic| topic.name.as_str().into())
        .collect();
    let send_topics_model = Rc::new(VecModel::<slint::SharedString>::default());
    send_topics_model.set_vec(send_topic_names);
    app.set_send_topics(ModelRc::from(send_topics_model));
    app.set_send_interval_ms(state.send.borrow().interval_ms as i32);
    {
        let send = state.send.borrow();
        update_send_props(&app, &send);
    }
    if !state.send_topics.is_empty() {
        select_send_topic(&app, &state, 0);
    }

    let timer = Rc::new(Timer::default());

    // Start.
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let records = records.clone();
        let topics = topics.clone();
        let timer = timer.clone();
        app.on_start(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let iface = app.get_iface().to_string();
            let group = app.get_group().to_string();
            let ports = app.get_ports().to_string();
            let config = match CaptureConfig::from_fields(&iface, &group, &ports) {
                Ok(config) => config,
                Err(e) => {
                    app.set_error_message(format!("Invalid capture settings: {e}").into());
                    return;
                }
            };

            let queue = Arc::new(CaptureQueue::default());
            match Capture::start(&config, queue.clone()) {
                Ok(capture) => {
                    *state.capture.borrow_mut() = Some(capture);
                    *state.queue.borrow_mut() = Some(queue);
                    app.set_running(true);
                    state.start.set(Instant::now());
                    *state.rates.borrow_mut() = RateTracker::default();
                    state.pending_view.set(false);
                    app.set_follow(true);
                    if let Some(source) = state.discovery_source.borrow_mut().as_mut() {
                        source.reset_poll_clock();
                    }

                    state.stats.borrow_mut().reset();
                    state.counts.borrow_mut().reset();
                    state.history.borrow_mut().clear();
                    state.displayed_ids.borrow_mut().clear();
                    state.selected_id.set(None);
                    state.dropped.set(0);
                    state.trimmed.set(0);
                    state.last_view_ms.set(0);

                    records.clear();
                    refresh_topics(
                        &app,
                        &state
            .counts
            .borrow()
            .rows(&state.known_topics, state.hide_self.get()),
                        &state.selection.borrow(),
                        &topics,
                    );
                    app.set_selected(empty_record());
                    app.set_selected_index(-1);
                    app.set_packets(0);
                    app.set_bytes(0);
                    app.set_rtps(0);
                    app.set_pps(0);
                    app.set_bps(0);
                    app.set_participants(0);
                    app.set_endpoints(0);
                    app.set_dropped(0);
                    app.set_trimmed(0);

                    let app_weak = app.as_weak();
                    let state = state.clone();
                    let records = records.clone();
                    let topics = topics.clone();
                    timer.start(TimerMode::Repeated, TICK, move || {
                        if let Some(app) = app_weak.upgrade() {
                            tick(&app, &state, &records, &topics);
                        }
                    });
                }
                Err(e) => app.set_error_message(format!("Capture failed: {e}").into()),
            }
        });
    }

    // Stop.
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let records = records.clone();
        let topics = topics.clone();
        let timer = timer.clone();
        app.on_stop(move || {
            if let Some(app) = app_weak.upgrade() {
                app.set_running(false);
            }
            timer.stop();
            if let Some(capture) = state.capture.borrow_mut().take() {
                capture.stop();
            }
            if let Some(queue) = state.queue.borrow_mut().take() {
                state.dropped.set(queue.dropped());
            }
            if let Some(app) = app_weak.upgrade() {
                poll_discovery(&app, &state, &topics, true);
                refresh_topics(
                    &app,
                    &state
            .counts
            .borrow()
            .rows(&state.known_topics, state.hide_self.get()),
                    &state.selection.borrow(),
                    &topics,
                );
                rebuild_view(&app, &state, &records);
                {
                    let discovery = state.discovery.borrow();
                    app.set_participants(discovery.participants() as i32);
                    app.set_endpoints(discovery.endpoints() as i32);
                }
                app.set_dropped(state.dropped.get().min(i32::MAX as u64) as i32);
                app.set_trimmed(state.trimmed.get().min(i32::MAX as u64) as i32);
            }
        });
    }

    // Clear.
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let records = records.clone();
        let topics = topics.clone();
        app.on_clear(move || {
            if let Some(app) = app_weak.upgrade() {
                if let Some(queue) = state.queue.borrow().as_ref() {
                    queue.clear();
                }
                records.clear();
                state.displayed_ids.borrow_mut().clear();
                state.history.borrow_mut().clear();
                state.selection.borrow_mut().clear();
                state.stats.borrow_mut().reset();
                state.counts.borrow_mut().reset();
                *state.rates.borrow_mut() = RateTracker::default();
                state.selected_id.set(None);
                state.dropped.set(0);
                state.trimmed.set(0);
                state.last_view_ms.set(0);
                state.pending_view.set(false);
                app.set_follow(true);
                app.set_selected(empty_record());
                refresh_topics(
                    &app,
                    &state
            .counts
            .borrow()
            .rows(&state.known_topics, state.hide_self.get()),
                    &state.selection.borrow(),
                    &topics,
                );
                app.set_packets(0);
                app.set_bytes(0);
                app.set_rtps(0);
                app.set_pps(0);
                app.set_bps(0);
                app.set_participants(0);
                app.set_endpoints(0);
                app.set_dropped(0);
                app.set_trimmed(0);
                app.set_selected_index(-1);
            }
        });
    }

    // Hide/show dds-echo's own traffic.
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let records = records.clone();
        let topics = topics.clone();
        app.on_hide_self_changed(move |hide| {
            if let Some(app) = app_weak.upgrade() {
                state.hide_self.set(hide);
                app.set_hide_self(hide);
                apply_selection(&app, &state, &records, &topics);
            }
        });
    }

    // Select.
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        app.on_select(move |index| {
            if index < 0 {
                return;
            }
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let Some(id) = state.displayed_ids.borrow().get(index as usize).copied() else {
                return;
            };
            let detail = {
                let history = state.history.borrow();
                history
                    .find(id)
                    .map(|record| to_detail(record, &state.known_topics))
            };
            if let Some(detail) = detail {
                app.set_selected(detail);
                app.set_selected_index(index);
                state.selected_id.set(Some(id));
            }
        });
    }

    // Toggle a topic row's selection (the center filter).
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let records = records.clone();
        let topics = topics.clone();
        app.on_toggle_topic(move |name| {
            if let Some(app) = app_weak.upgrade() {
                state.selection.borrow_mut().toggle(&name);
                apply_selection(&app, &state, &records, &topics);
            }
        });
    }

    // Clicking the topics title clears the selection.
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let records = records.clone();
        let topics = topics.clone();
        app.on_clear_topics(move || {
            if let Some(app) = app_weak.upgrade() {
                state.selection.borrow_mut().clear();
                apply_selection(&app, &state, &records, &topics);
            }
        });
    }

    // Right-click opens the context menu at the cursor.
    {
        let app_weak = app.as_weak();
        app.on_topic_menu(move |x, y| {
            if let Some(app) = app_weak.upgrade() {
                app.set_topic_menu_x(x);
                app.set_topic_menu_y(y);
                app.set_topic_menu_visible(true);
            }
        });
    }

    // Dismiss the context menu.
    {
        let app_weak = app.as_weak();
        app.on_dismiss_topic_menu(move || {
            if let Some(app) = app_weak.upgrade() {
                app.set_topic_menu_visible(false);
            }
        });
    }

    // The context menu's "Unselect All".
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let records = records.clone();
        let topics = topics.clone();
        app.on_unselect_all_topics(move || {
            if let Some(app) = app_weak.upgrade() {
                state.selection.borrow_mut().clear();
                app.set_topic_menu_visible(false);
                apply_selection(&app, &state, &records, &topics);
            }
        });
    }

    // Dismiss the error dialog.
    {
        let app_weak = app.as_weak();
        app.on_dismiss_error(move || {
            if let Some(app) = app_weak.upgrade() {
                app.set_error_message("".into());
            }
        });
    }

    // Switch between the Capture and Send tabs.
    {
        let app_weak = app.as_weak();
        app.on_select_tab(move |index| {
            if let Some(app) = app_weak.upgrade() {
                app.set_tab(index);
            }
        });
    }

    // Choose the topic to send.
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        app.on_select_send_topic(move |index| {
            if let Some(app) = app_weak.upgrade() {
                select_send_topic(&app, &state, index);
            }
        });
    }

    // Text and number edits are kept in the Rust form, not the Slint model, so
    // typing does not disturb the input cursor.
    {
        let state = state.clone();
        app.on_send_field_edited(move |path, text| {
            let mut send = state.send.borrow_mut();
            if let Some(form) = send.form.as_mut() {
                form.set_text(path.as_str(), text.to_string());
            }
        });
    }

    {
        let state = state.clone();
        app.on_send_field_toggled(move |path, value| {
            let mut send = state.send.borrow_mut();
            if let Some(form) = send.form.as_mut() {
                form.set_bool(path.as_str(), value);
            }
        });
    }

    // Apply an interval change immediately when repeating.
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        app.on_send_interval_edited(move |value| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let value = value.max(1) as u64;
            let repeating = {
                let mut send = state.send.borrow_mut();
                send.interval_ms = value;
                send.repeating
            };
            app.set_send_interval_ms(value as i32);
            if repeating {
                stop_sending(&app, &state);
                start_sending(&app, &state);
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let state = state.clone();
        app.on_send_once(move || {
            if let Some(app) = app_weak.upgrade() {
                let timer = state.send.borrow().timer.clone();
                send_now(&app, &state, &Rc::downgrade(&timer));
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let state = state.clone();
        app.on_send_start(move || {
            if let Some(app) = app_weak.upgrade() {
                start_sending(&app, &state);
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let state = state.clone();
        app.on_send_stop(move || {
            if let Some(app) = app_weak.upgrade() {
                stop_sending(&app, &state);
            }
        });
    }

    app.run()
}

#[cfg(test)]
mod copy_detail_tests {
    use super::*;
    use crate::model::{Announcement, Class, Frame, MessageKind};
    use stations_dds::cyclonedds::{CdrEncoding, CdrSerializer, DdsSequence};
    use stations_dds::format_manager::{FormatManagerCtlTopic, FormatWindowAssignment};
    use stations_dds::hello_world::HelloWorldTopic;
    use stations_dds::math::Quaternion;

    fn record() -> model::Record {
        model::Record {
            elapsed_ms: 1234,
            class: Class::Rtps,
            src_addr: "[fe80::1]:7401".to_string(),
            guid_prefix: Some("01108c7358111aa9089f38f5".to_string()),
            entity_id: Some("00000102".to_string()),
            dest_port: 7401,
            size: 4,
            topic: Some("HelloWorldTopic".to_string()),
            submessages: "INFO_TS, DATA".to_string(),
            hex: vec![0x00, 0x01, 0x00, 0x00],
            frames: Vec::new(),
            self_origin: true,
            kind: MessageKind::KnownTopic,
        }
    }

    fn data_frame(payload: Option<std::ops::Range<usize>>) -> Frame {
        Frame {
            id: 0x15,
            kind: "DATA".to_string(),
            writer: "00000102".to_string(),
            reader: String::new(),
            seq: "1".to_string(),
            len: 0,
            announcement: None,
            payload,
        }
    }

    #[test]
    fn detail_text_contains_self_sender_and_fields() {
        let text = detail_text(&record(), &KnownTopics::default());
        assert!(text.contains("time: 00:01.234"));
        assert!(text.contains("protocol: RTPS"));
        assert!(text.contains("source: self ([fe80::1]:7401)"));
        assert!(text.contains("participant: 01108c7358111aa9089f38f5"));
        assert!(text.contains("writer: 00000102"));
        assert!(text.contains("size: 4"));
        assert!(text.contains("topic: HelloWorldTopic"));
        assert!(text.contains("submessages: INFO_TS, DATA"));
        assert!(text.contains("hex:"));
    }

    #[test]
    fn detail_text_contains_discovery_announcements() {
        let mut r = record();
        r.frames = vec![Frame {
            id: 0x15,
            kind: "DATA".to_string(),
            writer: String::new(),
            reader: String::new(),
            seq: "1".to_string(),
            len: 0,
            announcement: Some(Announcement::Participant {
                prefix: "01108c7358111aa9089f38f5".to_string(),
            }),
            payload: None,
        }];
        let text = detail_text(&r, &KnownTopics::default());
        assert!(text.contains("Participant 01108c7358111aa9089f38f5"));
    }

    #[test]
    fn detail_text_shows_format_manager_sequence() {
        let sample = FormatManagerCtlTopic {
            windows: DdsSequence::from_vec(vec![FormatWindowAssignment {
                window: 0,
                format: "default".to_string(),
            }])
            .unwrap(),
        };
        let body = CdrSerializer::serialize(&sample, CdrEncoding::Xcdr1).unwrap();
        let mut payload = vec![0x00, 0x01, 0x00, 0x00];
        payload.extend_from_slice(&body);

        let mut r = record();
        r.topic = Some("FormatManagerCtlTopic".to_string());
        r.size = payload.len();
        r.hex = payload.clone();
        r.frames = vec![data_frame(Some(0..payload.len()))];

        let mut known = KnownTopics::default();
        known.insert(
            "FormatManagerCtlTopic",
            "format_manager::FormatManagerCtlTopic",
        );
        let text = detail_text(&r, &known);
        assert!(text.contains("decoded:"));
        assert!(text.contains("default"));
        assert!(text.contains("window: 0"));
    }

    #[test]
    fn detail_text_contains_decoded_fields() {
        let sample = HelloWorldTopic {
            greeting: "copy-test".to_string(),
            attitude: Quaternion {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
        };
        let body = CdrSerializer::serialize(&sample, CdrEncoding::Xcdr1).unwrap();
        let mut payload = vec![0x00, 0x01, 0x00, 0x00];
        payload.extend_from_slice(&body);

        let mut r = record();
        r.size = payload.len();
        r.hex = payload.clone();
        r.frames = vec![data_frame(Some(0..payload.len()))];

        let mut known = KnownTopics::default();
        known.insert("HelloWorldTopic", "hello_world::HelloWorldTopic");
        let text = detail_text(&r, &known);
        assert!(text.contains("decoded:"));
        assert!(text.contains("copy-test"));
        assert!(text.contains("attitude"));
    }
}

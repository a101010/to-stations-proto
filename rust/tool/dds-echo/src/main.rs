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

mod ui {
    slint::include_modules!();
}

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use slint::{ComponentHandle as _, ModelRc, Timer, TimerMode, VecModel};

use crate::capture::{Capture, CaptureConfig, CaptureQueue, DRAIN_MAX_PER_TICK};
use crate::model::{
    Discovery, History, KnownTopics, RateTracker, Stats, TopicCounts, TopicSelection,
};
use crate::ui::MainWindow;

use stations_dds::contract::TopicRegistry;
use stations_dds::cyclonedds::{BuiltinEndpointSample, DataReader};
use stations_dds::dds::Node;

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

    /// Topic registry (`contracts/topics.toml`).
    #[arg(long, default_value = concat!(env!("REPO_ROOT"), "/contracts/topics.toml"))]
    registry: String,

    /// CycloneDDS configuration XML for the discovery participant.
    #[arg(long)]
    dds_config: Option<PathBuf>,

    /// Disable the discovery participant and use passive capture only.
    #[arg(long)]
    no_discovery: bool,
}

/// A discovery-only participant and the builtin readers that identify writers.
struct DiscoverySource {
    // Declared before the node so the readers drop before the participant.
    publications: DataReader<BuiltinEndpointSample>,
    subscriptions: DataReader<BuiltinEndpointSample>,
    _node: Node,
    last_poll_ms: u64,
}

impl DiscoverySource {
    fn new(domain: u32, registry: TopicRegistry) -> Result<Self, String> {
        let node = Node::new(domain, registry).map_err(|e| e.to_string())?;
        let participant = node.participant();
        let publications = participant
            .create_builtin_publication_reader()
            .map_err(|e| e.to_string())?;
        let subscriptions = participant
            .create_builtin_subscription_reader()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            _node: node,
            publications,
            subscriptions,
            last_poll_ms: 0,
        })
    }

    fn poll(&mut self, now_ms: u64, discovery: &mut Discovery, force: bool) -> usize {
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
                for sample in &samples {
                    learned += learn_builtin_endpoint(
                        discovery,
                        sample,
                        model::EndpointKind::Writer,
                    );
                }
            }
            Err(e) => eprintln!("dds-echo discovery: publications read failed: {e}"),
        }
        match self.subscriptions.read() {
            Ok(samples) => {
                for sample in &samples {
                    learned += learn_builtin_endpoint(
                        discovery,
                        sample,
                        model::EndpointKind::Reader,
                    );
                }
            }
            Err(e) => eprintln!("dds-echo discovery: subscriptions read failed: {e}"),
        }
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
) -> usize {
    let topic = sample.topic_name();
    if topic.is_empty() || topic.starts_with("DCPS") {
        return 0;
    }
    let key = sample.key();
    let participant_key = sample.participant_key();
    let endpoint = model::Endpoint {
        guid: rtps::hex(&key.v),
        prefix: rtps::hex(&participant_key.v[..12]),
        kind,
        topic,
        type_name: sample.type_name_value(),
    };
    usize::from(discovery.learn_endpoint(endpoint))
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
    /// The registry's topics (name -> IDL type), loaded at startup.
    known_topics: KnownTopics,
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
    }
}

/// The lightweight table row; hex, decoded fields, and frames are deferred to
/// [`to_detail`].
fn to_row(r: &model::Record, known: &KnownTopics) -> ui::RecordRow {
    ui::RecordRow {
        time: r.time_label().into(),
        protocol: r.protocol().into(),
        source: r.src_addr.clone().into(),
        size: r.size as i32,
        topic: r.topic.clone().unwrap_or_default().into(),
        submessages: r.submessages.clone().into(),
        known: r.is_known(known),
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
        source: r.src_addr.clone().into(),
        size: r.size as i32,
        topic: r.topic.clone().unwrap_or_default().into(),
        submessages: r.submessages.clone().into(),
        hex: r.hex_string().into(),
        decoded: decoded_text(r, known).into(),
        guid: r.guid_prefix.clone().unwrap_or_default().into(),
        entity: r.entity_id.clone().unwrap_or_default().into(),
        known: r.is_known(known),
        frames: ModelRc::from(frames.as_slice()),
    }
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
    for (id, record) in history.matching(&selection) {
        if Some(id) == selected {
            selected_index = rows.len() as i32;
        }
        rows.push(to_row(record, &state.known_topics));
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

/// Re-apply the current selection: refresh the topics rows and rebuild the
/// record view.
fn apply_selection(
    app: &MainWindow,
    state: &State,
    records: &VecModel<ui::RecordRow>,
    known: &VecModel<ui::TopicRow>,
    unknown: &VecModel<ui::TopicRow>,
) {
    refresh_topics(
        app,
        &state.counts.borrow().rows(&state.known_topics),
        &state.selection.borrow(),
        known,
        unknown,
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
    known: &VecModel<ui::TopicRow>,
    unknown: &VecModel<ui::TopicRow>,
) {
    known.clear();
    unknown.clear();
    let mut known_total = 0i32;
    let mut unknown_total = 0i32;
    for row in rows {
        let packets = row.packets as i32;
        let ui_row = ui::TopicRow {
            name: row.name.clone().into(),
            known: row.known,
            packets,
            selected: selection.contains(&row.name),
        };
        if row.known {
            known_total += packets;
            known.push(ui_row);
        } else {
            unknown_total += packets;
            unknown.push(ui_row);
        }
    }
    app.set_known_total(known_total);
    app.set_unknown_total(unknown_total);
}

/// Poll the discovery readers and re-attribute retained records when new
/// endpoints are learned. Returns true when topic counts changed.
fn poll_discovery(
    app: &MainWindow,
    state: &State,
    known: &VecModel<ui::TopicRow>,
    unknown: &VecModel<ui::TopicRow>,
    force: bool,
) -> bool {
    let now_ms = state.start.get().elapsed().as_millis() as u64;
    let learned = {
        let mut source = state.discovery_source.borrow_mut();
        match source.as_mut() {
            Some(source) => {
                let mut discovery = state.discovery.borrow_mut();
                source.poll(now_ms, &mut discovery, force)
            }
            None => 0,
        }
    };
    if learned == 0 {
        return false;
    }

    let changes = {
        let discovery = state.discovery.borrow();
        state.history.borrow_mut().reattribute(&discovery)
    };
    if changes.is_empty() {
        return false;
    }

    {
        let mut counts = state.counts.borrow_mut();
        for (old, new) in &changes {
            counts.reattribute(old.as_deref(), new);
        }
    }
    state.pending_view.set(true);
    refresh_topics(
        app,
        &state.counts.borrow().rows(&state.known_topics),
        &state.selection.borrow(),
        known,
        unknown,
    );
    true
}

fn tick(
    app: &MainWindow,
    state: &State,
    records: &VecModel<ui::RecordRow>,
    known: &VecModel<ui::TopicRow>,
    unknown: &VecModel<ui::TopicRow>,
) {
    let now_ms = state.start.get().elapsed().as_millis() as u64;

    poll_discovery(app, state, known, unknown, false);

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
                state.stats.borrow_mut().observe(&record);
                state.counts.borrow_mut().observe(&record);
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
        &state.counts.borrow().rows(&state.known_topics),
        &state.selection.borrow(),
        known,
        unknown,
    );

    {
        let discovery = state.discovery.borrow();
        app.set_participants(discovery.participants() as i32);
        app.set_endpoints(discovery.endpoints() as i32);
    }

    let rates = state.rates.borrow_mut().sample(&state.stats.borrow(), now_ms);
    let stats = state.stats.borrow();
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
fn load_registry(path: &str) -> Option<TopicRegistry> {
    match TopicRegistry::load(Path::new(path)) {
        Ok(registry) => Some(registry),
        Err(e) => {
            eprintln!("dds-echo: failed to load registry {path}: {e}");
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

/// Point CycloneDDS at the shared StationsDDS configuration before creating the
/// discovery participant. An explicit path wins; otherwise an existing
/// `CYCLONEDDS_URI` is respected, then the deployed or source configuration.
fn install_dds_config(explicit: Option<&Path>) {
    let path = if let Some(path) = explicit {
        Some(path.to_path_buf())
    } else if std::env::var_os("CYCLONEDDS_URI").is_some() {
        None
    } else {
        default_dds_config()
    };

    if let Some(path) = path {
        let path = path.to_string_lossy().replace('\\', "/");
        std::env::set_var("CYCLONEDDS_URI", format!("file://{path}"));
    } else if std::env::var_os("CYCLONEDDS_URI").is_none() {
        eprintln!("dds-echo: no CycloneDDS config found; discovery uses defaults");
    }
}

fn default_dds_config() -> Option<PathBuf> {
    let deployed = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .and_then(|bin| bin.parent().map(|deploy| deploy.join("config")))
        .map(|config| config.join("cyclonedds-config.xml"));
    if deployed.as_ref().is_some_and(|path| path.is_file()) {
        return deployed;
    }

    let source = PathBuf::from(env!("REPO_ROOT"))
        .join("DDS")
        .join("cyclonedds-config.xml");
    source.is_file().then_some(source)
}

fn main() -> Result<(), slint::PlatformError> {
    let args = Args::parse();
    let registry = load_registry(&args.registry);
    let known_topics = registry
        .as_ref()
        .map(known_from_registry)
        .unwrap_or_default();
    let discovery_source = if args.no_discovery {
        None
    } else {
        match registry {
            Some(registry) => {
                install_dds_config(args.dds_config.as_deref());
                match DiscoverySource::new(args.domain, registry) {
                    Ok(source) => Some(source),
                    Err(e) => {
                        eprintln!("dds-echo: discovery participant disabled: {e}");
                        None
                    }
                }
            }
            None => {
                eprintln!("dds-echo: discovery participant disabled: registry unavailable");
                None
            }
        }
    };

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
        known_topics,
    });

    let records = Rc::new(VecModel::<ui::RecordRow>::default());
    let known = Rc::new(VecModel::<ui::TopicRow>::default());
    let unknown = Rc::new(VecModel::<ui::TopicRow>::default());
    app.set_records(ModelRc::from(records.clone()));
    app.set_known_topics(ModelRc::from(known.clone()));
    app.set_unknown_topics(ModelRc::from(unknown.clone()));
    refresh_topics(
        &app,
        &state.counts.borrow().rows(&state.known_topics),
        &state.selection.borrow(),
        &known,
        &unknown,
    );

    let timer = Rc::new(Timer::default());

    // Start.
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let records = records.clone();
        let known = known.clone();
        let unknown = unknown.clone();
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
                        &state.counts.borrow().rows(&state.known_topics),
                        &state.selection.borrow(),
                        &known,
                        &unknown,
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
                    let known = known.clone();
                    let unknown = unknown.clone();
                    timer.start(TimerMode::Repeated, TICK, move || {
                        if let Some(app) = app_weak.upgrade() {
                            tick(&app, &state, &records, &known, &unknown);
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
        let known = known.clone();
        let unknown = unknown.clone();
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
                poll_discovery(&app, &state, &known, &unknown, true);
                refresh_topics(
                    &app,
                    &state.counts.borrow().rows(&state.known_topics),
                    &state.selection.borrow(),
                    &known,
                    &unknown,
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
        let known = known.clone();
        let unknown = unknown.clone();
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
                    &state.counts.borrow().rows(&state.known_topics),
                    &state.selection.borrow(),
                    &known,
                    &unknown,
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
        let known = known.clone();
        let unknown = unknown.clone();
        app.on_toggle_topic(move |name| {
            if let Some(app) = app_weak.upgrade() {
                state.selection.borrow_mut().toggle(&name);
                apply_selection(&app, &state, &records, &known, &unknown);
            }
        });
    }

    // Clicking the topics title clears the selection.
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let records = records.clone();
        let known = known.clone();
        let unknown = unknown.clone();
        app.on_clear_topics(move || {
            if let Some(app) = app_weak.upgrade() {
                state.selection.borrow_mut().clear();
                apply_selection(&app, &state, &records, &known, &unknown);
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
        let known = known.clone();
        let unknown = unknown.clone();
        app.on_unselect_all_topics(move || {
            if let Some(app) = app_weak.upgrade() {
                state.selection.borrow_mut().clear();
                app.set_topic_menu_visible(false);
                apply_selection(&app, &state, &records, &known, &unknown);
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

    app.run()
}

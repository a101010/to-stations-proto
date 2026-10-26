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

/// The UI refresh / capture-drain tick.
const TICK: Duration = Duration::from_millis(100);

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
}

/// Shared state behind the UI.
struct State {
    capture: RefCell<Option<Capture>>,
    queue: RefCell<Option<Arc<CaptureQueue>>>,
    stats: RefCell<Stats>,
    counts: RefCell<TopicCounts>,
    discovery: RefCell<Discovery>,
    rates: RefCell<RateTracker>,
    start: Cell<Instant>,
    following: Cell<bool>,
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
fn to_row(r: &model::Record, known: &KnownTopics) -> ui::Record {
    ui::Record {
        time: r.time_label().into(),
        protocol: r.protocol().into(),
        source: r.src_addr.clone().into(),
        size: r.size as i32,
        topic: r.topic.clone().unwrap_or_default().into(),
        submessages: r.submessages.clone().into(),
        hex: "".into(),
        decoded: "".into(),
        guid: "".into(),
        entity: "".into(),
        known: r.is_known(known),
        frames: ModelRc::default(),
    }
}

/// The full detail model, built only when a row is selected.
fn to_detail(r: &model::Record, known: &KnownTopics) -> ui::Record {
    let mut row = to_row(r, known);
    row.guid = r.guid_prefix.clone().unwrap_or_default().into();
    row.entity = r.entity_id.clone().unwrap_or_default().into();
    row.hex = r.hex_string().into();
    row.decoded = decoded_text(r, known).into();
    let frames: Vec<ui::Frame> = r
        .frames
        .iter()
        .map(|f| ui::Frame {
            line: frame_line(f).into(),
            detail: frame_detail(f).into(),
        })
        .collect();
    row.frames = ModelRc::from(frames.as_slice());
    row
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
fn rebuild_view(app: &MainWindow, state: &State, records: &VecModel<ui::Record>) {
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
    records: &VecModel<ui::Record>,
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

fn tick(
    app: &MainWindow,
    state: &State,
    records: &VecModel<ui::Record>,
    known: &VecModel<ui::TopicRow>,
    unknown: &VecModel<ui::TopicRow>,
) {
    let now_ms = state.start.get().elapsed().as_millis() as u64;

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
        rebuild_view(app, state, records);
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

    // Follow the tail while the user has not scrolled up.
    let visible_h = app.get_list_visible_height();
    if state.following.get() && visible_h > 0.0 {
        let max_scroll = (app.get_list_content_height() - visible_h).max(0.0);
        app.set_list_content_y(-max_scroll);
    }
}

/// Load the topic registry into `KnownTopics`; on failure, log and return an
/// empty set (everything then shows as Unknown/hex).
fn load_known(path: &str) -> KnownTopics {
    let mut known = KnownTopics::default();
    match stations_dds::contract::TopicRegistry::load(std::path::Path::new(path)) {
        Ok(registry) => {
            for topic in &registry.topics {
                known.insert(&topic.name, &topic.idl_type);
            }
        }
        Err(e) => eprintln!("dds-echo: failed to load registry {path}: {e}"),
    }
    known
}

fn main() -> Result<(), slint::PlatformError> {
    let args = Args::parse();
    let app = MainWindow::new()?;
    let known_topics = load_known(&args.registry);

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
        stats: RefCell::new(Stats::default()),
        counts: RefCell::new(TopicCounts::default()),
        discovery: RefCell::new(Discovery::default()),
        rates: RefCell::new(RateTracker::default()),
        start: Cell::new(Instant::now()),
        following: Cell::new(true),
        selection: RefCell::new(TopicSelection::default()),
        history: RefCell::new(History::default()),
        displayed_ids: RefCell::new(Vec::new()),
        selected_id: Cell::new(None),
        dropped: Cell::new(0),
        trimmed: Cell::new(0),
        known_topics,
    });

    let records = Rc::new(VecModel::<ui::Record>::default());
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
                    state.following.set(true);

                    state.stats.borrow_mut().reset();
                    state.counts.borrow_mut().reset();
                    state.discovery.borrow_mut().reset();
                    state.history.borrow_mut().clear();
                    state.displayed_ids.borrow_mut().clear();
                    state.selected_id.set(None);
                    state.dropped.set(0);
                    state.trimmed.set(0);

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
                state.discovery.borrow_mut().reset();
                *state.rates.borrow_mut() = RateTracker::default();
                state.selected_id.set(None);
                state.dropped.set(0);
                state.trimmed.set(0);
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

    // Follow-the-tail: the ScrollView's `scrolled` fires only on user scroll.
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        app.on_list_scrolled(move || {
            if let Some(app) = app_weak.upgrade() {
                let cy = app.get_list_content_y();
                let ch = app.get_list_content_height();
                let vh = app.get_list_visible_height();
                state.following.set(cy <= -(ch - vh) + 1.0);
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

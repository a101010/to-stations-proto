//! The `dds-echo` traffic inspector (Slint).
//!
//! A real UDP capture backend (see [`capture`]) feeds the UI through a channel
//! drained on the event loop, so Slint models are only touched on the UI thread.

// Release builds are a GUI application (no console window); debug builds keep
// the console for diagnostics.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod capture;
mod model;
mod rtps;
mod sedp;

mod ui {
    slint::include_modules!();
}

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use clap::Parser;
use slint::{ComponentHandle as _, Model as _, ModelRc, Timer, TimerMode, VecModel};

use crate::capture::{Capture, CaptureConfig};
use crate::model::{Discovery, RateTracker, Stats, TopicCounts, TopicSelection, MAX_RECORDS};
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
}

/// Shared state behind the UI.
struct State {
    capture: RefCell<Option<Capture>>,
    rx: RefCell<Option<Receiver<model::Record>>>,
    stats: RefCell<Stats>,
    counts: RefCell<TopicCounts>,
    discovery: RefCell<Discovery>,
    rates: RefCell<RateTracker>,
    start: Cell<Instant>,
    following: Cell<bool>,
    /// The topic rows selected in the topics pane (the center filter).
    selection: RefCell<TopicSelection>,
    /// The full, capped list of records; the displayed model is the filtered view.
    all_records: RefCell<Vec<ui::Record>>,
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

fn to_ui(r: &model::Record) -> ui::Record {
    ui::Record {
        time: r.time_label().into(),
        protocol: r.protocol().into(),
        source: r.src_addr.clone().into(),
        size: r.size as i32,
        topic: r.topic.clone().unwrap_or_default().into(),
        submessages: r.submessages.clone().into(),
        hex: r.hex_string().into(),
        decoded: r.decoded.clone().into(),
        guid: r.guid_prefix.clone().unwrap_or_default().into(),
        entity: r.entity_id.clone().unwrap_or_default().into(),
        known: r.is_known(),
        frames: {
            let frames: Vec<ui::Frame> = r
                .frames
                .iter()
                .map(|f| ui::Frame {
                    line: frame_line(f).into(),
                    detail: frame_detail(f).into(),
                })
                .collect();
            ModelRc::from(frames.as_slice())
        },
    }
}

/// The filter name of a displayed record: its topic, or `unidentified`.
fn ui_topic(r: &ui::Record) -> Option<&str> {
    if r.topic.is_empty() {
        None
    } else {
        Some(r.topic.as_str())
    }
}

/// Rebuild the displayed records from the backing list, keeping only those that
/// pass `selection`.
fn rebuild_records(view: &VecModel<ui::Record>, all: &[ui::Record], selection: &TopicSelection) {
    view.clear();
    for r in all {
        if selection.matches(ui_topic(r)) {
            view.push(r.clone());
        }
    }
    while view.row_count() > MAX_RECORDS {
        view.remove(0);
    }
}

/// Re-apply the current selection: refresh the topics rows, rebuild the record
/// view, and drop the detail selection (its index no longer lines up).
fn apply_selection(
    app: &MainWindow,
    state: &State,
    records: &VecModel<ui::Record>,
    known: &VecModel<ui::TopicRow>,
    unknown: &VecModel<ui::TopicRow>,
) {
    refresh_topics(
        app,
        &state.counts.borrow().rows(),
        &state.selection.borrow(),
        known,
        unknown,
    );
    rebuild_records(records, &state.all_records.borrow(), &state.selection.borrow());
    app.set_selected_index(-1);
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

    // Drain the capture channel.
    {
        let rx = state.rx.borrow();
        if let Some(rx) = rx.as_ref() {
            loop {
                match rx.try_recv() {
                    Ok(mut record) => {
                        state.discovery.borrow_mut().observe_and_attr(&mut record);
                        state.stats.borrow_mut().observe(&record);
                        state.counts.borrow_mut().observe(&record);

                        let matched = state.selection.borrow().matches(record.topic.as_deref());
                        let ui = to_ui(&record);
                        {
                            let mut all = state.all_records.borrow_mut();
                            all.push(ui.clone());
                            while all.len() > MAX_RECORDS {
                                all.remove(0);
                            }
                        }
                        if matched {
                            records.push(ui);
                            while records.row_count() > MAX_RECORDS {
                                records.remove(0);
                            }
                        }
                    }
                    Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
                }
            }
        }
    }

    refresh_topics(
        app,
        &state.counts.borrow().rows(),
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

    // Follow the tail while the user has not scrolled up.
    let visible_h = app.get_list_visible_height();
    if state.following.get() && visible_h > 0.0 {
        let max_scroll = (app.get_list_content_height() - visible_h).max(0.0);
        app.set_list_content_y(-max_scroll);
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let args = Args::parse();
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
        rx: RefCell::new(None),
        stats: RefCell::new(Stats::default()),
        counts: RefCell::new(TopicCounts::default()),
        discovery: RefCell::new(Discovery::default()),
        rates: RefCell::new(RateTracker::default()),
        start: Cell::new(Instant::now()),
        following: Cell::new(true),
        selection: RefCell::new(TopicSelection::default()),
        all_records: RefCell::new(Vec::new()),
    });

    let records = Rc::new(VecModel::<ui::Record>::default());
    let known = Rc::new(VecModel::<ui::TopicRow>::default());
    let unknown = Rc::new(VecModel::<ui::TopicRow>::default());
    app.set_records(ModelRc::from(records.clone()));
    app.set_known_topics(ModelRc::from(known.clone()));
    app.set_unknown_topics(ModelRc::from(unknown.clone()));
    refresh_topics(
        &app,
        &state.counts.borrow().rows(),
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

            let (tx, rx) = mpsc::channel();
            match Capture::start(&config, tx) {
                Ok(capture) => {
                    *state.capture.borrow_mut() = Some(capture);
                    *state.rx.borrow_mut() = Some(rx);
                    app.set_running(true);
                    state.start.set(Instant::now());
                    *state.rates.borrow_mut() = RateTracker::default();
                    state.following.set(true);

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
            *state.rx.borrow_mut() = None;
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
                records.clear();
                state.all_records.borrow_mut().clear();
                state.selection.borrow_mut().clear();
                state.stats.borrow_mut().reset();
                state.counts.borrow_mut().reset();
                state.discovery.borrow_mut().reset();
                *state.rates.borrow_mut() = RateTracker::default();
                refresh_topics(
                    &app,
                    &state.counts.borrow().rows(),
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
                app.set_selected_index(-1);
            }
        });
    }

    // Select.
    {
        let app_weak = app.as_weak();
        let records = records.clone();
        app.on_select(move |index| {
            if index >= 0 {
                if let Some(app) = app_weak.upgrade() {
                    if let Some(record) = records.row_data(index as usize) {
                        app.set_selected(record);
                    }
                    app.set_selected_index(index);
                }
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

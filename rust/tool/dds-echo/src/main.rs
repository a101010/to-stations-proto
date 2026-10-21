//! The `dds-echo` traffic inspector (Slint mockup).
//!
//! The UI is driven by a simulated feed for now; the capture/SEDP/decode
//! stories replace it with the real backend.

mod model;

mod ui {
    slint::include_modules!();
}

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use slint::{ComponentHandle as _, Model as _, ModelRc, Timer, TimerMode, VecModel};

use crate::model::{RateTracker, SimFeed, Stats, TopicCounts, MAX_RECORDS};
use crate::ui::MainWindow;

/// The simulated feed's tick.
const TICK: Duration = Duration::from_millis(100);

/// Shared state behind the UI.
struct State {
    feed: RefCell<SimFeed>,
    stats: RefCell<Stats>,
    counts: RefCell<TopicCounts>,
    rates: RefCell<RateTracker>,
    start: Cell<Instant>,
    following: Cell<bool>,
}

fn to_ui(r: &model::Record) -> ui::Record {
    ui::Record {
        time: r.time_label().into(),
        class: r.class.label().into(),
        source: r.src_addr.clone().into(),
        size: r.size as i32,
        topic: r.topic.clone().unwrap_or_default().into(),
        summary: r.summary.clone().into(),
        hex: r.hex_string().into(),
        decoded: r.decoded.clone().into(),
        guid: r.guid_prefix.clone().unwrap_or_default().into(),
        entity: r.entity_id.clone().unwrap_or_default().into(),
        known: r.is_known(),
    }
}

fn refresh_topics(
    app: &MainWindow,
    rows: &[model::TopicRow],
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

    let burst = state.feed.borrow_mut().burst();
    for _ in 0..burst {
        let record = state.feed.borrow_mut().next(now_ms);
        state.stats.borrow_mut().observe(&record);
        state.counts.borrow_mut().observe(&record);
        records.push(to_ui(&record));
        while records.row_count() > MAX_RECORDS {
            records.remove(0);
        }
    }

    refresh_topics(app, &state.counts.borrow().rows(), known, unknown);

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
    let app = MainWindow::new()?;

    let state = Rc::new(State {
        feed: RefCell::new(SimFeed::new(0x5eed)),
        stats: RefCell::new(Stats::default()),
        counts: RefCell::new(TopicCounts::default()),
        rates: RefCell::new(RateTracker::default()),
        start: Cell::new(Instant::now()),
        following: Cell::new(true),
    });

    let records = Rc::new(VecModel::<ui::Record>::default());
    let known = Rc::new(VecModel::<ui::TopicRow>::default());
    let unknown = Rc::new(VecModel::<ui::TopicRow>::default());
    app.set_records(ModelRc::from(records.clone()));
    app.set_known_topics(ModelRc::from(known.clone()));
    app.set_unknown_topics(ModelRc::from(unknown.clone()));
    refresh_topics(&app, &state.counts.borrow().rows(), &known, &unknown);

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
            if let Some(app) = app_weak.upgrade() {
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
        });
    }

    // Stop.
    {
        let app_weak = app.as_weak();
        let timer = timer.clone();
        app.on_stop(move || {
            if let Some(app) = app_weak.upgrade() {
                app.set_running(false);
            }
            timer.stop();
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
                state.stats.borrow_mut().reset();
                state.counts.borrow_mut().reset();
                *state.rates.borrow_mut() = RateTracker::default();
                refresh_topics(&app, &state.counts.borrow().rows(), &known, &unknown);
                app.set_packets(0);
                app.set_bytes(0);
                app.set_rtps(0);
                app.set_pps(0);
                app.set_bps(0);
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

    // Follow-the-tail: the ScrollView's `scrolled` fires only on user scroll
    // (not on our programmatic scroll), so it cleanly toggles following.
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

    app.run()
}

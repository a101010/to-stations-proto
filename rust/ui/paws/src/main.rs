#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod controls;
mod gamepad;
mod state;
mod stick;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use clap::Parser;
use gilrs::{EventType, Gilrs};
use single_instance::Acquire;
use slint::{ComponentHandle as _, ModelRc, SharedString, Timer, TimerMode, VecModel};

use controls::Controls;
use gamepad::{Choice, Selection};
use state::SavedState;

slint::include_modules!();

#[derive(Parser, Debug)]
#[command(name = "paws", about = "to-stations virtual game controller")]
struct Args {
    /// Display id this controller window belongs to.
    #[arg(long)]
    display_id: u32,
}

struct AppState {
    gilrs: Option<Gilrs>,
    choices: Vec<Choice>,
    selection: Selection,
    remembered: Option<state::SavedController>,
    state_path: Option<PathBuf>,
    model: Rc<VecModel<SharedString>>,
    last_controls: Controls,
}

impl AppState {
    fn new(display_id: u32) -> Self {
        let (state_path, saved) = SavedState::load(display_id);
        let gilrs = Gilrs::new().ok();
        let mut choices = Vec::new();
        if let Some(gilrs) = &gilrs {
            choices = gamepad::xbox_choices(gilrs);
        }
        let selection = saved
            .controller
            .as_ref()
            .and_then(|saved| gamepad::find_remembered(&choices, saved))
            .map(Selection::Gamepad)
            .unwrap_or(Selection::Virtual);
        let model = Rc::new(VecModel::default());
        gamepad::update_model(&choices, &model);
        Self {
            gilrs,
            choices,
            selection,
            remembered: saved.controller,
            state_path,
            model,
            last_controls: Controls::neutral(),
        }
    }

    fn save(&self) {
        SavedState {
            controller: self.remembered.clone(),
        }
        .save(self.state_path.as_deref());
    }

    fn apply_selection(&self, ui: &MainWindow) {
        ui.set_controller_index(gamepad::index_for_selection(self.selection, &self.choices));
        ui.set_virtual_mode(matches!(self.selection, Selection::Virtual));
    }

    fn apply_current_controls(&mut self, ui: &MainWindow) {
        if let Selection::Gamepad(id) = self.selection {
            if let Some(gilrs) = &self.gilrs {
                if let Some(pad) = gilrs.connected_gamepad(id) {
                    let controls = Controls::read(&pad);
                    controls.apply(ui);
                    self.last_controls = controls;
                    return;
                }
            }
        }
        let neutral = Controls::neutral();
        neutral.apply(ui);
        self.last_controls = neutral;
    }

    fn select(&mut self, index: i32, ui: &MainWindow) {
        let selection = gamepad::selection_for_index(index, &self.choices);
        self.remembered = match selection {
            Selection::Virtual => None,
            Selection::Gamepad(id) => self
                .choices
                .iter()
                .find(|choice| choice.id == id)
                .map(Choice::saved),
        };
        self.selection = selection;
        self.save();
        self.apply_selection(ui);
        self.apply_current_controls(ui);
    }

    fn poll(&mut self, ui: &MainWindow) {
        let Some(gilrs) = self.gilrs.as_mut() else {
            return;
        };

        let mut list_changed = false;
        while let Some(event) = gilrs.next_event() {
            match event.event {
                EventType::Connected => list_changed = true,
                EventType::Disconnected => {
                    if let Selection::Gamepad(id) = self.selection {
                        if id == event.id {
                            self.selection = Selection::Virtual;
                        }
                    }
                    list_changed = true;
                }
                _ => {}
            }
        }

        if list_changed {
            gamepad::refresh_choices(gilrs, &mut self.choices, &self.model);

            // Only the remembered controller is ever auto-selected.
            if let Some(saved) = &self.remembered {
                if let Some(id) = gamepad::find_remembered(&self.choices, saved) {
                    self.selection = Selection::Gamepad(id);
                }
            }

            if let Selection::Gamepad(id) = self.selection {
                if !self.choices.iter().any(|choice| choice.id == id) {
                    self.selection = Selection::Virtual;
                }
            }

            ui.set_controller_index(gamepad::index_for_selection(self.selection, &self.choices));
            ui.set_virtual_mode(matches!(self.selection, Selection::Virtual));
            let neutral = Controls::neutral();
            neutral.apply(ui);
            self.last_controls = neutral;
        }

        if let Selection::Gamepad(id) = self.selection {
            if let Some(pad) = gilrs.connected_gamepad(id) {
                let controls = Controls::read(&pad);
                if controls != self.last_controls {
                    controls.apply(ui);
                    self.last_controls = controls;
                }
            } else {
                self.selection = Selection::Virtual;
                ui.set_controller_index(0);
                ui.set_virtual_mode(true);
                let neutral = Controls::neutral();
                neutral.apply(ui);
                self.last_controls = neutral;
            }
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let args = Args::parse();

    let _guard = match single_instance::acquire(&format!("paws-display-{}", args.display_id)) {
        Acquire::Primary(guard) => guard,
        Acquire::Duplicate => {
            eprintln!("paws: display id {} is already running", args.display_id);
            return Ok(());
        }
    };

    let ui = MainWindow::new()?;
    ui.set_display_id(args.display_id as i32);

    let state = Rc::new(RefCell::new(AppState::new(args.display_id)));
    {
        let st = state.borrow();
        ui.set_controllers(ModelRc::from(st.model.clone()));
        st.apply_selection(&ui);
    }
    state.borrow_mut().apply_current_controls(&ui);

    let weak = ui.as_weak();
    let cb_state = state.clone();
    ui.on_stick_drag(move |index, dx, dy, radius| {
        if !matches!(cb_state.borrow().selection, Selection::Virtual) {
            return;
        }
        if let Some(ui) = weak.upgrade() {
            let (x, y) = stick::clamp_stick(dx, dy, radius);
            match index {
                0 => {
                    ui.set_left_x(x);
                    ui.set_left_y(y);
                }
                1 => {
                    ui.set_right_x(x);
                    ui.set_right_y(y);
                }
                _ => {}
            }
        }
    });

    let weak = ui.as_weak();
    let cb_state = state.clone();
    ui.on_stick_release(move |index| {
        if !matches!(cb_state.borrow().selection, Selection::Virtual) {
            return;
        }
        if let Some(ui) = weak.upgrade() {
            match index {
                0 => {
                    ui.set_left_x(0.0);
                    ui.set_left_y(0.0);
                }
                1 => {
                    ui.set_right_x(0.0);
                    ui.set_right_y(0.0);
                }
                _ => {}
            }
        }
    });

    let weak = ui.as_weak();
    let cb_state = state.clone();
    ui.on_control_pressed(move |control, down| {
        if !matches!(cb_state.borrow().selection, Selection::Virtual) {
            return;
        }
        if let Some(ui) = weak.upgrade() {
            controls::set_virtual_button(&ui, control.as_str(), down);
        }
    });

    let weak = ui.as_weak();
    let cb_state = state.clone();
    ui.on_controller_selected(move |index| {
        if let Some(ui) = weak.upgrade() {
            cb_state.borrow_mut().select(index, &ui);
        }
    });

    let timer = Timer::default();
    let weak = ui.as_weak();
    let timer_state = state.clone();
    timer.start(TimerMode::Repeated, Duration::from_millis(16), move || {
        if let Some(ui) = weak.upgrade() {
            timer_state.borrow_mut().poll(&ui);
        }
    });

    ui.run()
}

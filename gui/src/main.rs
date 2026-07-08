//! Rusty Wire desktop GUI (iced) — the 3.x front-end, built on the same I/O-free
//! app-layer API as the CLI and TUI (via the `rusty_wire::prelude`).
//!
//! First pass: pick the antenna model, calculation mode, ground class, height and
//! velocity factor, and see the same results document the CLI/TUI render. Lives in
//! a standalone crate so iced's dependency tree stays out of the main package.
//!
//! Run with: `cd gui && cargo run --release`

use std::str::FromStr;

use iced::widget::{button, column, pick_list, row, scrollable, slider, text};
use iced::{Element, Length, Task};

use rusty_wire::app::AntennaModel;
use rusty_wire::prelude::*;

const ANTENNAS: &[&str] = &[
    "dipole",
    "inverted-v",
    "efhw",
    "loop",
    "ocfd",
    "trap-dipole",
];
const MODES: &[&str] = &["resonant", "non-resonant"];
const GROUNDS: &[&str] = &["poor", "average", "good"];
const HEIGHTS: &[&str] = &["7", "10", "12"];

struct Gui {
    antenna: String,
    mode: String,
    ground: String,
    height: String,
    velocity: f32,
    lines: Vec<String>,
}

impl Default for Gui {
    fn default() -> Self {
        let mut gui = Self {
            antenna: "dipole".into(),
            mode: "resonant".into(),
            ground: "average".into(),
            height: "10".into(),
            velocity: 1.0,
            lines: Vec::new(),
        };
        gui.recalculate();
        gui
    }
}

#[derive(Debug, Clone)]
enum Message {
    Antenna(String),
    Mode(String),
    Ground(String),
    Height(String),
    Velocity(f32),
    Recalculate,
}

impl Gui {
    fn config(&self) -> AppConfig {
        AppConfig {
            antenna_model: AntennaModel::from_str(&self.antenna).ok(),
            mode: if self.mode == "non-resonant" {
                CalcMode::NonResonant
            } else {
                CalcMode::Resonant
            },
            ground_class: match self.ground.as_str() {
                "poor" => GroundClass::Poor,
                "good" => GroundClass::Good,
                _ => GroundClass::Average,
            },
            antenna_height_m: self.height.parse().unwrap_or(10.0),
            velocity_factor: self.velocity as f64,
            ..AppConfig::default()
        }
    }

    fn recalculate(&mut self) {
        let results = run_calculation(self.config());
        let doc = results_display_document(&results);

        let mut lines = vec![doc.overview_heading.to_string()];
        lines.extend(doc.overview_header_lines);
        for bv in doc.band_views {
            lines.push(String::new());
            lines.push(bv.title);
            lines.extend(bv.lines);
        }
        lines.extend(doc.summary_lines);
        for section in doc.sections {
            lines.push(String::new());
            lines.extend(section.lines);
        }
        if !doc.warning_lines.is_empty() {
            lines.push(String::new());
            lines.extend(doc.warning_lines);
        }
        self.lines = lines;
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Antenna(v) => self.antenna = v,
            Message::Mode(v) => self.mode = v,
            Message::Ground(v) => self.ground = v,
            Message::Height(v) => self.height = v,
            Message::Velocity(v) => self.velocity = v,
            Message::Recalculate => {}
        }
        self.recalculate();
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let opts = |labels: &[&str]| labels.iter().map(|s| s.to_string()).collect::<Vec<_>>();

        let controls = row![
            column![
                text("Antenna"),
                pick_list(opts(ANTENNAS), Some(self.antenna.clone()), Message::Antenna),
            ]
            .spacing(4),
            column![
                text("Mode"),
                pick_list(opts(MODES), Some(self.mode.clone()), Message::Mode),
            ]
            .spacing(4),
            column![
                text("Ground"),
                pick_list(opts(GROUNDS), Some(self.ground.clone()), Message::Ground),
            ]
            .spacing(4),
            column![
                text("Height (m)"),
                pick_list(opts(HEIGHTS), Some(self.height.clone()), Message::Height),
            ]
            .spacing(4),
        ]
        .spacing(16);

        let vf = column![
            text(format!("Velocity factor: {:.2}", self.velocity)),
            slider(0.50..=1.0, self.velocity, Message::Velocity).step(0.01_f32),
        ]
        .spacing(4);

        let results = scrollable(text(self.lines.join("\n")).font(iced::Font::MONOSPACE))
            .width(Length::Fill)
            .height(Length::Fill);

        column![
            text("Rusty Wire").size(28),
            controls,
            vf,
            button("Recalculate").on_press(Message::Recalculate),
            results,
        ]
        .spacing(16)
        .padding(20)
        .into()
    }
}

fn main() -> iced::Result {
    iced::application("Rusty Wire", Gui::update, Gui::view).run()
}

mod model;
mod view;

use std::path::PathBuf;

use anyhow::{Result, anyhow};
use eframe::egui;

use model::Model;

/// Opens the timeline window. `file` is optional; without it the window starts empty.
pub fn run(file: Option<PathBuf>) -> Result<()> {
    let model = file.as_deref().map(Model::load).transpose()?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("macos-history-diff")
            .with_inner_size([1100.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "macos-history-diff",
        options,
        Box::new(move |_cc| Ok(Box::new(App { model, error: None }))),
    )
    .map_err(|e| anyhow!("starting window: {e}"))
}

struct App {
    model: Option<Model>,
    error: Option<String>,
}

impl App {
    fn open(&mut self, path: PathBuf) {
        match Model::load(&path) {
            Ok(model) => {
                self.model = Some(model);
                self.error = None;
            }
            Err(e) => self.error = Some(format!("{e:#}")),
        }
    }

    fn reload(&mut self) {
        if let Some(path) = self.model.as_ref().map(|m| m.file.clone()) {
            self.open(path);
        }
    }

    fn pick_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            self.open(path);
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let (up, down, toggle_base, clear_base, open, reload, dropped) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::K),
                i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::J),
                i.key_pressed(egui::Key::B) && !i.modifiers.command,
                i.key_pressed(egui::Key::Escape),
                i.modifiers.command && i.key_pressed(egui::Key::O),
                i.modifiers.command && i.key_pressed(egui::Key::R),
                i.raw.dropped_files.first().map(|f| f.path().to_path_buf()),
            )
        });

        if let Some(path) = dropped.filter(|p| !p.as_os_str().is_empty()) {
            self.open(path);
        }
        if open {
            self.pick_file();
        }
        if reload {
            self.reload();
        }
        if let Some(model) = self.model.as_mut() {
            if up {
                model.select_next();
            }
            if down {
                model.select_prev();
            }
            if toggle_base {
                model.toggle_base();
            }
            if clear_base {
                model.clear_base();
            }
        }

        match view::show(ui, self.model.as_mut(), self.error.as_deref()) {
            view::Action::None => {}
            view::Action::Open => self.pick_file(),
            view::Action::Reload => self.reload(),
        }
    }
}

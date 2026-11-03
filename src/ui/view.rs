use eframe::egui::{self, Color32, RichText};

use super::model::Model;
use crate::commands::{NO_HISTORY, format_date, version_name};
use crate::textdiff::{Kind, Line, Row};

pub enum Action {
    None,
    Open,
    Reload,
}

pub fn show(ui: &mut egui::Ui, model: Option<&mut Model>, error: Option<&str>) -> Action {
    let mut action = Action::None;

    egui::Panel::top("toolbar").show(ui, |ui| {
        ui.horizontal(|ui| {
            if ui.button("Open…").clicked() {
                action = Action::Open;
            }
            if model.is_some() && ui.button("Reload").clicked() {
                action = Action::Reload;
            }
            let title = match &model {
                Some(m) => m.file.display().to_string(),
                None => "No file open".to_string(),
            };
            ui.label(RichText::new(title).strong());
        });
        if let Some(err) = error {
            ui.colored_label(Color32::RED, err);
        }
        ui.add_space(4.0);
    });

    let Some(model) = model else {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label("Open a file (⌘O) or drop one here.");
            });
        });
        return action;
    };

    egui::Panel::left("timeline")
        .resizable(true)
        .default_size(300.0)
        .show(ui, |ui| timeline(ui, model));

    egui::CentralPanel::default().show(ui, |ui| {
        if !model.has_history() {
            ui.label(NO_HISTORY);
            return;
        }
        diff_pane(ui, model);
    });

    action
}

fn timeline(ui: &mut egui::Ui, model: &mut Model) {
    ui.heading("Versions");
    ui.label(
        RichText::new("↑/↓ select · B mark base · Esc clear")
            .small()
            .weak(),
    );
    ui.separator();

    let selected = model.selected;
    let base = model.base;
    let mut clicked = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for index in (0..model.versions.len()).rev() {
                let v = &model.versions[index];
                let stats = match (model.stats(index), model.is_binary(index)) {
                    (_, true) => "binary".to_string(),
                    (Some((a, r)), _) => format!("+{a} −{r}"),
                    (None, _) => "—".to_string(),
                };
                let base_mark = if base == Some(index) { "◆ " } else { "  " };
                let label = format!(
                    "{base_mark}{:>7}  {}  {stats}",
                    version_name(v),
                    format_date(v)
                );
                let response = ui.selectable_label(index == selected, label);
                if response.clicked() {
                    clicked = Some(index);
                }
                let saved_by = v.saved_by.as_deref().unwrap_or("unknown");
                ui.label(
                    RichText::new(format!("        saved by {saved_by}"))
                        .small()
                        .weak(),
                );
            }
        });

    if let Some(index) = clicked {
        model.select(index);
    }
}

fn diff_pane(ui: &mut egui::Ui, model: &Model) {
    let Some(pair) = model.pair() else {
        ui.label("The selected version is the base. Select another version to compare.");
        return;
    };

    let describe = |i: usize| {
        format!(
            "{} ({})",
            version_name(&model.versions[i]),
            format_date(&model.versions[i])
        )
    };
    let header = match pair.old {
        Some(old) => format!("{}  →  {}", describe(old), describe(pair.new)),
        None => format!("(start)  →  {}", describe(pair.new)),
    };
    ui.label(RichText::new(header).strong());
    ui.separator();

    if model.is_binary(pair.new) || pair.old.is_some_and(|i| model.is_binary(i)) {
        ui.label("Binary file; diff not shown.");
        return;
    }
    let Some(diff) = model.diff() else {
        return;
    };
    ui.label(format!("+{} −{}", diff.added, diff.removed));
    ui.add_space(4.0);

    let dark = ui.visuals().dark_mode;
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show_rows(ui, row_height, diff.rows.len(), |ui, range| {
            for row in &diff.rows[range] {
                match row {
                    Row::Gap(n) => {
                        ui.label(
                            RichText::new(format!("   ⋯ {n} unchanged lines"))
                                .monospace()
                                .italics()
                                .weak(),
                        );
                    }
                    Row::Line(line) => line_row(ui, line, dark),
                }
            }
        });
}

fn line_row(ui: &mut egui::Ui, line: &Line, dark: bool) {
    let (sign, bg) = match (line.kind, dark) {
        (Kind::Equal, _) => (' ', Color32::TRANSPARENT),
        (Kind::Insert, true) => ('+', Color32::from_rgba_unmultiplied(40, 160, 70, 60)),
        (Kind::Insert, false) => ('+', Color32::from_rgb(205, 245, 210)),
        (Kind::Delete, true) => ('−', Color32::from_rgba_unmultiplied(200, 50, 50, 60)),
        (Kind::Delete, false) => ('−', Color32::from_rgb(255, 215, 215)),
    };
    let gutter = |n: Option<usize>| n.map_or("    ".to_string(), |n| format!("{n:>4}"));
    let text = format!(
        "{} {} {sign} {}",
        gutter(line.old_no),
        gutter(line.new_no),
        line.text
    );
    egui::Frame::NONE.fill(bg).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(RichText::new(text).monospace());
    });
}

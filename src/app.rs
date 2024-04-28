use std::cmp::Reverse;

use include_dir::{include_dir, Dir};

use crate::{inspect::InspectReports, report::Report};

#[derive(Clone)]
pub struct MeteoApp {
    reports: Vec<Report>,
    inspect_view: InspectReports,
}

static REPORTS_DIR: Dir<'static> = include_dir!("assets/reports/raw");

impl MeteoApp {
    pub fn new() -> Self {
        let dir = &REPORTS_DIR;
        let mut reports = Vec::new();
        for entry in dir.entries() {
            if let Some(file) = entry.as_file() {
                let original = file.contents_utf8().unwrap().to_string();
                reports.push(Report::new(original))
            }
        }
        reports.sort_unstable_by_key(|report| Reverse(report.report.metadata.date));
        reports.dedup_by_key(|report| report.report.metadata.date);

        MeteoApp {
            inspect_view: InspectReports::new(&reports),
            reports,
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });
        self.inspect_view.ui(&self.reports, ctx);
    }
}

impl eframe::App for MeteoApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.ui(ctx, frame);
    }
}

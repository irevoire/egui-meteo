use std::cmp::Reverse;

use egui::{Layout, RichText};
use include_dir::{include_dir, Dir};

use crate::{dashboard::Dashboard, inspect::InspectReports, report::Report};

#[derive(Clone)]
pub struct MeteoApp {
    reports: Vec<Report>,

    viewing: View,
    dashboard: Dashboard,
    inspect_view: InspectReports,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum View {
    #[default]
    Dashboard,
    Inspect,
    About,
}

static REPORTS_DIR: Dir<'static> = include_dir!("assets/reports/raw");

impl MeteoApp {
    pub fn new() -> Self {
        let dir = &REPORTS_DIR;
        let mut reports = Vec::new();
        for entry in dir.entries() {
            if let Some(file) = entry.as_file() {
                let original = file.contents_utf8().unwrap().to_string();
                reports.push(Report::original(original))
            }
        }
        reports.sort_unstable_by_key(|report| Reverse(report.report.metadata.date));
        reports.dedup_by_key(|report| report.report.metadata.date);

        MeteoApp {
            inspect_view: InspectReports::new(&reports),
            dashboard: Dashboard::new(&reports),
            viewing: View::default(),
            reports,
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.selectable_value(&mut self.viewing, View::Dashboard, "Vue globale");
                ui.selectable_value(
                    &mut self.viewing,
                    View::Inspect,
                    "Inspecter les rapports individuel",
                );

                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::widgets::global_dark_light_mode_buttons(ui);
                    ui.selectable_value(&mut self.viewing, View::About, "À propos");
                });
            });
        });
        match self.viewing {
            View::Dashboard => self.dashboard.ui(ctx),
            View::Inspect => self.inspect_view.ui(&self.reports, ctx),
            View::About => self.about(ctx),
        }
    }

    fn about(&self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.set_max_width(500.);

                ui.horizontal_wrapped(|ui| {
                    ui.label("Salut, je m'appelle");
                    ui.label(RichText::new("Thomas Campistron").strong());
                    ui.label(", ou juste");
                    ui.label(RichText::new("Tamo").strong());
                    ui.label("sur internet. Je suis développeur pour");
                    ui.hyperlink_to("Meilisearch", "https://meilisearch.com");
                    ui.label("en télétravail. J'habite");
                    ui.hyperlink_to("au Vigan", "https://fr.wikipedia.org/wiki/Le_Vigan_(Gard)");
                    ui.label("et j'ai fais ce site après avoir découvert que le lycée de ma ville collectée les données météorologique depuis 2006.");
                    ui.label("Toutes les données affichée sur mon site viennent en réalité de :");
                    ui.hyperlink("http://meteo.lyc-chamson-levigan.ac-montpellier.fr/meteo/index.php?page=releve");
                    ui.label("Elles sont mises à jour tous les jours à 2h du matin.");
                });

                ui.add_space(20.);
                ui.horizontal_wrapped(|ui| {
                    ui.label("L'intégralité du code qui génère ce site web est disponible");
                    ui.hyperlink_to("ici", "https://github.com/irevoire/egui-meteo");
                    ui.label("où vous pouvez m'y faire des suggestions via les « issues ».");
                });
            });
        });
    }
}

impl eframe::App for MeteoApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.ui(ctx, frame);
    }
}

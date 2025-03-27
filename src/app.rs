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
                    egui::widgets::global_theme_preference_buttons(ui);
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
                    // Trick so we don't have to add spaces in the text below:
                    let width = ui.fonts(|f|f.glyph_width(&egui::TextStyle::Body.resolve(ui.style()), ' '));
                    ui.spacing_mut().item_spacing.x = width;

                    let now = time::OffsetDateTime::now_utc();

                    ui.label("Salut, je m'appelle");
                    ui.label(RichText::new("Thomas Campistron").strong());
                    ui.label(", ou juste");
                    ui.label(RichText::new("Tamo").strong());
                    ui.label("sur internet. J'ai");

                    let birthdate = time::OffsetDateTime::new_utc(time::Date::from_calendar_date(1996, time::Month::November, 21).unwrap(), time::Time::from_hms(0, 0, 0).unwrap());
                    let alive_since = now - birthdate;
                    let years = alive_since.whole_days() / 365;
                    ui.label(years.to_string()).on_hover_ui(|ui| {ui.label(RichText::new("C'est jeune").small());});

                    ui.label("ans et je suis développeur pour");
                    ui.hyperlink_to("Meilisearch", "https://meilisearch.com");
                    ui.label("en télétravail. J'habite");
                    ui.hyperlink_to("au Vigan", "https://fr.wikipedia.org/wiki/Le_Vigan_(Gard)");

                    let now = time::OffsetDateTime::now_utc();
                    let moved = time::OffsetDateTime::new_utc(time::Date::from_calendar_date(2023, time::Month::June, 8).unwrap(), time::Time::from_hms(0, 0, 0).unwrap());
                    let elapsed = now - moved;
                    let years = elapsed.whole_days() / 365;
                    let months = (elapsed.whole_days() % 365) / 30;

                    match years {
                        0 => ui.label(format!("depuis {months} mois")),
                        1 => ui.label(format!("depuis {years} an et {months} mois")),
                        _ => ui.label(format!("depuis {years} ans")),
                    };

                    ui.label("et j'ai fait ce site après avoir découvert que le lycée à côté de chez moi collecte des données météorologiques depuis 2006.");
                    ui.label("Toutes les données affichées sur mon site viennent en réalité de :");
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

impl Default for MeteoApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for MeteoApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.ui(ctx, frame);
    }
}

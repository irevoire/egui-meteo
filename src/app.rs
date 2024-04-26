use std::{io::Read, sync::mpsc::TryRecvError};

use egui::{Label, Pos2, Rect, Window};
use scraper::{Html, Selector};

pub struct MeteoApp {
    report_list: Vec<Report>,
    available_reports: Vec<meteo::Report>,

    // Example stuff:
    label: String,

    value: f32,
}

struct Report {
    name: String,
    url: String,
    selected: bool,
    status: DownloadingStatus,
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error(transparent)]
    NetworkError(#[from] reqwest::Error),
    #[error(transparent)]
    ReportParsingError(#[from] meteo::ParseError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("Internal Error")]
    InternalError,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Default)]
enum DownloadingStatus {
    #[default]
    NotDownloading,
    Downloading(
        std::sync::mpsc::Receiver<Result<(String, meteo::Report), (Option<String>, Error)>>,
    ),
    Downloaded {
        original: String,
        report: meteo::Report,
    },
    Failed {
        original: Option<String>,
        error: Error,
    },
}

impl DownloadingStatus {
    pub fn downloading(&self) -> bool {
        matches!(self, Self::Downloading(_))
    }

    pub fn try_fetch_report(&mut self) {
        match std::mem::take(self) {
            DownloadingStatus::NotDownloading => (),
            DownloadingStatus::Downloading(recv) => match recv.try_recv() {
                Ok(Ok((original, report))) => {
                    *self = Self::Downloaded { original, report };
                }
                Ok(Err((original, error))) => {
                    *self = Self::Failed { original, error };
                }
                Err(TryRecvError::Empty) => {
                    *self = Self::Downloading(recv);
                }
                Err(TryRecvError::Disconnected) => {
                    *self = Self::Failed {
                        original: None,
                        error: Error::InternalError,
                    };
                }
            },
            DownloadingStatus::Downloaded { .. } => (),
            DownloadingStatus::Failed { .. } => (),
        }
    }

    pub fn not_downloading(&self) -> bool {
        matches!(self, Self::NotDownloading)
    }

    pub fn downloaded(&self) -> bool {
        matches!(self, Self::Downloaded { .. })
    }

    pub fn failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }
}

impl Default for MeteoApp {
    fn default() -> Self {
        let base_url = "http://meteo.lyc-chamson-levigan.ac-montpellier.fr/meteo/";
        let main_page = format!("{base_url}?page=releve");

        let mut body = Vec::new();
        let mut main_page = reqwest::blocking::get(main_page).unwrap();
        main_page.read_to_end(&mut body).unwrap();

        let (body, _, _) = encoding_rs::WINDOWS_1252.decode(&body);
        let document = Html::parse_document(&body);
        let selector = Selector::parse("#gauche select option").unwrap();
        let files = document
            .select(&selector)
            .filter_map(|el| el.attr("value").map(|attr| (el.inner_html(), attr))) // skip everything that doesn't contains a value
            .filter(|(_name, url)| !url.is_empty()) // skip the empty values
            .filter(|(_name, url)| !url.contains("NOAA")) // skip the NOAA stuff, it's the last two months
            .map(|(name, url)| Report {
                name,
                url: format!("{base_url}{url}"),
                selected: false,
                status: DownloadingStatus::NotDownloading,
            })
            .collect();

        Self {
            report_list: files,
            available_reports: Vec::new(),
            label: "Hello World!".to_owned(),
            value: 2.7,
        }
    }
}

impl MeteoApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Default::default()
    }

    pub fn ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::right("right_panel").show(ctx, |ui| {
            egui::ScrollArea::both()
                .drag_to_scroll(false)
                .show(ui, |ui| {
                    for report in self.report_list.iter_mut() {
                        ui.horizontal(|ui| {
                            ui.toggle_value(&mut report.selected, &report.name);
                            ui.separator();
                            match report.status {
                                DownloadingStatus::Failed { ref error, .. } => {
                                    ui.label("❌").on_hover_ui(|ui| {
                                        ui.label(error.to_string());
                                    })
                                }
                                DownloadingStatus::NotDownloading => ui.label("🔗"),
                                DownloadingStatus::Downloading(_) => ui.spinner(),
                                DownloadingStatus::Downloaded { .. } => ui.label("✓"),
                            };
                        });
                    }
                });
        });
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

        egui::CentralPanel::default().show(ctx, |ui| {
            for report in self.available_reports.iter() {
                Window::new(&report.metadata.name).show(ctx, |ui| todo!());
            }
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("eframe template");

            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(&mut self.label);
            });
        });
    }

    pub fn update(&mut self) {
        for report in self.report_list.iter_mut() {
            if report.selected && report.status.not_downloading() {
                let (sender, receiver) = std::sync::mpsc::channel();
                report.status = DownloadingStatus::Downloading(receiver);

                let url = report.url.to_string();
                std::thread::spawn(move || {
                    // if the receiver crashes then the whole ui is probably down
                    let _ = sender.send(Self::download_report(url));
                });
            } else if report.status.downloading() {
                report.status.try_fetch_report();
            }
        }
    }

    fn download_report(url: String) -> Result<(String, meteo::Report), (Option<String>, Error)> {
        let mut body = Vec::new();
        reqwest::blocking::get(&url)
            .map_err(|err| (None, err.into()))?
            .read_to_end(&mut body)
            .map_err(|err| (None, err.into()))?;
        let (body, _, _) = encoding_rs::WINDOWS_1252.decode(&body);
        Ok((
            body.to_string(),
            body.parse::<meteo::Report>()
                .map_err(|err| (Some(body.to_string()), err.into()))?,
        ))
    }
}

impl eframe::App for MeteoApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.ui(ctx, frame);
        self.update();
    }
}

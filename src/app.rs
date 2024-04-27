use std::{ops::RangeInclusive, sync::mpsc::TryRecvError};

use egui::{Color32, Ui, Window};
use egui_plot::{AxisHints, GridInput, GridMark, Legend, Line, Plot, PlotPoint};
use scraper::{Html, Selector};

#[derive(Clone)]
pub struct MeteoApp {
    reports: Vec<SingleReport>,
}

struct SingleReport {
    name: String,
    url: String,
    selected: bool,
    status: DownloadingStatus,
    original: Option<String>,
    report: Option<meteo::Report>,
    error: Option<Error>,
    displaying: DisplayingReport,
}

impl Clone for SingleReport {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            url: self.url.clone(),
            selected: self.selected,
            status: match self.status {
                DownloadingStatus::Downloading(_) => DownloadingStatus::NotDownloading,
                DownloadingStatus::NotDownloading => DownloadingStatus::NotDownloading,
                DownloadingStatus::Failed => DownloadingStatus::Failed,
                DownloadingStatus::Downloaded => DownloadingStatus::Downloaded,
            },
            original: self.original.clone(),
            report: self.report.clone(),
            error: None,
            displaying: self.displaying.clone(),
        }
    }
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

impl SingleReport {
    pub fn ui(&mut self, ctx: &egui::Context) {
        if self.selected {
            let mut still_opened = true;
            Window::new(&self.name)
                .open(&mut still_opened)
                .default_width(800.0)
                .default_height(500.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.displaying.mode,
                            DisplayMode::Temperature,
                            "temperatures",
                        );
                        ui.selectable_value(&mut self.displaying.mode, DisplayMode::Rain, "rain");
                        ui.selectable_value(&mut self.displaying.mode, DisplayMode::Raw, "Raw");
                    });
                    ui.separator();

                    match self.displaying.mode {
                        DisplayMode::Temperature => self.temperature(ui),
                        DisplayMode::Rain => self.rain(ui),
                        DisplayMode::Raw => self.raw(ui),
                    }
                });
            self.selected = still_opened;
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    fn x_grid(input: GridInput) -> Vec<GridMark> {
        const MINS_PER_DAY: f64 = 24.0 * 60.0;
        const MINS_PER_H: f64 = 60.0;

        // Note: this always fills all possible marks. For optimization, `input.bounds`
        // could be used to decide when the low-interval grids (minutes) should be added.

        let mut marks = vec![];

        let (min, max) = input.bounds;
        let min = min.floor() as i32;
        let max = max.ceil() as i32;

        for i in min..=max {
            let step_size = if i % MINS_PER_DAY as i32 == 0 {
                // 1 day
                MINS_PER_DAY
            } else if i % MINS_PER_H as i32 == 0 {
                // 1 hour
                MINS_PER_H
            } else if i % 5 == 0 {
                // 5min
                5.0
            } else {
                // skip grids below 5min
                continue;
            };

            marks.push(GridMark {
                value: i as f64,
                step_size,
            });
        }

        marks
    }

    fn create_plot_time(name: &str, formatter: impl Fn(f64) -> String + 'static) -> Plot {
        const MINS_PER_DAY: f64 = 24.0 * 60.0;
        const MINS_PER_H: f64 = 60.0;

        fn day(x: f64) -> f64 {
            (x / MINS_PER_DAY).floor()
        }

        fn hour(x: f64) -> f64 {
            (x.rem_euclid(MINS_PER_DAY) / MINS_PER_H).floor()
        }

        fn minute(x: f64) -> f64 {
            x.rem_euclid(MINS_PER_H).floor()
        }
        fn is_approx_zero(val: f64) -> bool {
            val.abs() < 1e-6
        }

        fn is_approx_integer(val: f64) -> bool {
            val.fract().abs() < 1e-6
        }

        let time_formatter = |mark: GridMark, _digits, _range: &RangeInclusive<f64>| {
            let minutes = mark.value;
            if minutes < 0.0 || 5.0 * MINS_PER_DAY <= minutes {
                // No labels outside value bounds
                String::new()
            } else if is_approx_integer(minutes / MINS_PER_DAY) {
                // Days
                format!("Day {}", day(minutes))
            } else {
                // Hours and minutes
                format!("{h}:{m:02}", h = hour(minutes), m = minute(minutes))
            }
        };

        let label_fmt = move |_s: &str, val: &PlotPoint| {
            format!(
                "Day {d}, {h}:{m:02}\n{p}",
                d = day(val.x),
                h = hour(val.x),
                m = minute(val.x),
                p = formatter(val.y)
            )
        };

        Plot::new(name)
            .legend(Legend::default())
            .custom_x_axes(vec![AxisHints::new_x()
                .label("Date")
                .formatter(time_formatter)])
            .label_formatter(label_fmt)
    }

    pub fn temperature(&mut self, ui: &mut Ui) {
        if let Some(ref report) = self.report {
            let plot = Self::create_plot_time("Temperature", |degree| format!("{degree:.2}°C"));
            plot.show(ui, |ui| {
                // gather all data
                let low_temp: Vec<_> = report
                    .days
                    .iter()
                    .map(|day| [day.date.day() as f64, day.low_temp as f64])
                    .collect();
                let mean_temp: Vec<_> = report
                    .days
                    .iter()
                    .map(|day| [day.date.day() as f64, day.mean_temp as f64])
                    .collect();
                let high_temp: Vec<_> = report
                    .days
                    .iter()
                    .map(|day| [day.date.day() as f64, day.high_temp as f64])
                    .collect();

                // display all data
                ui.line(
                    Line::new(low_temp)
                        .color(Color32::LIGHT_BLUE)
                        .name("temperature minimale"),
                );
                ui.line(
                    Line::new(mean_temp)
                        .color(Color32::GREEN)
                        .name("temperature moyenne"),
                );
                ui.line(
                    Line::new(high_temp)
                        .color(Color32::RED)
                        .name("temperature maximale"),
                );
            });
        }
    }

    pub fn rain(&mut self, ui: &mut Ui) {
        if let Some(ref report) = self.report {
            let plot = Plot::new("Pluie").legend(Legend::default());
            plot.show(ui, |ui| {
                // gather all data
                let rain: Vec<_> = report
                    .days
                    .iter()
                    .map(|day| [day.date.day() as f64, day.rain as f64])
                    .collect();

                // display all data
                ui.line(Line::new(rain).color(Color32::LIGHT_BLUE).name("pluie"));
            });
        }
    }

    pub fn raw(&mut self, ui: &mut Ui) {
        if let Some(ref original) = self.original {
            ui.label(original);
        }
    }
}

#[derive(Default)]
enum DownloadingStatus {
    #[default]
    NotDownloading,
    Downloading(
        std::sync::mpsc::Receiver<Result<(String, meteo::Report), (Option<String>, Error)>>,
    ),
    Failed,
    Downloaded,
}

#[derive(Default, Clone)]
struct DisplayingReport {
    mode: DisplayMode,
}

#[derive(Default, Debug, PartialEq, Clone, Copy)]
enum DisplayMode {
    #[default]
    Temperature,
    Rain,
    Raw,
}

impl DownloadingStatus {
    pub fn downloading(&self) -> bool {
        matches!(self, Self::Downloading(_))
    }

    pub fn try_fetch_report(&mut self) -> (Option<String>, Option<meteo::Report>, Option<Error>) {
        match std::mem::take(self) {
            DownloadingStatus::Downloaded { .. }
            | DownloadingStatus::Failed { .. }
            | DownloadingStatus::NotDownloading => (None, None, None),
            DownloadingStatus::Downloading(recv) => match recv.try_recv() {
                Ok(Ok((original, report))) => {
                    *self = Self::Downloaded;
                    (Some(original), Some(report), None)
                }
                Ok(Err((original, error))) => {
                    *self = Self::Failed;
                    (original, None, Some(error))
                }
                Err(TryRecvError::Empty) => {
                    *self = Self::Downloading(recv);
                    (None, None, None)
                }
                Err(TryRecvError::Disconnected) => {
                    *self = Self::Failed;
                    (None, None, Some(Error::InternalError))
                }
            },
        }
    }

    pub fn not_downloading(&self) -> bool {
        matches!(self, Self::NotDownloading)
    }

    #[allow(unused)]
    pub fn downloaded(&self) -> bool {
        matches!(self, Self::Downloaded { .. })
    }

    #[allow(unused)]
    pub fn failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }
}

impl MeteoApp {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Self {
        let base_url = "http://meteo.lyc-chamson-levigan.ac-montpellier.fr/meteo/";
        let main_page = format!("{base_url}?page=releve");

        log::info!("Downloading the main page");
        let body = get(&main_page).unwrap();

        let (body, _, _) = encoding_rs::WINDOWS_1252.decode(&body);
        let document = Html::parse_document(&body);
        let selector = Selector::parse("#gauche select option").unwrap();
        let files = document
            .select(&selector)
            .filter_map(|el| el.attr("value").map(|attr| (el.inner_html(), attr))) // skip everything that doesn't contains a value
            .filter(|(_name, url)| !url.is_empty()) // skip the empty values
            .filter(|(_name, url)| !url.contains("NOAA")) // skip the NOAA stuff, it's the last two months
            .map(|(name, url)| SingleReport {
                name,
                url: format!("{base_url}{url}"),
                selected: false,
                status: DownloadingStatus::NotDownloading,
                displaying: DisplayingReport::default(),
                original: None,
                report: None,
                error: None,
            })
            .collect();

        MeteoApp { reports: files }
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new() -> Self {
        let base_url = "http://meteo.lyc-chamson-levigan.ac-montpellier.fr/meteo/";
        let main_page = format!("{base_url}?page=releve");

        log::info!("Downloading the main page");
        let body = get(&main_page).await.unwrap();

        let (body, _, _) = encoding_rs::WINDOWS_1252.decode(&body);
        let document = Html::parse_document(&body);
        let selector = Selector::parse("#gauche select option").unwrap();
        let files = document
            .select(&selector)
            .filter_map(|el| el.attr("value").map(|attr| (el.inner_html(), attr))) // skip everything that doesn't contains a value
            .filter(|(_name, url)| !url.is_empty()) // skip the empty values
            .filter(|(_name, url)| !url.contains("NOAA")) // skip the NOAA stuff, it's the last two months
            .map(|(name, url)| SingleReport {
                name,
                url: format!("{base_url}{url}"),
                selected: false,
                status: DownloadingStatus::NotDownloading,
                displaying: DisplayingReport::default(),
                original: None,
                report: None,
                error: None,
            })
            .collect();

        MeteoApp { reports: files }
    }

    pub fn ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::right("right_panel").show(ctx, |ui| {
            egui::ScrollArea::both()
                .drag_to_scroll(false)
                .show(ui, |ui| {
                    for report in self.reports.iter_mut() {
                        ui.horizontal(|ui| {
                            ui.toggle_value(&mut report.selected, &report.name);
                            ui.separator();
                            match report.status {
                                DownloadingStatus::Failed => ui.label("❌").on_hover_ui(|ui| {
                                    if let Some(ref error) = report.error {
                                        ui.label(error.to_string());
                                    }
                                }),
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

        egui::CentralPanel::default().show(ctx, |_ui| {
            for report in self.reports.iter_mut() {
                report.ui(ctx);
            }
        });
    }

    pub fn update(&mut self) {
        for report in self.reports.iter_mut() {
            if report.selected && report.status.not_downloading() {
                let (sender, receiver) = std::sync::mpsc::channel();
                report.status = DownloadingStatus::Downloading(receiver);

                let url = report.url.to_string();
                #[cfg(not(target_arch = "wasm32"))]
                std::thread::spawn(move || {
                    // if the receiver crashes then the whole ui is probably down
                    let _ = sender.send(Self::download_report(url));
                });
                #[cfg(target_arch = "wasm32")]
                wasm_bindgen_futures::spawn_local(async move {
                    // if the receiver crashes then the whole ui is probably down
                    let _ = sender.send(Self::download_report(url).await);
                });
            } else if report.status.downloading() {
                let (original, meteo_report, error) = report.status.try_fetch_report();

                if let Some(original) = original {
                    report.original = Some(original);
                }
                if let Some(meteo_report) = meteo_report {
                    report.report = Some(meteo_report);
                    report.error = None;
                }
                if let Some(error) = error {
                    report.error = Some(error);
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn download_report(url: String) -> Result<(String, meteo::Report), (Option<String>, Error)> {
        let body = get(&url).map_err(|err| (None, err.into()))?;
        let (body, _, _) = encoding_rs::WINDOWS_1252.decode(&body);
        Ok((
            body.to_string(),
            body.parse::<meteo::Report>()
                .map_err(|err| (Some(body.to_string()), err.into()))?,
        ))
    }

    #[cfg(target_arch = "wasm32")]
    async fn download_report(
        url: String,
    ) -> Result<(String, meteo::Report), (Option<String>, Error)> {
        let body = get(&url).await.map_err(|err| (None, err.into()))?;
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

#[cfg(not(target_arch = "wasm32"))]
fn get(url: &str) -> Result<Vec<u8>, Error> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .unwrap();
    runtime.block_on(async {
        let ret = reqwest::get(url).await?;
        Ok(ret.bytes().await?.to_vec())
    })
}

#[cfg(target_arch = "wasm32")]
async fn get(url: &str) -> Result<Vec<u8>, Error> {
    let ret = reqwest::get(url).await?;
    Ok(ret.bytes().await?.to_vec())
}

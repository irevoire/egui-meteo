use std::{ops::RangeInclusive, sync::mpsc::TryRecvError};

use egui::{Color32, RichText, Ui, Window};
use egui_plot::{AxisHints, GridInput, GridMark, Legend, Line, Plot, PlotPoint};
use scraper::{Html, Selector};
use time::{macros::format_description, Date, Duration, Month, OffsetDateTime, Time};

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
                .default_width(800.0)
                .default_height(500.0)
                .open(&mut still_opened)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.displaying.mode,
                            DisplayMode::Temperature,
                            "Températures",
                        );
                        ui.selectable_value(&mut self.displaying.mode, DisplayMode::Rain, "Pluie");
                        ui.selectable_value(&mut self.displaying.mode, DisplayMode::Wind, "Vent");
                        ui.selectable_value(&mut self.displaying.mode, DisplayMode::Text, "Texte");
                    });
                    ui.separator();

                    match self.displaying.mode {
                        DisplayMode::Temperature => self.temperature(ui),
                        DisplayMode::Rain => self.rain(ui),
                        DisplayMode::Wind => self.wind(ui),
                        DisplayMode::Text => self.text(ui),
                    }
                });
            self.selected = still_opened;
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    fn x_grid(input: GridInput) -> Vec<GridMark> {
        let null_time = OffsetDateTime::from_unix_timestamp(0).unwrap();
        let max_time = OffsetDateTime::from_unix_timestamp(253402300799).unwrap();

        let (start, end) = input.bounds;
        let (start, end) = (
            Self::date_from_chart(start).unwrap_or(null_time),
            Self::date_from_chart(end).unwrap_or(max_time),
        );

        let duration = end - start;

        let mut marks = vec![];

        let decade_step_size = Self::date_to_chart(null_time + Duration::days(365 * 10));
        for year in [1990, 2000, 2010, 2020, 2030] {
            let date = OffsetDateTime::new_utc(
                Date::from_ordinal_date(year, 1).unwrap(),
                Time::from_hms(0, 0, 0).unwrap(),
            );
            if (start..end).contains(&date) {
                marks.push(GridMark {
                    value: Self::date_to_chart(date),
                    step_size: decade_step_size,
                });
            }
        }
        let year_step_size = Self::date_to_chart(null_time + Duration::days(365));
        let month_step_size = Self::date_to_chart(null_time + Duration::DAY * 30);
        let day_step_size = Self::date_to_chart(null_time + Duration::DAY);
        let hour_step_size = Self::date_to_chart(null_time + Duration::HOUR);
        let minute_step_size = Self::date_to_chart(null_time + Duration::MINUTE);

        for year in start.year()..=end.year() {
            // First add the mark
            let date = OffsetDateTime::new_utc(
                Date::from_ordinal_date(year, 1).unwrap(),
                Time::from_hms(0, 0, 0).unwrap(),
            );
            if (start..end).contains(&date) {
                marks.push(GridMark {
                    value: Self::date_to_chart(date),
                    step_size: year_step_size,
                });
            }
            // Early exit if there is too many months to display
            if duration.whole_days() > 720 {
                continue;
            }
            // Second, prepare the range for the month
            let s = if year == start.year() {
                start.month() as u8
            } else {
                Month::January as u8
            };
            let e = if year == end.year() {
                end.month() as u8
            } else {
                Month::December as u8
            };
            for month in s..=e {
                let month = Month::try_from(month).unwrap();
                let date = date.replace_month(month).unwrap();
                if (start..end).contains(&date) {
                    marks.push(GridMark {
                        value: Self::date_to_chart(date),
                        step_size: month_step_size,
                    });
                }
                if duration.whole_days() > 120 {
                    continue;
                }
                let s = if year == start.year() && month == start.month() {
                    start.day()
                } else {
                    1
                };
                let e = if year == end.year() && month == end.month() {
                    end.day()
                } else {
                    31
                };
                for day in s..=e {
                    let date = match date.replace_day(day) {
                        Ok(date) => date,
                        Err(_) => continue,
                    };
                    if (start..end).contains(&date) {
                        marks.push(GridMark {
                            value: Self::date_to_chart(date),
                            step_size: day_step_size,
                        });
                    }
                    if duration.whole_hours() > 48 {
                        continue;
                    }
                    let s = if year == start.year() && month == start.month() && day == start.day()
                    {
                        start.hour()
                    } else {
                        0
                    };
                    let e = if year == end.year() && month == end.month() && day == end.day() {
                        end.hour()
                    } else {
                        23
                    };

                    for hour in s..=e {
                        let date = date.replace_hour(hour).unwrap();
                        if (start..end).contains(&date) {
                            marks.push(GridMark {
                                value: Self::date_to_chart(date),
                                step_size: hour_step_size,
                            });
                        }
                        if duration.whole_minutes() > 120 {
                            continue;
                        }
                        let s = if year == start.year()
                            && month == start.month()
                            && day == start.day()
                            && hour == start.hour()
                        {
                            start.hour()
                        } else {
                            0
                        };
                        let e = if year == end.year()
                            && month == end.month()
                            && day == end.day()
                            && hour == end.hour()
                        {
                            end.hour()
                        } else {
                            59
                        };
                        for minute in s..=e {
                            let date = date.replace_minute(minute).unwrap();
                            if (start..end).contains(&date) {
                                marks.push(GridMark {
                                    value: Self::date_to_chart(date),
                                    step_size: minute_step_size,
                                });
                            }
                        }
                    }
                }
            }
        }

        marks
    }

    fn create_plot_time(name: &str, formatter: impl Fn(f64) -> String + 'static) -> Plot {
        let time_formatter = |mark: GridMark, _digits, _range: &RangeInclusive<f64>| {
            let step = Self::date_from_chart(mark.step_size).unwrap();
            let step = step - OffsetDateTime::from_unix_timestamp(0).unwrap();
            let days = step.whole_days();
            let format = if days > 365 * 3 {
                format_description!("[year]")
            } else if days > 30 * 3 {
                format_description!("[month]/[year]")
            } else if days > 3 {
                format_description!("[day]/[month]/[year]")
            } else {
                format_description!("[day]/[month]/[year] - [hour]:[minute]")
            };
            Self::date_from_chart(mark.value)
                .unwrap()
                .format(format)
                .unwrap()
        };

        let label_fmt = move |_s: &str, val: &PlotPoint| {
            let date = Self::date_from_chart(val.x)
                .map(|date| {
                    date.format(format_description!(
                        "[day]/[month]/[year] - [hour]:[minute]"
                    ))
                    .unwrap()
                })
                .unwrap_or(String::from(""));
            format!("{}\n{}", date, formatter(val.y))
        };

        Plot::new(name)
            .legend(Legend::default())
            .custom_x_axes(vec![AxisHints::new_x()
                .label("Date")
                .formatter(time_formatter)])
            .x_grid_spacer(Self::x_grid)
            .label_formatter(label_fmt)
    }

    // since we MUST store an f64 but all we have is a i64 and we absolutely don't want to lose any precision
    // we're going to store it as-is in a f64 via a transmute instead of doing a cast
    fn date_to_chart(date: OffsetDateTime) -> f64 {
        // unsafe { std::mem::transmute(date.unix_timestamp()) }
        date.unix_timestamp() as f64
    }

    fn date_from_chart(axis: f64) -> Option<OffsetDateTime> {
        // let unix_timestamp: i64 = unsafe { std::mem::transmute(axis) };
        let unix_timestamp: i64 = axis as i64;
        OffsetDateTime::from_unix_timestamp(unix_timestamp).ok()
    }

    pub fn temperature(&mut self, ui: &mut Ui) {
        if let Some(ref report) = self.report {
            let plot = Self::create_plot_time("Temperature", |degree| format!("{degree:.2}°C"))
                .custom_y_axes(vec![AxisHints::new_y().label("Temperature en °C")]);
            plot.show(ui, |ui| {
                // gather all data
                let low_temp: Vec<_> = report
                    .days
                    .iter()
                    .map(|day| {
                        [
                            Self::date_to_chart(day.low_temp_date.assume_utc()),
                            day.low_temp as f64,
                        ]
                    })
                    .collect();
                let mean_temp: Vec<_> = report
                    .days
                    .iter()
                    .map(|day| {
                        [
                            Self::date_to_chart(day.date.with_hms(12, 0, 0).unwrap().assume_utc()),
                            day.mean_temp as f64,
                        ]
                    })
                    .collect();
                let high_temp: Vec<_> = report
                    .days
                    .iter()
                    .map(|day| {
                        [
                            Self::date_to_chart(day.high_temp_date.assume_utc()),
                            day.high_temp as f64,
                        ]
                    })
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
            let plot = Self::create_plot_time("Pluie", |rain| format!("{rain:.2}mm"))
                .custom_y_axes(vec![AxisHints::new_y().label("Pluie en mm/m²")]);
            plot.show(ui, |ui| {
                // gather all data
                let rain: Vec<_> = report
                    .days
                    .iter()
                    .map(|day| {
                        [
                            Self::date_to_chart(day.date.with_hms(12, 0, 0).unwrap().assume_utc()),
                            day.rain as f64,
                        ]
                    })
                    .collect();

                // display all data
                ui.line(Line::new(rain).color(Color32::LIGHT_BLUE).name("pluie"));
            });
        }
    }

    pub fn wind(&mut self, ui: &mut Ui) {
        if let Some(ref report) = self.report {
            let plot = Self::create_plot_time("Vent", |wind| format!("{wind:.2}km/h"))
                .custom_y_axes(vec![AxisHints::new_y().label("Vent en km/h")]);
            plot.show(ui, |ui| {
                let mean_wind: Vec<_> = report
                    .days
                    .iter()
                    .map(|day| {
                        [
                            Self::date_to_chart(day.date.with_hms(12, 0, 0).unwrap().assume_utc()),
                            day.avg_wind_speed as f64,
                        ]
                    })
                    .collect();
                let high_wind: Vec<_> = report
                    .days
                    .iter()
                    .map(|day| {
                        [
                            Self::date_to_chart(
                                day.high_wind_speed_date
                                    .unwrap_or_else(|| day.date.with_hms(12, 0, 0).unwrap())
                                    .assume_utc(),
                            ),
                            day.high_wind_speed as f64,
                        ]
                    })
                    .collect();

                // display all data
                ui.line(
                    Line::new(mean_wind)
                        .color(Color32::GREEN)
                        .name("vent moyen"),
                );
                ui.line(
                    Line::new(high_wind)
                        .color(Color32::RED)
                        .name("vent maximal"),
                );
            });
        }
    }

    pub fn text(&mut self, ui: &mut Ui) {
        if let Some(ref original) = self.original {
            ui.label(RichText::new(original).monospace());
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
    Wind,
    Text,
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

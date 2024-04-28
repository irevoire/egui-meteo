use egui::{Color32, Context, RichText, Ui, Window};
use egui_plot::{AxisHints, Line};

use crate::{date_to_chart, plot::create_plot_time, report::Report};

#[derive(Clone)]
pub struct InspectReports {
    parameters: Vec<Parameter>,
}

impl InspectReports {
    pub fn new(reports: &[Report]) -> Self {
        let mut parameters = vec![Parameter::default(); reports.len()];
        if let Some(parameter) = parameters.last_mut() {
            parameter.selected = true;
        }
        Self { parameters }
    }

    pub fn ui(&mut self, reports: &[Report], ctx: &Context) {
        egui::SidePanel::right("right_panel").show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (parameter, report) in self.parameters.iter_mut().zip(reports) {
                    ui.horizontal(|ui| {
                        ui.toggle_value(&mut parameter.selected, report.name());
                    });
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |_ui| {
            for (parameter, report) in self.parameters.iter_mut().zip(reports) {
                parameter.ui(report, ctx);
            }
        });
    }
}

#[derive(Default, Clone)]
struct Parameter {
    selected: bool,
    displaying: DisplayingReport,
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

impl Parameter {
    pub fn ui(&mut self, report: &Report, ctx: &egui::Context) {
        if self.selected {
            let mut still_opened = true;
            Window::new(report.name())
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
                        DisplayMode::Temperature => self.temperature(report, ui),
                        DisplayMode::Rain => self.rain(report, ui),
                        DisplayMode::Wind => self.wind(report, ui),
                        DisplayMode::Text => self.text(report, ui),
                    }
                });
            self.selected = still_opened;
        }
    }

    pub fn temperature(&mut self, report: &Report, ui: &mut Ui) {
        let report = &report.report;
        let plot = create_plot_time("Temperature", |degree| format!("{degree:.2}°C"))
            .custom_y_axes(vec![AxisHints::new_y().label("Temperature en °C")]);
        plot.show(ui, |ui| {
            // gather all data
            let low_temp: Vec<_> = report
                .days
                .iter()
                .map(|day| {
                    [
                        date_to_chart(day.low_temp_date.assume_utc()),
                        day.low_temp as f64,
                    ]
                })
                .collect();
            let mean_temp: Vec<_> = report
                .days
                .iter()
                .map(|day| {
                    [
                        date_to_chart(day.date.with_hms(12, 0, 0).unwrap().assume_utc()),
                        day.mean_temp as f64,
                    ]
                })
                .collect();
            let high_temp: Vec<_> = report
                .days
                .iter()
                .map(|day| {
                    [
                        date_to_chart(day.high_temp_date.assume_utc()),
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

    pub fn rain(&mut self, report: &Report, ui: &mut Ui) {
        let report = &report.report;
        let plot = create_plot_time("Pluie", |rain| format!("{rain:.2}mm"))
            .custom_y_axes(vec![AxisHints::new_y().label("Pluie en mm/m²")]);
        plot.show(ui, |ui| {
            // gather all data
            let rain: Vec<_> = report
                .days
                .iter()
                .map(|day| {
                    [
                        date_to_chart(day.date.with_hms(12, 0, 0).unwrap().assume_utc()),
                        day.rain as f64,
                    ]
                })
                .collect();

            // display all data
            ui.line(Line::new(rain).color(Color32::LIGHT_BLUE).name("pluie"));
        });
    }

    pub fn wind(&mut self, report: &Report, ui: &mut Ui) {
        let report = &report.report;
        let plot = create_plot_time("Vent", |wind| format!("{wind:.2}km/h"))
            .custom_y_axes(vec![AxisHints::new_y().label("Vent en km/h")]);
        plot.show(ui, |ui| {
            let mean_wind: Vec<_> = report
                .days
                .iter()
                .map(|day| {
                    [
                        date_to_chart(day.date.with_hms(12, 0, 0).unwrap().assume_utc()),
                        day.avg_wind_speed as f64,
                    ]
                })
                .collect();
            let high_wind: Vec<_> = report
                .days
                .iter()
                .map(|day| {
                    [
                        date_to_chart(
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

    pub fn text(&mut self, report: &Report, ui: &mut Ui) {
        ui.label(RichText::new(&report.original).monospace());
    }
}

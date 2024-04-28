use egui::{Color32, RichText, Ui};
use egui_plot::{AxisHints, Line};
use time::Month;

use crate::{date_to_chart, plot::create_plot_time};

pub struct Report {
    pub original: Option<String>,
    pub report: meteo::Report,
}

impl Clone for Report {
    fn clone(&self) -> Self {
        Self {
            original: self.original.clone(),
            report: self.report.clone(),
        }
    }
}

impl Report {
    pub fn original(original: String) -> Self {
        let report = original.parse().unwrap();
        Self {
            original: Some(original),
            report,
        }
    }

    pub fn merge(&self, other: &Self) -> Self {
        let mut report = self.report.clone();
        report.merge(other.report.clone()).unwrap();

        Self {
            original: None,
            report,
        }
    }

    pub fn name(&self) -> String {
        let date = self.report.metadata.date;
        let month = match date.month() {
            Month::January => "Janvier",
            Month::February => "Février",
            Month::March => "Mars",
            Month::April => "Avril",
            Month::May => "Mai",
            Month::June => "Juin",
            Month::July => "Juillet",
            Month::August => "Aout",
            Month::September => "Septembre",
            Month::October => "Octobre",
            Month::November => "Novembre",
            Month::December => "Décembre",
        };
        format!("{} - {month}", date.year())
    }
}

#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub enum DisplayReport {
    #[default]
    Temperature,
    Rain,
    Wind,
    Text,
}

impl DisplayReport {
    pub fn ui(&mut self, report: &Report, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(self, Self::Temperature, "Températures");
            ui.selectable_value(self, Self::Rain, "Pluie");
            ui.selectable_value(self, Self::Wind, "Vent");
            if report.original.is_some() {
                ui.selectable_value(self, Self::Text, "Texte");
            }
        });
        ui.separator();

        match self {
            DisplayReport::Temperature => self.temperature(report, ui),
            DisplayReport::Rain => self.rain(report, ui),
            DisplayReport::Wind => self.wind(report, ui),
            DisplayReport::Text => self.text(report, ui),
        }
    }

    pub fn temperature(&mut self, report: &Report, ui: &mut Ui) {
        let report = &report.report;
        let plot = create_plot_time("Temperature", |degree| format!("{degree:.2}°C"))
            .link_axis(ui.id(), true, false)
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
            .link_axis(ui.id(), true, false)
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
            .link_axis(ui.id(), true, false)
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
        if let Some(ref original) = report.original {
            ui.label(RichText::new(original).monospace());
        } else {
            ui.label("The report was generated and there is no original");
        }
    }
}

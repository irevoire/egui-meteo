use egui::{Context, Window};

use crate::report::{DisplayReport, Report};

#[derive(Clone)]
pub struct InspectReports {
    parameters: Vec<Parameter>,
    filter: String,
}

impl InspectReports {
    pub fn new(reports: &[Report]) -> Self {
        let mut parameters = vec![Parameter::default(); reports.len()];
        if let Some(parameter) = parameters.first_mut() {
            parameter.selected = true;
        }
        Self {
            parameters,
            filter: String::new(),
        }
    }

    pub fn ui(&mut self, reports: &[Report], ctx: &Context) {
        egui::SidePanel::right("right_panel").show(ctx, |ui| {
            // get the list of stuff we need to dislay:
            let to_display: Vec<_> = self
                .parameters
                .iter_mut()
                .zip(reports)
                .filter(|(_param, report)| {
                    self.filter.is_empty()
                        || report
                            .name()
                            .to_lowercase()
                            .contains(&self.filter.to_lowercase())
                })
                .collect();

            ui.vertical(|ui| {
                ui.text_edit_singleline(&mut self.filter);
                ui.label(format!("Total: {}", reports.len()));
                if !self.filter.is_empty() {
                    ui.label(format!("Après filtre: {}", to_display.len()));
                }

                ui.separator();
            });
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (parameter, report) in to_display {
                    if self.filter.is_empty()
                        || report
                            .name()
                            .to_lowercase()
                            .contains(&self.filter.to_lowercase())
                    {
                        ui.horizontal(|ui| {
                            ui.toggle_value(&mut parameter.selected, report.name());
                        });
                    }
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
pub struct Parameter {
    selected: bool,
    displaying: DisplayReport,
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
                    self.displaying.ui(report, ui);
                });
            self.selected = still_opened;
        }
    }
}

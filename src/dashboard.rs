use egui::Context;

use crate::report::{DisplayReport, Report};

#[derive(Clone)]
pub struct Dashboard {
    maxi_report: Report,
    displaying: DisplayReport,
}

impl Dashboard {
    pub fn new(reports: &[Report]) -> Self {
        let mut reports = reports.iter();
        let first_report = reports.next().unwrap().clone();
        let maxi_report = reports.fold(first_report, |left, right| left.merge(&right));

        Self {
            maxi_report,
            displaying: DisplayReport::default(),
        }
    }

    pub fn ui(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| self.displaying.ui(&self.maxi_report, ui));
    }
}

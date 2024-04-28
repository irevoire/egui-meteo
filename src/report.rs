use time::Month;

pub struct Report {
    pub original: String,
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
    pub fn new(original: String) -> Self {
        let report = original.parse().unwrap();
        Self { original, report }
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

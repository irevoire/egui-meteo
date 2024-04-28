#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod dashboard;
mod inspect;
mod plot;
mod report;
pub use app::MeteoApp;
use time::OffsetDateTime;

fn date_to_chart(date: OffsetDateTime) -> f64 {
    date.unix_timestamp() as f64
}

fn date_from_chart(axis: f64) -> Option<OffsetDateTime> {
    let unix_timestamp: i64 = axis as i64;
    OffsetDateTime::from_unix_timestamp(unix_timestamp).ok()
}

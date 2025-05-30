use std::{ops::RangeInclusive, sync::Arc};

use egui_plot::{AxisHints, CoordinatesFormatter, GridInput, GridMark, Legend, Plot, PlotPoint};
use meteo::Report;
use time::{macros::format_description, Date, Duration, Month, OffsetDateTime, Time};

use crate::{date_from_chart, date_to_chart};

#[allow(clippy::collapsible_if)]
fn x_grid(input: GridInput) -> Vec<GridMark> {
    let min_time = OffsetDateTime::from_unix_timestamp(-377705116800).unwrap();
    let null_time = OffsetDateTime::from_unix_timestamp(0).unwrap();
    let max_time = OffsetDateTime::from_unix_timestamp(253402300799).unwrap();

    let (start, end) = input.bounds;
    let (start, end) = (
        date_from_chart(start).unwrap_or(min_time),
        date_from_chart(end).unwrap_or(max_time),
    );

    let duration = end - start;

    let mut marks = vec![];

    let year_step_size = date_to_chart(null_time + Duration::days(365));
    let month_step_size = date_to_chart(null_time + Duration::DAY * 30);
    let day_step_size = date_to_chart(null_time + Duration::DAY);
    let hour_step_size = date_to_chart(null_time + Duration::HOUR);
    let minute_step_size = date_to_chart(null_time + Duration::MINUTE);

    for year in start.year()..=end.year() {
        // First add the mark
        let date = OffsetDateTime::new_utc(
            Date::from_ordinal_date(year, 1).unwrap(),
            Time::from_hms(0, 0, 0).unwrap(),
        );
        // Early exit if there is too many years to display
        if duration.whole_days() > 365 * 20 {
            if (start..end).contains(&date) && year % 10 == 0 {
                marks.push(GridMark {
                    value: date_to_chart(date),
                    step_size: year_step_size * 10.0,
                });
            }
            continue;
        }
        // Early exit if there is too many months to display
        if duration.whole_days() > 365 * 3 {
            if (start..end).contains(&date) {
                marks.push(GridMark {
                    value: date_to_chart(date),
                    step_size: year_step_size,
                });
            }
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
            if duration.whole_days() > 30 * 3 {
                if (start..end).contains(&date) {
                    marks.push(GridMark {
                        value: date_to_chart(date),
                        step_size: month_step_size,
                    });
                }
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
                if duration.whole_days() > 90 {
                    if (start..end).contains(&date) && day % 24 == 0 {
                        marks.push(GridMark {
                            value: date_to_chart(date),
                            step_size: day_step_size * 24.0,
                        });
                    }
                }
                if duration.whole_days() > 60 {
                    if (start..end).contains(&date) && day % 12 == 0 {
                        marks.push(GridMark {
                            value: date_to_chart(date),
                            step_size: day_step_size * 12.0,
                        });
                    }
                }
                if duration.whole_days() > 30 {
                    if (start..end).contains(&date) && day % 6 == 0 {
                        marks.push(GridMark {
                            value: date_to_chart(date),
                            step_size: day_step_size * 6.0,
                        });
                    }
                }
                if duration.whole_days() > 15 {
                    if (start..end).contains(&date) && day % 3 == 0 {
                        marks.push(GridMark {
                            value: date_to_chart(date),
                            step_size: day_step_size * 3.0,
                        });
                    }
                }
                if duration.whole_days() > 2 {
                    if (start..end).contains(&date) {
                        marks.push(GridMark {
                            value: date_to_chart(date),
                            step_size: day_step_size,
                        });
                    }
                    continue;
                }
                let s = if year == start.year() && month == start.month() && day == start.day() {
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
                    if duration.whole_hours() > 25 {
                        if (start..end).contains(&date) && hour % 6 == 0 {
                            marks.push(GridMark {
                                value: date_to_chart(date),
                                step_size: hour_step_size * 6.0,
                            });
                        }
                    }
                    if duration.whole_hours() > 15 {
                        if (start..end).contains(&date) && hour % 3 == 0 {
                            marks.push(GridMark {
                                value: date_to_chart(date),
                                step_size: hour_step_size * 3.0,
                            });
                        }
                    }
                    if duration.whole_hours() > 2 {
                        if (start..end).contains(&date) {
                            marks.push(GridMark {
                                value: date_to_chart(date),
                                step_size: hour_step_size,
                            });
                        }
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
                                value: date_to_chart(date),
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

pub fn create_plot_time<'a>(
    name: &'a str,
    report: &Report,
    formatter: impl Fn(f64) -> String + 'static,
) -> Plot<'a> {
    let time_formatter = |mark: GridMark, _range: &RangeInclusive<f64>| {
        let step = date_from_chart(mark.step_size).unwrap();
        let step = step - OffsetDateTime::from_unix_timestamp(0).unwrap();
        let days = step.whole_days();
        let format = if days > 364 {
            format_description!("[year]")
        } else if days > 29 {
            format_description!("[year]/[month]")
        } else if days > 0 {
            format_description!("[year]/[month]/[day]")
        } else {
            format_description!("[year]/[month]/[day] - [hour]:[minute]")
        };
        date_from_chart(mark.value).unwrap().format(format).unwrap()
    };

    let format_plot_point = Arc::new(move |point: &PlotPoint| {
        let date = date_from_chart(point.x)
            .map(|date| {
                date.format(format_description!(
                    "[year]/[month]/[day] - [hour]:[minute]"
                ))
                .unwrap()
            })
            .unwrap_or(String::from(""));
        format!("{}\n{}", date, formatter(point.y))
    });

    let fmt = format_plot_point.clone();

    let mut start = report
        .first_date()
        .with_time(Time::from_hms(0, 0, 0).unwrap())
        .assume_utc();
    let end = report
        .last_date()
        .with_time(Time::from_hms(23, 59, 59).unwrap())
        .assume_utc();
    if (end - start).whole_days() > 60 {
        start = end - Duration::DAY * 60;
    }
    Plot::new(name)
        .legend(Legend::default())
        .default_x_bounds(date_to_chart(start), date_to_chart(end))
        .coordinates_formatter(
            egui_plot::Corner::LeftBottom,
            CoordinatesFormatter::new(move |point, _| fmt(point)),
        )
        .custom_x_axes(vec![AxisHints::new_x()
            .label("Date")
            .formatter(time_formatter)])
        .x_grid_spacer(x_grid)
        .label_formatter(move |_, point| format_plot_point(point))
}

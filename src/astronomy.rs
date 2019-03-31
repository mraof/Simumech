use astro::time;
use astro::lunar::{Phase, self};
use chrono::{Utc, Timelike, Datelike};

pub fn decimal_day(day: &time::DayOfMonth) -> f64 {

    (day.day as f64)
        + (day.hr as f64) / 24.0
        + (day.min as f64) / 24.0 / 60.0
        + (day.sec as f64) / 24.0 / 60.0 / 60.0
        - day.time_zone / 24.0
}

/*
pub fn time_from_decimal_day(day: f64) -> (u8, u8, f64) {
    let hour = day.fract() * 24.0;
    let min = hour.fract() * 60.0;
    let sec = min.fract() * 60.0;
    (hour as u8, min as u8, sec)
}
*/

pub fn time_from_moon_phase(phase: Phase) -> f64 {
    let now = Utc::now();
    let day = time::DayOfMonth {
        day: now.day() as u8,
        hr: now.hour() as u8,
        min: now.minute() as u8,
        sec: now.second() as f64 + (now.nanosecond() as f64 / 10e9),
        time_zone: 0.0,
    };
    let date = time::Date {
        year: now.year() as i16,
        month: now.month() as u8,
        decimal_day: decimal_day(&day),
        cal_type: time::CalType::Gregorian,
    };
    let julian_day = time::julian_day(&date);
    let delta_t = time::delta_t(now.year(), date.month);
    let julian_day = time::julian_ephemeris_day(julian_day, delta_t);
    let phase = lunar::time_of_phase(&date, &phase);
    (phase - julian_day).abs()
}

#[test]
pub fn test_time_from_phases() {
    println!("New: {}", time_from_moon_phase(Phase::New));
    println!("First: {}", time_from_moon_phase(Phase::First));
    println!("Full: {}", time_from_moon_phase(Phase::Full));
    println!("Last: {}", time_from_moon_phase(Phase::Last));
}
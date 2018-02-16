use chrono;
use types::{DesiredMeeting, Meeting};
use chrono_tz::Tz;
use chrono::prelude::*;
use fixtures::test_desired_meeting;

// Generate intervals for a desired date, respecting mint, maxt, step and duration
fn generate_meetings_for_date(
    date: chrono::Date<Tz>,
    mint: chrono::NaiveTime,
    maxt: chrono::NaiveTime,
    step: chrono::Duration,
    duration: chrono::Duration,
    id: &str,
    reject_datetime_fn: &Fn(chrono::DateTime<Tz>, chrono::DateTime<Tz>) -> bool,
) -> Vec<Meeting> {
    let mut counter = 0;
    let mut res: Vec<Meeting> = Vec::new();
    let mut t = date.and_time(mint).expect("Cannot parse min date");
    loop {
        let ident = format!("{}_{}", id, counter);
        let start = t;
        let end = t + duration;
        if end.time() > maxt {
            break;
        }
        t = t + step;
        counter += 1;
        if reject_datetime_fn(start, end) {
            continue;
        }
        res.push(Meeting {
            id: ident,
            start: start.with_timezone(&chrono::Utc),
            end: end.with_timezone(&chrono::Utc),
        })
    }
    res
}

pub fn default_reject_date(date: chrono::Date<Tz>) -> bool {
    let wkday = date.weekday();
    (wkday == chrono::Weekday::Wed || wkday == chrono::Weekday::Sat
        || wkday == chrono::Weekday::Sun)
}

pub fn default_reject_datetime(start: chrono::DateTime<Tz>, end: chrono::DateTime<Tz>) -> bool {
    let date = start.date();
    let lunch_start = date.and_time(chrono::NaiveTime::from_hms(12, 00, 00))
        .expect("Cannot parse date for lunch");
    let lunch_end = date.and_time(chrono::NaiveTime::from_hms(13, 00, 00))
        .expect("Cannot parse date for lunch");
    (start < lunch_end) && (end > lunch_start)
}

// Generate intervals for a desired meeting
pub fn generate_all_possible_meetings(
    tm: &DesiredMeeting,
    reject_date_fn: &Fn(chrono::Date<Tz>) -> bool,
    reject_datetime_fn: &Fn(chrono::DateTime<Tz>, chrono::DateTime<Tz>) -> bool,
) -> Vec<Meeting> {
    // Start date to end date, every day
    // Skip Wednesday, Saturday and Sunday
    // Call generate interval for a day
    // From min time, add duration until end time > end hour
    let mut res: Vec<Meeting> = Vec::new();
    let tz = tm.timezone;
    let start_date: chrono::DateTime<Tz> = tm.min_date.with_timezone(&tz);
    let end_date: chrono::DateTime<Tz> = tm.max_date.with_timezone(&tz);
    let mut date = start_date.date();
    let mint = start_date.time();
    let maxt = end_date.time();
    let mut id = 0;
    loop {
        if date > end_date.date() {
            break;
        }
        if reject_date_fn(date) {
            date = date + chrono::Duration::days(1);
            continue;
        }

        let ident = format!("{}_{}", tm.slug, id);
        res.extend(generate_meetings_for_date(
            date,
            mint,
            maxt,
            tm.step,
            tm.duration,
            &ident,
            reject_datetime_fn,
        ));
        date = date + chrono::Duration::days(1);
        id += 1
    }
    res
}

#[test]
fn test_generate_interval() {
    let intervals = generate_all_possible_meetings(
        &test_desired_meeting(),
        &default_reject_date,
        &default_reject_datetime,
    );
    // from 11 am to 4 PM on Thursday and Friday, the Saturday is ignored
    // Removing the lunch both days, how many intervals are there?
    // Morning (2)
    // 11 to 1130 1130 to 12
    // Afternoon(6)
    // 1 to 130 // 130 to 2 // 2 to 230 // 230 to 3
    // 3 to 330 // 330 to 4
    // So 20 intervals total
    assert_eq!(intervals.len(), 16);
}

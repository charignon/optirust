use chrono;
use types::{Meeting, DesiredMeeting};
use chrono_tz::Tz;
use chrono::prelude::*;
use fixtures::{test_desired_meeting};
// True if the YAML entry is a valid meeting, a valid meeting
// is one that does not last for a full day
// Generate intervals for a desired date, respecting mint, maxt, step and duration
fn generate_meetings_for_date(
    date:chrono::Date<Tz>,
    mint:chrono::NaiveTime,
    maxt:chrono::NaiveTime,
    step:chrono::Duration,
    duration:chrono::Duration,
    id:String)
    -> Vec<Meeting> {

    let mut res:Vec<Meeting> = Vec::new();
    let mut t = date.and_time(mint).unwrap();
    loop {
        let start = t;
        let end = t + duration;
        if end.time() > maxt {
            break
        }
        t = t + step;
        // Lunch
        if (start.hour() > 12 && start.hour() < 13) || (end.hour() > 12 && end.hour() < 13) {
            continue
        }
        res.push(Meeting{
            id: id.clone(),
            start: start.with_timezone(&chrono::Utc),
            end: end.with_timezone(&chrono::Utc),
        })
    }
    res
}

// Generate intervals for a desired meeting
pub fn generate_all_possible_meetings(tm: &DesiredMeeting) -> Vec<Meeting> {
    // Start date to end date, every day
    // Skip Wednesday, Saturday and Sunday
    // Call generate interval for a day
    // From min time, add duration until end time > end hour
    let mut res:Vec<Meeting> = Vec::new();
    let tz = tm.timezone;
    let start_date:chrono::DateTime<Tz> = tm.min_date.with_timezone(&tz);
    let end_date:chrono::DateTime<Tz> = tm.max_date.with_timezone(&tz);
    let mut date = start_date.date();
    let mint = start_date.time();
    let maxt = end_date.time();
    loop {
        if date > end_date.date() {
            break
        }
        let wkday = date.weekday();
        if //wkday == chrono::Weekday::Wed ||
           wkday == chrono::Weekday::Sat ||
           wkday == chrono::Weekday::Sun
        {
            date = date + chrono::Duration::days(1);
            continue;
        }

        res.extend(
            generate_meetings_for_date(date, mint, maxt, tm.step, tm.duration, tm.title.clone())
        );
        date = date + chrono::Duration::days(1);
    }
    res
}

#[test]
fn test_generate_interval() {
    let intervals =  generate_all_possible_meetings(&test_desired_meeting());
    // from 11 am to 4 PM on Thursday and Friday, the Saturday is ignored
    // Removing the lunch both days, how many intervals are there?
    // Morning (2)
    // 11 to 1130 1130 to 12
    // Afternoon(8)
    // 12 to 1230 // 1230 to 1 // 1 to 130 // 130 to 2 // 2 to 230 // 230 to 3
    // 3 to 330 // 330 to 4
    // So 20 intervals total
    assert_eq!(intervals.len(), 20);
}

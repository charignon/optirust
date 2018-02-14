use types::{DesiredMeeting, Meeting, MeetingCandidate, MeetingsTree, Solution};

use calendar3;
use hyper;
use oauth2;
use chrono;
use hyper_rustls;
use rayon;
use bio::data_structures::interval_tree::IntervalTree;
use std::path::Path;
use std::collections::HashMap;
use calendar3::CalendarHub;
use self::oauth2::{read_application_secret, ApplicationSecret, Authenticator,
                   DefaultAuthenticatorDelegate, DiskTokenStorage, FlowType};
use self::hyper::net::HttpsConnector;
use self::rayon::prelude::*;
use chrono::prelude::*;

impl Meeting {
    fn from_api(s: calendar3::Event) -> Meeting {
        Meeting {
            id: s.id.unwrap(),
            start: chrono::DateTime::parse_from_rfc3339(&s.start.unwrap().date_time.unwrap())
                .unwrap()
                .with_timezone(&chrono::Utc),
            end: chrono::DateTime::parse_from_rfc3339(&s.end.unwrap().date_time.unwrap())
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }
}
type CalendarHubType = CalendarHub<
    hyper::Client,
    Authenticator<DefaultAuthenticatorDelegate, DiskTokenStorage, hyper::Client>,
>;
fn candidate_and_meeting_to_event(
    desired_meeting: &DesiredMeeting,
    candidate: &MeetingCandidate,
) -> calendar3::Event {
    let mut attendees: Vec<String> = Vec::new();
    attendees.extend(desired_meeting.attendees.clone());
    if let Some(room) = candidate.room.clone() {
        attendees.push(room);
    }

    let attendees = Some(
        attendees
            .iter()
            .map(|k| calendar3::EventAttendee {
                email: Some(k.to_string()),
                response_status: Some("needsAction".to_string()),
                ..Default::default()
            })
            .collect::<Vec<calendar3::EventAttendee>>(),
    );

    calendar3::Event {
        attendees,
        start: Some(calendar3::EventDateTime {
            date_time: Some(candidate.start.to_rfc3339()),
            ..Default::default()
        }),
        end: Some(calendar3::EventDateTime {
            date_time: Some(candidate.end.to_rfc3339()),
            ..Default::default()
        }),
        description: Some(desired_meeting.description.to_string()),
        reminders: Some(calendar3::EventReminders {
            use_default: Some(true),
            overrides: None,
        }),
        summary: Some(desired_meeting.title.to_string()),
        ..Default::default()
    }
}

pub fn book_with_api(s: &Solution) {
    let mut es: Vec<calendar3::Event> = Vec::new();

    for (desired_meeting, candidate) in &s.candidates {
        es.push(candidate_and_meeting_to_event(desired_meeting, candidate));
    }

    es.par_iter()
        .map(|e| {
            get_calendar_hub()
                .events()
                .insert(e.clone(), "primary")
                .doit()
                .unwrap()
        })
        .collect::<Vec<(hyper::client::Response, calendar3::Event)>>();
}

// Return a CalendarHub object to work with the google calendar API
pub fn get_calendar_hub() -> CalendarHubType {
    let secret = read_client_secret(CLIENT_SECRET_FILE);
    let client = hyper::Client::with_connector(HttpsConnector::new(hyper_rustls::TlsClient::new()));
    let authenticator = Authenticator::new(
        &secret,
        DefaultAuthenticatorDelegate,
        client,
        DiskTokenStorage::new(&"token_store.json".to_string()).unwrap(),
        Some(FlowType::InstalledInteractive),
    );
    let client = hyper::Client::with_connector(HttpsConnector::new(hyper_rustls::TlsClient::new()));
    CalendarHub::new(client, authenticator)
}

fn valid_api_meeting(
    person: &str,
    l: calendar3::Event,
    _ignore_all_day_events: bool,
    ignore_meetings_with_no_response: bool,
) -> bool {
    let has_bound = !l.start.unwrap().date_time.is_none() && !l.end.unwrap().date_time.is_none();
    if l.attendees.is_none() {
        return has_bound;
    }
    let attendees: Vec<calendar3::EventAttendee> = l.attendees.unwrap();
    // TODO Make that more idiomatic
    // "accepted" or "tentative"
    let attendees_status: Vec<String> = attendees
        .into_iter()
        .filter(|k| k.email.clone().unwrap_or_else(|| "XXX".to_string()) == person)
        .map(|l| l.response_status.unwrap())
        .collect();
    if attendees_status.len() != 1 {
        return false;
    }
    let status = &attendees_status[0];
    //
    // Expected behavior:
    //
    // ignore_meetings_with_no_response | status        | Valid?
    //////////////////////////////////////////////////////////////
    // true                             | tentative     | true
    // true                             | accepted      | true
    // true                             | declined      | false
    // true                             | needsAction   | false
    // false                            | tentative     | true
    // false                            | accepted      | true
    // false                            | declined      | true
    // false                            | needsAction   | true

    let has_no_response = !vec!["accepted".to_string(), "tentative".to_string()].contains(status);
    if ignore_meetings_with_no_response && has_no_response {
        return false;
    }
    has_bound
}

// Convert a vector of meeting to an interval tree for ease of
// intersection computation
fn meetings_to_tree(meetings: &[Meeting]) -> MeetingsTree {
    let mut intervals: MeetingsTree = IntervalTree::new();
    for m in meetings {
        intervals.insert(m.start..m.end, m.id.clone());
    }
    intervals
}

fn fetch_one_availability_with_api(
    person: &str,
    hub: &CalendarHubType,
    ignore_all_day_events: bool,
    ignore_meetings_with_no_response: bool,
) -> MeetingsTree {
    println!("Fetching for {:?}", person);
    let result = hub.events()
        .list(person)
        .max_results(200)
        .order_by("startTime")
        .single_events(true)
        .time_min(&chrono::Utc::now().to_rfc3339())
        .doit();
    let (_, events) = result.unwrap();
    let events: Vec<calendar3::Event> = events.items.unwrap();

    meetings_to_tree(&events
        .into_iter()
        .filter(|l| {
            valid_api_meeting(
                person,
                l.clone(),
                ignore_all_day_events,
                ignore_meetings_with_no_response,
            )
        })
        .map(Meeting::from_api)
        .collect::<Vec<Meeting>>())
}

pub fn fetch_availability_with_api(
    people: Vec<String>,
    ignore_all_day_events: bool,
    ignore_meetings_with_no_response: bool,
) -> HashMap<String, MeetingsTree> {
    let mut res: HashMap<String, IntervalTree<DateTime<chrono::Utc>, String>> = HashMap::new();

    let availability = people
        .par_iter()
        .map(|a| {
            fetch_one_availability_with_api(
                a,
                &get_calendar_hub(),
                ignore_all_day_events,
                ignore_meetings_with_no_response,
            )
        })
        .collect::<Vec<MeetingsTree>>()
        .into_iter();

    for it in people.into_iter().zip(availability) {
        let (p, t) = it;
        res.insert(p, t);
    }

    res
}

const CLIENT_SECRET_FILE: &str = "client_secret.json";

// reads the JSON secret file
fn read_client_secret(file: &str) -> ApplicationSecret {
    read_application_secret(Path::new(file)).unwrap()
}

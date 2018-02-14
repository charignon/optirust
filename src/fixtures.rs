use types::{DesiredMeeting, MeetingCandidate, MeetingsTree};
use std::collections::HashMap;
use types;
use chrono;

#[allow(dead_code)]
pub fn test_input() -> String {
    "- title: title
  description: description
  attendees:
  - laurent.charignon@foo.com
  - foo.bar@laurent.com
  min_date: 2018-02-08T10:00:00
  max_date: 2018-02-20T18:00:00
- title: title2
  description: description 2
  attendees:
  - laurent.charignon@foo.com
  - contact@laurent.com
  min_date: 2018-02-08T11:00:00
  max_date: 2018-02-10T16:00:00
"
        .to_string()
}

#[allow(dead_code)]
pub fn test_invalid_input() -> String {
    "- title: title
  description: description
  attendees:
  - laurent.charignon@foo.com
  - foo.bar@laurent.com
  min_date: 2018-02-08T10:00:00
  max_date: 2018-02-20T18:00:00
- title: title
  description: description 2
  attendees:
  - laurent.charignon@foo.com
  - contact@laurent.com
  min_date: 2018-02-08T11:00:00
  max_date: 2018-02-10T16:00:00
"
        .to_string()
}

#[allow(dead_code)]
pub fn fetch_results(emails: Vec<String>) -> HashMap<String, MeetingsTree> {
    let mut l = HashMap::new();
    for i in emails {
        let mut k = MeetingsTree::new();
        if i == "laurent.charignon@foo.com" {
            let from = "2018-02-08T14:00:00-08:00"
                .parse::<chrono::DateTime<chrono::Utc>>()
                .expect("Error from");
            let to = "2018-02-08T14:45:00-08:00"
                .parse::<chrono::DateTime<chrono::Utc>>()
                .expect("error to");
            k.insert(from..to, "meeting".to_string())
        }
        l.insert(i, k);
    }
    l
}

#[allow(dead_code)]
pub fn test_config() -> String {
    "
small_rooms:
  - foo@bar.com
  - joe@baz.com
large_rooms:
  - bozorg@jam.com
"
        .to_string()
}

#[allow(dead_code)]
pub fn test_desired_meetings() -> Vec<DesiredMeeting> {
    let a = types::read_input_str(&test_input());
    a.clone()
}

#[allow(dead_code)]
pub fn test_desired_meeting() -> DesiredMeeting {
    let a = types::read_input_str(&test_input());
    a[1].clone()
}

#[allow(dead_code)]
pub fn sample_cbc_solution() -> String {
    "Optimal - objective value -2422.00000000
   3576 id10873                 1                   -1161
  18404 id0                     1                   -1261"
        .to_string()
}

#[allow(dead_code)]
pub fn sample_candidate_a() -> MeetingCandidate {
    MeetingCandidate {
        title: "title".to_string(),
        id: "id10873".to_string(),
        start: chrono::Utc::now(),
        end: chrono::Utc::now(),
        room: Some("foo".to_string()),
        score: 23,
    }
}

#[allow(dead_code)]
pub fn sample_candidate_b() -> MeetingCandidate {
    MeetingCandidate {
        title: "title2".to_string(),
        id: "0".to_string(),
        start: chrono::Utc::now(),
        end: chrono::Utc::now(),
        room: Some("bar".to_string()),
        score: 23,
    }
}

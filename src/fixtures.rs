use types::{DesiredMeeting, Input, MeetingCandidate};
use chrono;

#[allow(dead_code)]
pub fn test_input() -> String {
    "
meetings:
  - title: title
    description: description
    attendees:
    - laurent.charignon@foo.com
    - foo.bar@laurent.com
    min_date: 2018-02-08 10:00:00
    max_date: 2018-02-20 18:00:00
  - title: title2
    description: description 2
    attendees:
    - laurent.charignon@foo.com
    - contact@laurent.com
    min_date: 2018-02-08 11:00:00
    max_date: 2018-02-10 16:00:00
".to_string()
}

#[allow(dead_code)]
pub fn test_config() -> String {
    "
rooms:
  small:
    - name: Foo
      email: foo@bar.com
    - name: Bar
      email: joe@baz.com
  large:
    - name: Bozorg
      email: bozorg@jam.com
".to_string()
}

#[allow(dead_code)]
pub fn test_desired_meetings() -> Vec<DesiredMeeting> {
    let a = Input::from_yaml_str(&test_input());
    return a.meetings.clone();
}


#[allow(dead_code)]
pub fn test_desired_meeting() -> DesiredMeeting {
    let a = Input::from_yaml_str(&test_input());
    return a.meetings[1].clone();
}

#[allow(dead_code)]
pub fn sample_cbc_solution() -> String {
    "Optimal - objective value -2422.00000000
   3576 id10873                 1                   -1161
  18404 id0                     1                   -1261".to_string()
}

#[allow(dead_code)]
pub fn sample_candidate_a() -> MeetingCandidate{
    MeetingCandidate{
        title: "title".to_string(),
        id: "id10873".to_string(),
        start: chrono::Utc::now(),
        end: chrono::Utc::now(),
        room: "foo".to_string(),
        score: 23
    }
}

#[allow(dead_code)]
pub fn sample_candidate_b() -> MeetingCandidate{
    MeetingCandidate{
        title: "title2".to_string(),
        id: "0".to_string(),
        start: chrono::Utc::now(),
        end: chrono::Utc::now(),
        room: "bar".to_string(),
        score: 23
    }
}

use std::hash::{Hash, Hasher};
use std::collections::HashMap;
use std::collections::HashSet;
use std::iter::FromIterator;
use std::io::prelude::*;
use std::fs::File;

use bio::data_structures::interval_tree::IntervalTree;
use chrono::prelude::*;
use chrono;
use gcal;
use gen;
use serde_yaml;
use solver;
use chrono_tz::Tz;
use std::fmt;
use fixtures::{test_config, test_input, test_invalid_input};
pub type ScoringFnType = Box<
    Fn(&chrono::DateTime<Utc>, &chrono::DateTime<Utc>, &[String], &HashMap<String, MeetingsTree>)
        -> usize,
>;
pub type RejectDateTimeFnType = Box<Fn(chrono::DateTime<Tz>, chrono::DateTime<Tz>) -> bool>;
pub type RejectDateFnType = Box<Fn(chrono::Date<Tz>) -> bool>;
pub type RoomPickerFnType = Box<Fn(usize) -> Vec<String>>;
pub type SolverFnType =
    Box<Fn(&solver::SolverInput) -> Option<HashMap<DesiredMeeting, MeetingCandidate>>>;
pub type FetchFnType = Box<Fn(Vec<String>, bool, bool) -> HashMap<String, MeetingsTree>>;

// Options is a struct to represent all the tweakable part of the workflow
// it can be used to modify the behavior of the whole program for example by
// swapping scoring functions, fetching strategy or room picking algorithm.
// It should we built at the high level from the user input.
pub struct Options {
    // How to fetch the meetings from the API
    // Default: fetching in // with google calendar
    pub fetch_fn: FetchFnType,

    // How to solve the problem
    // Default: use a CBC solver
    pub solver_fn: SolverFnType,

    // Given the size of a meeting returns a list of email addresses of rooms where it
    // could happen.
    // Default: no room booked
    pub room_picker_fn: RoomPickerFnType,

    // Function to decide what day to reject. You can use that to reject meetings on
    // weekend for example
    // Default: reject Wednesdays and weekend
    pub reject_date_fn: RejectDateFnType,

    // Function what slot to reject, you can use that to reject meetings over
    // lunch for example
    // Default: reject meeting over lunch (12 to 1pm)
    pub reject_datetime_fn: RejectDateTimeFnType,

    // Scoring function
    // Default: score is high for clustered meetings (to avoid fragmentation)
    pub scoring_fn: ScoringFnType,

    // If true will ignore all day events when scheduling
    // Default: true, we ignore all day events (false isn't supported, TODO to implement)
    pub ignore_all_day_events: bool,

    // If true will ignore meetings with no response and
    // try to schedule over them
    // default: true
    pub ignore_meetings_with_no_response: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            fetch_fn: Box::new(gcal::fetch_availability_with_api),
            solver_fn: Box::new(solver::solve_with_cbc_solver),
            scoring_fn: Box::new(compute_score),
            ignore_all_day_events: true,
            ignore_meetings_with_no_response: true,
            room_picker_fn: Box::new(|_| vec![]),
            reject_date_fn: Box::new(gen::default_reject_date),
            reject_datetime_fn: Box::new(gen::default_reject_datetime),
        }
    }
}

// Compute the score for a slot given list of attendees and their availability
// Can be better
fn compute_score(
    start: &chrono::DateTime<chrono::Utc>,
    end: &chrono::DateTime<chrono::Utc>,
    attendees: &[String],
    availability: &HashMap<String, MeetingsTree>,
) -> usize {
    let mut score = 1;

    // TODO Need to add score based on start of the range
    for a in attendees.iter() {
        // Get all the meetings in the two hour range. Each of them is worth 20
        score += availability[a]
            .find(*start - chrono::Duration::hours(2)..*end + chrono::Duration::hours(2))
            .count() * 20;
        // Get all the meetings in the 1 hour range. Each of them is worth 100
        score += availability[a]
            .find(*start - chrono::Duration::hours(1)..*end + chrono::Duration::hours(1))
            .count() * 100;
        // Get all the meetings in the 30 min range. Each of them is worth 300
        score += availability[a]
            .find(*start - chrono::Duration::minutes(30)..*end + chrono::Duration::minutes(30))
            .count() * 300;
        // Get all the meetings in the 15 min range. Each of them is worth 600
        score += availability[a]
            .find(*start - chrono::Duration::minutes(15)..*end + chrono::Duration::minutes(15))
            .count() * 600;
    }
    score
}

// rooms available to book, small means 2 people or less
// large 3+ people
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub small_rooms: Vec<String>,
    pub large_rooms: Vec<String>,
}

impl Config {
    pub fn from_yaml_str(s: &str) -> Config {
        serde_yaml::from_str(&s).unwrap()
    }

    pub fn room_picker(&self, size: usize) -> Vec<String> {
        if size <= 2 {
            self.small_rooms.clone()
        } else {
            self.large_rooms.clone()
        }
    }

    pub fn from_file(file: &str) -> Config {
        let mut config = File::open(file).expect("file not found");
        let mut contents = String::new();
        config
            .read_to_string(&mut contents)
            .expect("something went wrong reading the file");
        Config::from_yaml_str(&contents)
    }
}
// A potential meeting, linked to a desired meeting
#[derive(Clone, PartialEq)]
pub struct MeetingCandidate {
    pub title: String,
    pub id: String,
    pub start: DateTime<chrono::Utc>,
    pub end: DateTime<chrono::Utc>,
    pub room: String,
    pub score: usize,
}

impl MeetingCandidate {
    #[allow(dead_code)]
    pub fn intersects(&self, o: &MeetingCandidate) -> bool {
        (self.start < o.end) && (o.start < self.end)
    }
}

impl fmt::Debug for MeetingCandidate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Candidate {:?} {{ start_time: {:?}, end_time: {:?}, room: {:?}, score: {:?} }}",
            self.id,
            self.start.with_timezone(&chrono::Local).to_string(),
            self.end.with_timezone(&chrono::Local).to_string(),
            self.room,
            self.score
        )
    }
}
impl fmt::Debug for Solution {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let solved = format!("Solved: {:?}\n", self.solved);
        let candidates_str = self.candidates
            .iter()
            .map(|it| format!("{:?}\n >>> {:?}\n", it.0, it.1))
            .collect::<Vec<String>>()
            .join("===============================\n");
        write!(f, "{}\n{}\n", solved, candidates_str)
    }
}

// A meeting that is already scheduled before the program runs
// There meetings are stored as a tree for easy intersection
// computation
#[derive(Debug)]
pub struct Meeting {
    pub id: String,
    pub start: DateTime<chrono::Utc>,
    pub end: DateTime<chrono::Utc>,
}

pub type MeetingsTree = IntervalTree<DateTime<chrono::Utc>, String>;

pub struct Solution {
    pub solved: bool,
    pub candidates: HashMap<DesiredMeeting, MeetingCandidate>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct InputDesiredMeeting {
    title: String,
    description: String,
    attendees: Vec<String>,
    min_date: chrono::NaiveDateTime,
    max_date: chrono::NaiveDateTime,
    step: Option<i64>,
    duration: Option<i64>,
    timezone: Option<String>,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct DesiredMeeting {
    pub title: String,
    pub slug: String,
    pub description: String,
    pub attendees: Vec<String>,          // email
    pub min_date: DateTime<chrono::Utc>, // Parse as a date
    pub max_date: DateTime<chrono::Utc>, // Parse as a date
    pub step: chrono::Duration,
    pub duration: chrono::Duration,
    pub timezone: Tz,
}

fn to_slug(s: &str) -> String {
    "foo".to_string()
}

impl DesiredMeeting {
    fn from_input_desired_meeting(i: &InputDesiredMeeting) -> DesiredMeeting {
        let timezone = i.timezone
            .clone()
            .unwrap_or("America/Los_Angeles".to_string());
        let tz: Tz = timezone.parse().unwrap();
        let min_d = tz.from_local_datetime(&i.min_date)
            .unwrap()
            .with_timezone(&Utc);
        let max_d = tz.from_local_datetime(&i.max_date)
            .unwrap()
            .with_timezone(&Utc);
        let duration = chrono::Duration::minutes(i.duration.unwrap_or(30));
        let step = chrono::Duration::minutes(i.step.unwrap_or(30));
        DesiredMeeting {
            title: i.title.clone(),
            slug: to_slug(&i.title),
            description: i.description.clone(),
            attendees: i.attendees.clone(),
            min_date: min_d,
            max_date: max_d,
            step: step,
            duration: duration,
            timezone: tz,
        }
    }
}

fn panic_if_invalid(meetings: &Vec<InputDesiredMeeting>) {
    let all_titles = meetings
        .iter()
        .map(|k| k.title.to_string())
        .collect::<Vec<String>>();
    let all_titles_count = all_titles.len();
    let titles_set: HashSet<String> = HashSet::from_iter(all_titles.into_iter());
    if titles_set.len() != all_titles_count {
        panic!("Two meetings cannot have the same title");
    }
}

pub fn read_input_str(content: &str) -> Vec<DesiredMeeting> {
    let input: Vec<InputDesiredMeeting> = serde_yaml::from_str(&content).unwrap();
    panic_if_invalid(&input);
    input
        .iter()
        .map(DesiredMeeting::from_input_desired_meeting)
        .collect()
}

pub fn read_input(file: &str) -> Vec<DesiredMeeting> {
    let mut input = File::open(file).expect("file not found");
    let mut contents = String::new();
    input
        .read_to_string(&mut contents)
        .expect("something went wrong reading the file");
    read_input_str(&contents)
}

impl Hash for DesiredMeeting {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.title.hash(state);
        self.description.hash(state);
    }
}

#[test]
#[should_panic]
fn panic_two_desired_meeting_same_title() {
    read_input_str(&test_invalid_input());
}

#[test]
fn can_build_input() {
    let a = read_input_str(&test_input());
    assert_eq!(a[0].title, "title");
    assert_eq!(a[0].description, "description");
    assert_eq!(a[0].attendees[0], "laurent.charignon@foo.com");
    assert_eq!(a[0].timezone.name(), "America/Los_Angeles");
    // 10 PST -> 18 UTC
    // 18 PST -> 2 UTC
    assert_eq!(a[0].min_date.hour(), 18);
    assert_eq!(a[0].max_date.hour(), 2);
}

#[test]
fn can_build_config() {
    let a = Config::from_yaml_str(&test_config());
    assert_eq!(a.large_rooms[0], "bozorg@jam.com")
}

/* Project Optirust


- TODO Better testing for generate_solution!
- TODO Flag to ignore the non accepted meeting
- TODO Make it possible to accept day long event
- TODO Make sure the code works when no room specified

- TODO Make the input parsing a desired meeting parsing and remove one type
- TODO Make constraints configurable using a config file
- TODO Add example files in a separate folder
- TODO Explain better how to use the program
- TODO Test usability on a new host
- TODO Log level

 */
extern crate bio;
extern crate chrono;
extern crate chrono_tz;
#[macro_use]
extern crate clap;
extern crate serde_yaml;
extern crate yaml_rust;
extern crate google_calendar3 as calendar3;
extern crate yup_oauth2 as oauth2;
extern crate hyper;
extern crate hyper_rustls;
extern crate rayon;
use chrono::prelude::*;
use chrono_tz::Tz;
use std::collections::HashMap;
use std::collections::HashSet;
use std::iter::FromIterator;
use std::process;
use std::vec::Vec;
use std::ops::Fn;

mod app;
mod fixtures;
mod gcal;
mod gen;
mod solver;
mod types;

use bio::data_structures::interval_tree::IntervalTree;
use fixtures::{test_input, test_config};
use types::{Input, Config, DesiredMeeting, MeetingsTree,
            RoomConfig, MeetingCandidate, Solution, Meeting};


type ScoringFnType = Box<Fn(
    &chrono::DateTime<Utc>, &chrono::DateTime<Utc>,
    &Vec<String>, &HashMap<String, MeetingsTree>) -> usize>;
type RejectDateTimeFnType = Box<Fn(chrono::DateTime<Tz>, chrono::DateTime<Tz>) -> bool>;
type RejectDateFnType = Box<Fn(chrono::Date<Tz>) -> bool>;
type RoomPickerFnType = Box<Fn(usize) -> Vec<String>>;
type SolverFnType = Box<Fn(&solver::SolverInput) -> Option<HashMap<DesiredMeeting, MeetingCandidate>>>;
type FetchFnType = Box<Fn(Vec<String>) -> HashMap<String, MeetingsTree>>;

// Options is a struct to represent all the tweakable part of the workflow
// it can be used to modify the behavior of the whole program for example by
// swapping scoring functions, fetching strategy or room picking algorithm.
// It should we built at the high level from the user input.
struct Options {
    // How to fetch the meetings from the API
    // Default: fetching in // with google calendar
    fetch_fn: FetchFnType,

    // How to solve the problem
    // Default: use a CBC solver
    solver_fn: SolverFnType,

    // Given the size of a meeting returns a list of email addresses of rooms where it
    // could happen.
    // Default: no room booked
    room_picker_fn: RoomPickerFnType,

    // Function to decide what day to reject. You can use that to reject meetings on
    // weekend for example
    // Default: reject Wednesdays and weekend
    reject_date_fn: RejectDateFnType,

    // Function what slot to reject, you can use that to reject meetings over
    // lunch for example
    // Default: reject meeting over lunch (12 to 1pm)
    reject_datetime_fn: RejectDateTimeFnType,

    // Scoring function
    // Default: score is high for clustered meetings (to avoid fragmentation)
    scoring_fn: ScoringFnType,

    // TODO Use this, currently unused
    // If true will ignore all day events when scheduling
    // Default: true, we ignore all day events
    ignore_all_day_events: bool,

    // TODO Use
    // If true will consider pending meeting busy and not try to schedule over them
    // default: true
    consider_pending_meetings_busy: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            fetch_fn: Box::new(gcal::fetch_availability_with_api),
            solver_fn: Box::new(solver::solve_with_cbc_solver),
            scoring_fn: Box::new(compute_score),
            ignore_all_day_events: true,
            consider_pending_meetings_busy: true,
            room_picker_fn: Box::new(|_| vec![]),
            reject_date_fn: Box::new(gen::default_reject_date),
            reject_datetime_fn: Box::new(gen::default_reject_datetime),
        }
    }
}

// Extract email address from a vec of RoomConfig
fn rooms_vec_to_emails(v: &Vec<RoomConfig>) -> Vec<String> {
    v.into_iter().map(|k| k.email.to_string()).collect::<Vec<String>>()
}

// Extract the list of attendees emails from the input and config
fn extract_attendees(i: &Vec<DesiredMeeting>, c: &RoomPickerFnType) -> Vec<String>
{
    let mut s: HashSet<String> = HashSet::new();
    for m in i {
        let attendees = m.attendees.iter().map(|k| k.to_string()).collect::<Vec<String>>();
        s.extend(c(attendees.len()));
        s.extend(attendees);
    }
    Vec::from_iter(s.into_iter())
}

// Compute the score for a slot given list of attendees and their availability
// Can be better
fn compute_score(start: &chrono::DateTime<chrono::Utc>,
                 end: &chrono::DateTime<chrono::Utc>,
                 attendees: &Vec<String>,
                 availability: &HashMap<String, MeetingsTree>
) -> usize {
    let mut score = 1;

    // TODO Need to add score based on start of the range
    for a in attendees.iter() {
        // Get all the meetings in the two hour range. Each of them is worth 20
        score += availability[a].find(*start-chrono::Duration::hours(2)..*end+chrono::Duration::hours(2)).count() * 20;
        // Get all the meetings in the 1 hour range. Each of them is worth 100
        score += availability[a].find(*start-chrono::Duration::hours(1)..*end+chrono::Duration::hours(1)).count() * 100;
        // Get all the meetings in the 30 min range. Each of them is worth 300
        score += availability[a].find(*start-chrono::Duration::minutes(30)..*end+chrono::Duration::minutes(30)).count() * 300;
        // Get all the meetings in the 15 min range. Each of them is worth 600
        score += availability[a].find(*start-chrono::Duration::minutes(15)..*end+chrono::Duration::minutes(15)).count() * 600;
    }
    score
}

fn default_room_picker(size: usize, config: &Config) -> Vec<String>{
    if size <= 2 {
        rooms_vec_to_emails(&config.small_rooms)
    } else {
        rooms_vec_to_emails(&config.large_rooms)
    }
}

// Generated a candidate for a desired meeting for the interval specified by Meeting
// None if not possible (no availability)
fn generate_meeting_candidate(
    tm: &DesiredMeeting,
    avail: &HashMap<String, MeetingsTree>,
    ident: String,
    room_picker: &RoomPickerFnType,
    scoring_fn: &ScoringFnType,
    i: &Meeting
) -> Option<MeetingCandidate>
{

    let possible_rooms:Vec<String> = room_picker(tm.attendees.len());
    let mandatory_attendees = &tm.attendees;
    let conflicts:usize = mandatory_attendees
        .iter()
        .map(|k| avail[k].find(i.start..i.end).count())
        .sum();


    if conflicts != 0 {
        return None;
    }

    // What is a suitable room?
    let mut suitable_room:Option<String> = None;
    for r in &possible_rooms {
        if avail[r].find(i.start..i.end).count() == 0 {
            suitable_room = Some(r.to_string());
            break;
        }
    }

    if suitable_room.is_none() {
        return None;
    }

    // Create a suitable candidate
    Some(MeetingCandidate{
        title: tm.title.to_string(),
        id: ident,
        start: i.start,
        end: i.end,
        room: suitable_room.unwrap(),
        score: scoring_fn(&i.start, &i.end, &mandatory_attendees, &avail)
    })
}

// Given a fetch function (to fetch availability), input and output return a solution
// to the scheduling problem using a given solving strategy
fn generate_solution(
    desired_meetings: &Vec<DesiredMeeting>,
    opts: &Options
) -> Solution {
    // the goal of the function is to build the input for the solver
    // This will be fed to the solver at the end of the function
    let mut solver_input = solver::SolverInput::new();
    solver_input.desired_meetings = desired_meetings.clone();


    // Build the tree of all the candidates as well as three of the solver's input
    // fields: the list of candidates and groupping of candidates for each desired meetings
    let mut tree:IntervalTree<DateTime<chrono::Utc>, String> = IntervalTree::new();
    println!("First loop!");
    {
        let avail:HashMap<String, MeetingsTree> = (opts.fetch_fn)(extract_attendees(desired_meetings, &opts.room_picker_fn));
        let mut id = 0;
        for me in desired_meetings {
            for interval in gen::generate_all_possible_meetings(&me, &opts.reject_date_fn, &opts.reject_datetime_fn) {
                let ident = format!("id{}", id);
                if let Some(m) = generate_meeting_candidate(me, &avail, ident.clone(), &opts.room_picker_fn, &opts.scoring_fn, &interval) {
                    solver_input.candidates.insert(ident.to_string(), m);
                    id += 1;
                    solver_input.candidate_per_desired_meeting
                        .entry(me.title.to_string())
                        .or_insert(Vec::new())
                        .push(ident.to_string());
                    tree.insert(interval.start..interval.end, ident.to_string());
                }
            }
        }
    }

    // For each candidate check if it intersects with other things from the tree
    // aggregate all the pair of intervals that intersect
    println!("Second loop!");
    {
        let mut intersections_set:HashSet<String> = HashSet::new();
        for c in &solver_input.candidates {
            let ref ident = c.0;
            let mut intersect = tree.find(c.1.start..c.1.end).map(|r| r.data().to_string()).collect::<Vec<String>>();
            for k in &intersect {
                if k != *ident {
                    let (small, big) = if k < *ident {
                        (k, *ident)
                    } else {
                        (*ident, k)
                    };
                    let combined = vec![small.to_string(), big.to_string()];
                    let combined_ident = combined.clone().join("-");
                    if !intersections_set.contains(&combined_ident) {
                        solver_input.intersections.push(combined.clone());
                        intersections_set.insert(combined_ident.clone());
                    }
                }
            }
        }
    }

    // Feed all the input to the solver to find an optimal solution
    println!("Calling solver!");
    match (opts.solver_fn)(&solver_input) {
        Some(m) => Solution{solved: true, candidates: m},
        None => Solution{solved: false, candidates: HashMap::new()}
    }
}

fn main() {
    let matches = app::build_app().get_matches();
    let options = {
        let config = Config::from_file(
            matches.value_of("config").expect("Please give a valid config file")
        );
        Options{
            room_picker_fn: Box::new(move |k| default_room_picker(k, &config)),
            ..Default::default()
        }
    };

    let input = Input::from_file(
        matches.value_of("input").expect("Please give a valid input file")
    );

    let sol = generate_solution(&input.meetings, &options);

    if !sol.solved {
        eprintln!("Cannot find meetings to solve the constraints!");
        process::exit(1);
    }

    println!("{:?}", sol);
    match matches.occurrences_of("book") {
        0 => println!("Dry run mode, not booking!"),
        _ => {
            println!("Booking!");
            gcal::book_with_api(&sol);
        }
    }
}


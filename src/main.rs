/* Project Optirust

- TODO Better testing!
- TODO Make it possible to accept day long event
- TODO Make constraints configurable
- TODO Factor out part that people might want to override
- TODO Explain better how to use the program
- TODO Test usability on a new host

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

use gcal::{book_with_api, fetch_availability_with_api};

// Extract email address from a vec of RoomConfig
fn rooms_vec_to_emails(v: &Vec<RoomConfig>) -> Vec<String> {
    v.into_iter().map(|k| k.email.to_string()).collect::<Vec<String>>()
}

// Extract the list of attendees emails from the input and config
fn extract_attendees(i: &Input, c: &Config) -> Vec<String> {
    let mut s: HashSet<String> = HashSet::new();
    s.extend(rooms_vec_to_emails(&c.small_rooms));
    s.extend(rooms_vec_to_emails(&c.large_rooms));

    for m in &i.meetings {
        s.extend(m.attendees.iter().map(|k| k.to_string()).collect::<Vec<String>>());
    }
    Vec::from_iter(s.into_iter())
}

#[test]
fn test_extract_attendees() {
    let mut a = extract_attendees(
        &Input::from_yaml_str(&test_input()),
        &Config::from_yaml_str(&test_config())
    );
    a.sort();
    assert_eq!(a,
               ["bozorg@jam.com",
                "contact@laurent.com",
                "foo.bar@laurent.com",
                "foo@bar.com",
                "joe@baz.com",
                "laurent.charignon@foo.com"]
)
}

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


fn generate_meeting_candidate(tm: &DesiredMeeting,
                              avail: &HashMap<String, MeetingsTree>,
                              config: &Config,
                              ident: String,
                              i: &Meeting) -> Option<MeetingCandidate> {

    let possible_rooms:Vec<String>;
    if tm.attendees.len() <= 2 {
        possible_rooms = rooms_vec_to_emails(&config.small_rooms);
    } else {
        possible_rooms = rooms_vec_to_emails(&config.large_rooms);
    }

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

    if suitable_room == None {
        return None;
    }

    // Create a suitable candidate
    Some(MeetingCandidate{
        title: tm.title.to_string(),
        id: ident,
        start: i.start,
        end: i.end,
        room: suitable_room.unwrap(),
        score: compute_score(&i.start, &i.end, &mandatory_attendees, &avail)
    })
}

fn generate_solution<F>(
    fetch: F,
    input: &Input,
    config: &Config
) -> Solution
where F: Fn(Vec<String>) -> HashMap<String, MeetingsTree> {
    // the goal of the function is to build the input for the solver
    // This will be fed to the solver at the end of the function
    let mut solver_input = solver::SolverInput::new();
    solver_input.desired_meetings = input.meetings.clone();


    // Build the tree of all the candidates as well as three of the solver's input
    // fields: the list of candidates and groupping of candidates for each desired meetings
    let mut tree:IntervalTree<DateTime<chrono::Utc>, String> = IntervalTree::new();
    println!("First loop!");
    {
        let avail:HashMap<String, MeetingsTree> = fetch(extract_attendees(input, config));
        let mut id = 0;
        for me in &input.meetings {
            for interval in gen::generate_all_possible_meetings(&me) {
                let ident = format!("id{}", id);
                if let Some(m) = generate_meeting_candidate(me, &avail, &config, ident.clone() , &interval) {
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
    match solver::solve_with_cbc_solver(&solver_input) {
        Some(m) => Solution{solved: true, candidates: m},
        None => Solution{solved: false, candidates: HashMap::new()}
    }
}

fn main() {
    let matches = app::build_app().get_matches();
    let input = Input::from_file(matches.value_of("input").unwrap());
    let config = Config::from_file(matches.value_of("config").unwrap());

    let sol = generate_solution(
        fetch_availability_with_api,
        &input,
        &config
    );

    if !sol.solved {
        println!("{:?}", sol);
        eprintln!("Cannot find meetings to solve the constraints!");
        process::exit(1);
    }

    println!("{:?}", sol);
    match matches.occurrences_of("book") {
        0 => println!("Dry run mode, not booking!"),
        _ => {
            println!("Booking!");
            book_with_api(&sol);
        }
    }
}


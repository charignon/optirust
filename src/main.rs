/* Project Optirust

- TODO Use LP
- TODO Fix lunch intersection
- TODO Make it possible to accept day long event
- TODO Make constraints configurable
- TODO Factor out part that people might want to override
- TODO Fix Warning

 */
extern crate bio;
extern crate chrono;
extern crate chrono_tz;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate clap;
extern crate serde_yaml;
extern crate yaml_rust;
extern crate google_calendar3 as calendar3;
extern crate yup_oauth2 as oauth2;
extern crate hyper;
extern crate hyper_rustls;
extern crate rayon;
use std::collections::HashMap;
use std::collections::HashSet;
use std::iter::FromIterator;
use std::process;
use std::vec::Vec;
use std::ops::Fn;
use rayon::prelude::*;

mod fixtures;
mod app;
mod gcal;
mod gen;
mod types;

use fixtures::{test_input, test_config};
use types::{Input, Config, DesiredMeeting, MeetingsTree,
            RoomConfig, MeetingCandidate, Solution};

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
    let mut score = 0;

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

// Generate candidate meetings for a desired meeting
// candidate meetings have a score
fn generate_meeting_candidates(
    tm: &DesiredMeeting,
    avail: &HashMap<String, MeetingsTree>,
    config: &Config) -> Vec<MeetingCandidate> {
    let mut candidates:Vec<MeetingCandidate> = Vec::new();
    let intervals = gen::generate_all_possible_meetings(tm);
    let possible_rooms:Vec<String>;
    if tm.attendees.len() <= 2 {
        possible_rooms = rooms_vec_to_emails(&config.small_rooms);
    } else {
        possible_rooms = rooms_vec_to_emails(&config.large_rooms);
    }

    let mandatory_attendees = &tm.attendees;

    // Each interval will lead to a desired meeting if at least a room is available
    // and the attendees are all available as well
    for i in &intervals {
        // Can all the attendees make it?

        let conflicts:usize = mandatory_attendees
            .iter()
            .map(|k| avail[k].find(i.start..i.end).count())
            .sum();


        if conflicts != 0 {
            continue;
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
            continue;
        }

        // Create a suitable candidate
        candidates.push(MeetingCandidate{
            id: "foobar".to_string(),
            start: i.start,
            end: i.end,
            room: suitable_room.unwrap(),
            score: compute_score(&i.start, &i.end, &mandatory_attendees, &avail)
        });
    }
    candidates
}

fn generate_solution<F>(
    fetch: F,
    input: &Input,
    config: &Config
) -> Solution
where F: Fn(Vec<String>) -> HashMap<String, MeetingsTree> {

    // Solution
    let mut picked_candidates:HashMap<DesiredMeeting, MeetingCandidate> = HashMap::new();

    // Fetch the availability for all the relevant attendees
    let mut availability_for_all_attendees:HashMap<String, MeetingsTree> = fetch(extract_attendees(input, config));

    // For each meeting figure out the best option
    for it in &input.meetings {
        // Score them
        let mut candidates = generate_meeting_candidates(it, &availability_for_all_attendees, &config);
        candidates.sort_by(|a,b| a.score.cmp(&b.score));
        println!("{:?}", candidates);
        if let Some(c) = candidates.pop() {
            picked_candidates.insert(it.clone(), c.clone());
            // Update availability
            for a in &it.attendees {
                let mut new_tree = availability_for_all_attendees[a].clone();
                new_tree.insert(c.start.clone()..c.end.clone(), c.id.clone());
                availability_for_all_attendees.insert(a.to_string(), new_tree);
            };
        } else {
            return Solution {solved:false, candidates: HashMap::new()};
        }
    }
    Solution{solved: true, candidates: picked_candidates}
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


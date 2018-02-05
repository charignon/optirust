/* Project Optirust

- TODO Fix lunch intersection
- TODO Make it possible to accept day long event
- TODO Fallback to other solver if cbc not available
- TODO Relax constraint on step / duration and all meetings same start and end range
- TODO Make constraints configurable
- TODO Factor out part that people might want to override

 */
extern crate bio;
extern crate chrono;
extern crate chrono_tz;
#[macro_use]
extern crate clap;
extern crate serde_yaml;
#[macro_use]
extern crate serde_derive;
extern crate yaml_rust;
extern crate google_calendar3 as calendar3;
extern crate yup_oauth2 as oauth2;
extern crate hyper;
extern crate hyper_rustls;
extern crate rayon;
use std::io::prelude::*;
use std::fs::File;
use std::collections::HashMap;
use std::collections::HashSet;
use std::iter::FromIterator;
use std::process;
use std::vec::Vec;
use std::ops::Fn;
use std::process::Command;

mod fixtures;
mod app;
mod gcal;
mod gen;
mod types;

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
#[derive(Debug, Serialize)]
struct SolverInput {
    intersections: Vec<Vec<String>>,
    candidates_scores: HashMap<String, usize>,
    candidate_per_desired_meeting: HashMap<String, Vec<String>>,
}

impl SolverInput {
    fn new() -> SolverInput {
        SolverInput {
            intersections: Vec::new(),
            candidates_scores: HashMap::new(),
            candidate_per_desired_meeting: HashMap::new(),
        }
    }
}

fn to_solver_fmt(s: &SolverInput) -> String {
    // Generate the optimal function from the scores
    let objective_string = format!("Maximize\nobj: {}\n", {
        s.candidates_scores
            .iter()
            .map(|it| format!("{} {}", it.1, it.0 ))
            .collect::<Vec<String>>()
            .join(" + ")
    });

    let variables_string = format!("Binary\n{}\nEnd", {
        s.candidates_scores
            .iter()
            .map(|it| format!("{}", it.0 ))
            .collect::<Vec<String>>()
            .join(" ")
    });

    // Generate the constraint for each meeting should happen
    let one_candidate_per_meeting_constraint_string = {
        s.candidate_per_desired_meeting
            .iter()
            .map(|it| format!("{} = 1", it.1.join(" + ")))
            .collect::<Vec<String>>()
            .join("\n")
    };

    let intersection_constraint_string = {
        s.intersections
            .iter()
            .map(|it| format!("{} <= 1", it.join(" + ")))
            .collect::<Vec<String>>()
            .join("\n")
    };

    format!("{}\n Subject To\n {} \n {} \n {}",
            objective_string,
            one_candidate_per_meeting_constraint_string,
            intersection_constraint_string,
            variables_string
    )
}

fn read_res(candidates: &HashMap<String, MeetingCandidate>,
            desired_meetings: &Vec<DesiredMeeting>
) -> Option<HashMap<DesiredMeeting, MeetingCandidate>> {
    let mut res: HashMap<DesiredMeeting, MeetingCandidate> = HashMap::new();
    // If not optimal return None
    let mut input = File::open("solution.sol").expect("file not found");
    let mut contents = String::new();
    input.read_to_string(&mut contents)
        .expect("something went wrong reading the file");
    let mut lines = contents.lines();
    let first_line = lines.next().unwrap();
    if !first_line.contains("Optimal") {
        return None;
    }
    let k:f32 = first_line.split_whitespace().collect::<Vec<&str>>().last().unwrap().parse().unwrap();
    let score = - k;
    println!("Total score is {}", score);

    for l in lines {
        let words:Vec<&str> = l.split_whitespace().collect();
        let ident = words[1];
        let candidate = candidates.get(ident).unwrap();
        let desired_meeting = desired_meetings
            .iter()
            .find(|k| k.title == candidate.title)
            .unwrap();

        res.insert(desired_meeting.clone(), candidate.clone());
    }
    return Some(res);
}

fn generate_solution<F>(
    fetch: F,
    input: &Input,
    config: &Config
) -> Solution
where F: Fn(Vec<String>) -> HashMap<String, MeetingsTree> {

    let avail:HashMap<String, MeetingsTree> = fetch(extract_attendees(input, config));
    let intervals = gen::generate_all_possible_meetings(&input.meetings[0]);
    let mut solver_input = SolverInput::new();
    let mut id = 0;
    let mut candidates:HashMap<String, MeetingCandidate> = HashMap::new();

    // This assumes that all the intervals are the same for all the meetings
    // and step <= duration
    // TODO Fix this assumption!
    for i in &intervals {
        let mut intersection:Vec<String> = Vec::new();
        for it in &input.meetings {
            let ident = format!("id{}", id);
            if let Some(m) = generate_meeting_candidate(it, &avail, &config, ident.clone() , &i) {
                solver_input.candidates_scores.insert(ident.to_string(), m.score);
                candidates.insert(ident.to_string(), m);
                intersection.push(ident.to_string());
                id += 1;
                solver_input.candidate_per_desired_meeting.entry(it.title.to_string()).or_insert(Vec::new()).push(ident.to_string());
            }
        }
        if intersection.len() > 1 {
            solver_input.intersections.push(intersection);
        }
    }

    let mut buffer = File::create("temp.lp").unwrap();
    buffer.write(to_solver_fmt(&solver_input).as_bytes()).expect("Cannot write to disk!");

    Command::new("cbc")
        .args(&["temp.lp", "solve", "solution", "solution.sol"])
        .output()
        .expect("failed to execute process, make sure cbc is in the path");

    // Expand the maximize function, score * candidate for all the candidates
    match read_res(&candidates, &input.meetings) {
        Some(m) => Solution{solved: true, candidates: m},
        None => Solution{solved: false, candidates: HashMap::new()}
    }
}

fn main() {
    let matches = app::build_app().get_matches();
    let input = Input::from_file(matches.value_of("input").unwrap());
    let config = Config::from_file(matches.value_of("config").unwrap());

    //if !all_identical_time_frame(&input.meetings) {
    if false {
        eprintln!("Multiple time frame detected, call the program once for each time frame");
        process::exit(1);
    }

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


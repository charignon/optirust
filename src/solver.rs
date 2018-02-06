use std::collections::HashMap;
use types::{DesiredMeeting, MeetingCandidate};
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;

#[derive(Debug)]
pub struct SolverInput {
    pub intersections: Vec<Vec<String>>,
    pub candidate_per_desired_meeting: HashMap<String, Vec<String>>,
    pub candidates: HashMap<String, MeetingCandidate>,
    pub desired_meetings: Vec<DesiredMeeting>,
}

pub fn solve(s: &SolverInput) -> Option<HashMap<DesiredMeeting, MeetingCandidate>> {
    let mut buffer = File::create("temp.lp").unwrap();
    buffer.write(to_solver_fmt(&s).as_bytes()).expect("Cannot write to disk!");

    Command::new("cbc")
        .args(&["temp.lp", "solve", "solution", "solution.sol"])
        .output()
        .expect("failed to execute process, make sure cbc is in the path");

    read_res("solution.sol", &s)
}

impl SolverInput {
    pub fn new() -> SolverInput {
        SolverInput {
            intersections: Vec::new(),
            candidate_per_desired_meeting: HashMap::new(),
            candidates: HashMap::new(),
            desired_meetings: Vec::new(),
        }
    }
}

fn to_solver_fmt(s: &SolverInput) -> String {
    let objective_string = format!("Maximize\nobj: {}\n", {
        s.candidates
            .iter()
            .map(|it| format!("{} {}", it.1.score, it.0 ))
            .collect::<Vec<String>>()
            .join(" + ")
    });

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

    let variables_string = format!("Binary\n{}\nEnd", {
        s.candidates
            .iter()
            .map(|it| format!("{}", it.0 ))
            .collect::<Vec<String>>()
            .join(" ")
    });

    format!("{}\n Subject To\n {} \n {} \n {}",
            objective_string,
            one_candidate_per_meeting_constraint_string,
            intersection_constraint_string,
            variables_string
    )
}

fn read_res(
    cbc_solver_result_filename: &str,
    solver_input: &SolverInput,
) -> Option<HashMap<DesiredMeeting, MeetingCandidate>> {

    let mut input = File::open(cbc_solver_result_filename).expect("file not found");
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

    let mut res: HashMap<DesiredMeeting, MeetingCandidate> = HashMap::new();
    for l in lines {
        let words:Vec<&str> = l.split_whitespace().collect();
        let ident = words[1];
        let val = words[2];
        if val == "1" {
            let candidate = solver_input.candidates.get(ident).unwrap();
            let desired_meeting = solver_input.desired_meetings
                .iter()
                .find(|k| k.title == candidate.title)
                .unwrap();

            res.insert(desired_meeting.clone(), candidate.clone());
        }
    }
    return Some(res);
}


use std::collections::HashMap;
use types::{DesiredMeeting, MeetingCandidate};
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;
use fixtures;

#[derive(Debug)]
pub struct SolverInput {
    pub intersections: Vec<Vec<String>>,
    pub candidate_per_desired_meeting: HashMap<String, Vec<String>>,
    pub candidates: HashMap<String, MeetingCandidate>,
    pub desired_meetings: Vec<DesiredMeeting>,
}

pub fn solve_with_cbc_solver(s: &SolverInput) -> Option<HashMap<DesiredMeeting, MeetingCandidate>> {
    let mut buffer = File::create("temp.lp").unwrap();
    buffer.write(s.to_lp_fmt().as_bytes()).expect("Cannot write to disk!");

    Command::new("cbc")
        .args(&["temp.lp", "solve", "solution", "solution.sol"])
        .output()
        .expect("failed to execute process, make sure 'cbc' is in yout path");

    let mut input = File::open("solution.sol").expect("file not found");
    let mut contents = String::new();
    input.read_to_string(&mut contents)
        .expect("something went wrong reading the file");

    read_cbc_solver_solution(&contents, &s)
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

    fn to_lp_fmt(&self) -> String {
        let objective_string = format!("  obj: {}", {
            let mut k = self.candidates
                .iter()
                .map(|it| format!("{} {}", it.1.score, it.0 ))
                .collect::<Vec<String>>();
            k.sort();
            k.join(" + ")
        });

        let one_candidate_per_meeting_constraints = {
            self.candidate_per_desired_meeting
                .iter()
                .map(|it| format!("  {} = 1", it.1.join(" + ")))
                .collect::<Vec<String>>()
        };

        let intersection_constraints = {
            self.intersections
                .iter()
                .map(|it| format!("  {} <= 1", it.join(" + ")))
                .collect::<Vec<String>>()
        };

        let variables_string = {
            let mut k = self.candidates
                .iter()
                .map(|it| format!("{}", it.0 ))
                .collect::<Vec<String>>();
            k.sort();
            format!("  {}", k.join(" "))
        };
        let mut constraints = one_candidate_per_meeting_constraints;
        constraints.extend(intersection_constraints);
        constraints.sort();

        format!("Maximize\n{}\nSubject To\n{}\nBinary\n{}\nEnd",
                objective_string,
                constraints.join("\n"),
                variables_string
        )
    }
}

#[test]
fn test_to_lp_fmt() {
    let mut input = SolverInput::new();
    let candidate_a =  fixtures::sample_candidate_a();
    let candidate_b =  fixtures::sample_candidate_b();
    let desired_meetings = fixtures::test_desired_meetings() ;

    input.candidates.insert("id10873".to_string(), candidate_a.clone());
    input.candidates.insert("id0".to_string(), candidate_b.clone());
    input.desired_meetings.extend(desired_meetings.clone());
    input.candidate_per_desired_meeting.insert("title".to_string(), vec!["id10873".to_string()]);
    input.candidate_per_desired_meeting.insert("title2".to_string(), vec!["id0".to_string()]);

    assert_eq!(
        "Maximize
  obj: 23 id0 + 23 id10873
Subject To
  id0 = 1
  id10873 = 1
Binary
  id0 id10873
End",
        input.to_lp_fmt()
    );

}

fn read_cbc_solver_solution(
    solution: &str,
    solver_input: &SolverInput,
) -> Option<HashMap<DesiredMeeting, MeetingCandidate>> {

    let mut lines = solution.lines();
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

#[test]
fn test_read_cbc_solver_solution_with_good_solver_input() {
    let mut input = SolverInput::new();
    let candidate_a =  fixtures::sample_candidate_a();
    let candidate_b =  fixtures::sample_candidate_b();
    let desired_meetings = fixtures::test_desired_meetings() ;

    input.candidates.insert("id10873".to_string(), candidate_a.clone());
    input.candidates.insert("id0".to_string(), candidate_b.clone());
    input.desired_meetings.extend(desired_meetings.clone());

    let mut expected_output = HashMap::new();
    expected_output.insert(desired_meetings[0].clone(), candidate_a);
    expected_output.insert(desired_meetings[1].clone(), candidate_b);

    assert_eq!(
        Some(expected_output),
        read_cbc_solver_solution(&fixtures::sample_cbc_solution(), &input)
    );

}


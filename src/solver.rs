use std::collections::HashMap;
use std::fs::File;
use std::collections::HashSet;
use std::io::prelude::*;
use std::iter::FromIterator;
use std::process::Command;
use chrono;
use chrono::prelude::*;
use fixtures;
use gen;
use bio::data_structures::interval_tree::IntervalTree;
use types::{DesiredMeeting, MeetingsTree,
            MeetingCandidate, Meeting,
            RoomPickerFnType, ScoringFnType, Options};


#[derive(Debug)]
pub struct SolverInput {
    pub intersections: HashSet<Vec<String>>,
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

fn build_intersections_pairs(
    candidates: &HashMap<String, MeetingCandidate>,
) -> HashSet<Vec<String>> {
    let mut tree:IntervalTree<DateTime<chrono::Utc>, String> = IntervalTree::new();
    for it in candidates {
        let c = it.1;
        tree.insert(c.start..c.end, c.id.to_string());
    }

    let mut intersections: HashSet<Vec<String>> = HashSet::new();
    for c in candidates {
        let ref ident = c.0;
        let range = c.1.start..c.1.end;
        for k in tree.find(range).map(|r| r.data()).filter(|k| k != ident) {
            let mut combined = vec![k.to_string(), ident.to_string()];
            combined.sort();
            intersections.insert(combined);
        }
    }
    intersections
}

impl SolverInput {
    pub fn new() -> SolverInput {
        SolverInput {
            intersections: HashSet::new(),
            candidate_per_desired_meeting: HashMap::new(),
            candidates: HashMap::new(),
            desired_meetings: Vec::new(),
        }
    }

    // TODO Test this
    pub fn new_from_desired_meetings_and_opts(desired_meetings: Vec<DesiredMeeting>,
                                              opts: &Options) -> SolverInput {
        let mut solver_input = SolverInput::new();
        solver_input.desired_meetings = desired_meetings.clone();
        let avail:HashMap<String, MeetingsTree> = (opts.fetch_fn)(extract_attendees(&desired_meetings, &opts.room_picker_fn));
        for me in desired_meetings {
            for interval in gen::generate_all_possible_meetings(&me, &opts.reject_date_fn, &opts.reject_datetime_fn) {
                if let Some(m) = generate_meeting_candidate(&me, &avail, interval.id.to_string(), &opts.room_picker_fn, &opts.scoring_fn, &interval) {
                    solver_input
                        .candidates
                        .insert(interval.id.to_string(), m);
                    solver_input.
                        candidate_per_desired_meeting
                        .entry(me.title.to_string())
                        .or_insert(Vec::new())
                        .push(interval.id.to_string());
                }
            }
        }
        solver_input.intersections = build_intersections_pairs(&solver_input.candidates);
        solver_input
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


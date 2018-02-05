use std::hash::{Hash, Hasher};
use std::collections::HashMap;
use std::io::prelude::*;
use std::fs::File;

use bio::data_structures::interval_tree::IntervalTree;
use chrono::prelude::*;
use chrono;
use chrono_tz::Tz;
use yaml_rust;
use fixtures::{test_input, test_config};

// The input to the program containing what meetings the user
// wants to schedule
pub struct Input {
    pub meetings: Vec<DesiredMeeting>
}

impl Input {
    pub fn from_yaml_str(s: &str) -> Input{
        let docs = yaml_rust::YamlLoader::load_from_str(s).unwrap();
        Input::from_yaml(&docs[0])
    }

    pub fn from_file(file: &str) -> Input {
        let mut input = File::open(file).expect("file not found");
        let mut contents = String::new();
        input.read_to_string(&mut contents)
            .expect("something went wrong reading the file");
        Input::from_yaml_str(&contents)
    }
}
#[test]
fn can_build_input() {
    let a = Input::from_yaml_str(&test_input());
    assert_eq!(a.meetings[0].title, "title");
    assert_eq!(a.meetings[0].description, "description");
    assert_eq!(a.meetings[0].attendees[0], "laurent.charignon@foo.com");
    assert_eq!(a.meetings[0].timezone.name(), "America/Los_Angeles");
    // 10 PST -> 18 UTC
    // 18 PST -> 2 UTC
    assert_eq!(a.meetings[0].min_date.hour(), 18);
    assert_eq!(a.meetings[0].max_date.hour(), 2);
}


#[derive(Eq, PartialEq, Clone, Debug)]
pub struct DesiredMeeting {
    pub title: String,
    pub description: String,
    pub attendees: Vec<String>, // email
    pub min_date: DateTime<chrono::Utc>, // Parse as a date
    pub max_date: DateTime<chrono::Utc>, // Parse as a date
    pub step: chrono::Duration,
    pub duration: chrono::Duration,
    pub timezone: Tz,
}

impl Hash for DesiredMeeting {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.title.hash(state);
        self.description.hash(state);
    }
}

// The config file format describes the small and large
// rooms available to book, small means 2 people or less
// large 3+ people
pub struct Config {
    pub small_rooms: Vec<RoomConfig>,
    pub large_rooms: Vec<RoomConfig>
}

impl Config {
    pub fn from_yaml_str(s: &str) -> Config{
        let docs = yaml_rust::YamlLoader::load_from_str(s).unwrap();
        Config::from_yaml(&docs[0])
    }

    pub fn from_file(file: &str) -> Config {
        let mut config = File::open(file).expect("file not found");
        let mut contents = String::new();
        config.read_to_string(&mut contents)
            .expect("something went wrong reading the file");
        Config::from_yaml_str(&contents)
    }
}
#[test]
fn can_build_config() {
    let a = Config::from_yaml_str(&test_config());
    assert_eq!(a.small_rooms[0].name, "Foo");
    assert_eq!(a.large_rooms[0].email, "bozorg@jam.com")
}

#[allow(dead_code)]
pub struct RoomConfig {
    pub name: String,
    pub email: String,
}

// A potential meeting, linked to a desired meeting
#[derive(Clone)]
pub struct MeetingCandidate {
    pub title: String,
    pub id: String,
    pub start:DateTime<chrono::Utc>,
    pub end: DateTime<chrono::Utc>,
    pub room: String,
    pub score: usize,
}

impl MeetingCandidate {
    #[allow(dead_code)]
    pub fn intersects(&self, o: &MeetingCandidate) -> bool {
        return (self.start < o.end) && (o.start < self.end)
    }
}

use std::fmt;
impl fmt::Debug for MeetingCandidate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Candidate {:?} {{ start_time: {:?}, end_time: {:?}, room: {:?}, score: {:?} }}",
               self.id,
               self.start.with_timezone(&chrono::Local).to_string(),
               self.end.with_timezone(&chrono::Local).to_string(),
               self.room,
               self.score)

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
    pub start:DateTime<chrono::Utc>,
    pub end: DateTime<chrono::Utc>
}
pub type MeetingsTree = IntervalTree<DateTime<chrono::Utc>, String>;

pub struct Solution {
    pub solved: bool,
    pub candidates: HashMap<DesiredMeeting, MeetingCandidate>,
}

// We use YAML for all the format, each of the pub struct can implement the
// YamlParsable trait for easy deserialization
pub trait YamlParsable {
    // Given a Yaml mapping to a pub struct, parses and return an instance of it
    fn from_yaml(s:&yaml_rust::Yaml) -> Self;
}

fn parse_list_of<T: YamlParsable>(s: &yaml_rust::Yaml) -> Vec<T> {
    s.as_vec().unwrap().into_iter().map(|x| T::from_yaml(x)).collect()
}

impl YamlParsable for RoomConfig {
    fn from_yaml(s:&yaml_rust::Yaml) -> RoomConfig {
        return RoomConfig{
            name: s["name"].as_str().unwrap().to_string(),
            email: s["email"].as_str().unwrap().to_string()
        }
    }
}

impl YamlParsable for Config {
    fn from_yaml(s:&yaml_rust::Yaml) -> Config {
        return Config {
            small_rooms: parse_list_of(&s["rooms"]["small"]),
            large_rooms: parse_list_of(&s["rooms"]["large"])
        }
    }
}

impl YamlParsable for DesiredMeeting {
    fn from_yaml(s:&yaml_rust::Yaml) -> DesiredMeeting {
        let timezone = s["timezone"].as_str().unwrap_or("America/Los_Angeles").to_string();
        let tz: Tz = timezone.parse().unwrap();
        let min_d = tz.datetime_from_str(s["min_date"].as_str().unwrap(), "%Y-%m-%d %H:%M:%S");
        let max_d = tz.datetime_from_str(s["max_date"].as_str().unwrap(), "%Y-%m-%d %H:%M:%S");
        let duration = s["duration"].as_i64().unwrap_or(30);
        let step = s["step"].as_i64().unwrap_or(30);

        DesiredMeeting {
            title: s["title"].as_str().unwrap().to_string(),
            description: s["description"].as_str().unwrap().to_string(),
            attendees: s["attendees"].as_vec().unwrap().into_iter().map(
                |x| x.as_str().unwrap().to_string()
            ).collect(),
            step: chrono::Duration::minutes(step),
            duration: chrono::Duration::minutes(duration),
            min_date: min_d.expect("Cannot convert min date").with_timezone(&chrono::Utc),
            max_date: max_d.expect("Cannot convert max date").with_timezone(&chrono::Utc),
            timezone: tz,
        }
    }
}

impl YamlParsable for Input {
    fn from_yaml(s:&yaml_rust::Yaml) -> Input {
        return Input {
            meetings: parse_list_of(&s["meetings"]),
        }
    }

}

impl YamlParsable for Meeting {
    fn from_yaml(s:&yaml_rust::Yaml) -> Meeting {
        let start_time = s["start"]["dateTime"].as_str().unwrap();
        let end_time = s["end"]["dateTime"].as_str().unwrap();

        Meeting {
            id: s["id"].as_str().unwrap().to_string(),
            start: start_time.parse::<DateTime<Utc>>().unwrap(),
            end: end_time.parse::<DateTime<Utc>>().unwrap()
        }
    }
}
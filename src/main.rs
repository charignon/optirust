

/* Project Optirust

Goal: Implement calendar optimized scheduler using rust

What are the features that I want to build
1. we can parse the input file and map to a struct
2. we can generate objects for the attendees and rooms from the struct
3. we can populate those with their calendars
4. we can generate the possible meetings and score them
5. we can generate the equation system with everything and solve it
6. we can display a solution

Constraint: 10 lines per function
Constraint: 100% test coverage
Constraint: Timezone aware
Constraint: as much FP as possible, pluggable I/O and dependency, the middle should be
as isolated from the rest as possible

1. we can parse the input file and map to a struct
==================================================
What is the input?
We need two files, a config file with the paramater and an input file with the
requested meeting.
Let's define a struct for the options and another one for the input.

1.a The config file
===================
- DONE What is it like?
- DONE Can we parse it?

1.b The input file
==================
- TODO What is it like?
- TODO Can we parse it?
 */
extern crate yaml_rust;
use std::vec::Vec;
use yaml_rust::{YamlLoader};

fn parse_list_of<T: YamlParsable>(s: &yaml_rust::Yaml) -> Vec<T> {
    s.as_vec().unwrap().into_iter().map(|x| T::from_yaml(x)).collect() 
}


trait YamlParsable {
    // Given a Yaml mapping to a struct, parses and return an instance of it
   fn from_yaml(s:&yaml_rust::Yaml) -> Self; 
}

struct RoomConfig {
    name: String,
    email: String,
}

impl YamlParsable for RoomConfig {
    fn from_yaml(s:&yaml_rust::Yaml) -> RoomConfig {
        return RoomConfig{
            name: s["name"].as_str().unwrap().to_string(),
            email: s["email"].as_str().unwrap().to_string()
        }
    }
}

struct Config {
    small_rooms: Vec<RoomConfig>,
    large_rooms: Vec<RoomConfig>
}

impl YamlParsable for Config {
    fn from_yaml(s:&yaml_rust::Yaml) -> Config {
        return Config {
            small_rooms: parse_list_of(&s["rooms"]["small"]),
            large_rooms: parse_list_of(&s["rooms"]["large"])
        }
    }
}

fn build_config(s: &str) -> Config{
    let docs = YamlLoader::load_from_str(s).unwrap();
    return Config::from_yaml(&docs[0])
}

#[test]
fn can_build_config() {
    let s = "
rooms:
  small:
    - name: Foo
      email: foo@bar.com
    - name: Bar
      email: joe@baz.com
  large:
    - name: Bozorg
      email: bozorg@jam.com
";
    let a = build_config(s);
    assert_eq!(a.small_rooms[0].name, "Foo");
    assert_eq!(a.large_rooms[0].email, "bozorg@jam.com")
}









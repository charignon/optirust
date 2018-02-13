/* Project Optirust

- TODO Make it possible to accept day long event
- TODO Make sure the code works when no room specified

- TODO Make constraints configurable using a config file
- TODO Add example files in a separate folder
- TODO Test usability on a new host
- TODO Log level
- TODO Add to the description the powered by optirust tagline
*/
extern crate bio;
extern crate chrono;
extern crate chrono_tz;
#[macro_use]
extern crate clap;
extern crate google_calendar3 as calendar3;
extern crate hyper;
extern crate hyper_rustls;
extern crate rayon;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate yup_oauth2 as oauth2;

use std::collections::HashMap;
use std::process;

mod app;
mod fixtures;
mod gcal;
mod gen;
mod solver;
mod types;

use types::{Config, Solution};

fn main() {
    let matches = app::build_app().get_matches();
    let options = {
        let config = Config::from_file(
            matches
                .value_of("config")
                .expect("Please give a valid config file"),
        );
        types::Options {
            room_picker_fn: Box::new(move |k| config.room_picker(k)),
            ..Default::default()
        }
    };

    let input = types::read_input(
        matches
            .value_of("input")
            .expect("Please give a valid input file"),
    );

    let solver_input = solver::SolverInput::new_from_desired_meetings_and_opts(input, &options);

    let sol = match (options.solver_fn)(&solver_input) {
        Some(m) => Solution {
            solved: true,
            candidates: m,
        },
        None => Solution {
            solved: false,
            candidates: HashMap::new(),
        },
    };

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

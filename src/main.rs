/* Project Optirust
- TODO Test usability on a new host (30')
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
        let config_filename = matches.value_of("config");
        if let Some(config_filename) = config_filename {
            let config = Config::from_file(config_filename);
            let (c, d, e) = (config.clone(), config.clone(), config.clone());
            let room_picker = Box::new(move |k| c.room_picker(k));
            let reject_date = Box::new(move |k| d.reject_date_fn(k));
            let reject_datetime = Box::new(move |k, l| e.reject_datetime_fn(k, l));
            types::Options {
                room_picker_fn: room_picker,
                reject_date_fn: reject_date,
                reject_datetime_fn: reject_datetime,
                ignore_all_day_events: config.ignore_all_day_events,
                ignore_meetings_with_no_response: config.ignore_meetings_with_no_response,
                ..Default::default()
            }
        } else {
            types::Options {
                ..Default::default()
            }
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
            gcal::book_with_api(&sol, true);
        }
    }
}

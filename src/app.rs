use clap::{App, Arg};

pub fn build_app() -> App<'static, 'static> {
    App::new("optirust")
        .version(crate_version!())
        .author("Laurent Charignon <l.charignon@gmail.com>")
        .about("Calendar schedule optimizer")
        .usage("optirust <inputfile> <configfile>")
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .takes_value(true)
                .help("Sets the input file to use")
                .required(true),
        )
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .takes_value(true)
                .help("Sets the config file to use")
                .required(true),
        )
        .arg(
            Arg::with_name("book")
                .short("b")
                .long("book")
                .help("Really book"),
        )
}

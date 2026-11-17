use std::path::PathBuf;

use clap::{Arg, ArgAction, command, value_parser};

use nullfs::{NullFS, parse_options};

fn main() -> std::io::Result<()> {
    let matches = command!()
        .arg(
            Arg::new("MOUNT")
                .help("path to the mounting point")
                .index(1)
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("OPTION")
                .help("mount options")
                .short('o')
                .long("option")
                .action(ArgAction::Append),
        )
        .get_matches();

    let path = matches
        .get_one::<PathBuf>("MOUNT")
        .expect("MOUNT is a required argument");

    let config = parse_options(
        matches
            .get_many::<String>("OPTION")
            .unwrap_or_default()
            .map(String::as_str),
    );

    fuser::mount(NullFS, path, &config)
}

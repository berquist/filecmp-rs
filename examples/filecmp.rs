//! Example for demo() in filecmp.py

use clap::{App, Arg};
use filecmp::DirCmp;

fn main() {
    let matches = App::new("filecmp")
        .version("0.1.0")
        .author("owtotwo <owtotwo@163.com>")
        .about("A filecmp tool like filecmp.py in python3 standard library.")
        .arg(
            Arg::with_name("recur")
                .short("r")
                .long("recur")
                .multiple(false)
                .takes_value(false)
                .required(false)
                .help("Compare file in folder recursively"),
        )
        .arg(
            Arg::with_name("folder_a")
                .value_name("FOLDER_A")
                .help("One folder you want to compare")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("folder_b")
                .value_name("FOLDER_B")
                .help("Another folder you want to compare")
                .required(true)
                .index(2),
        )
        .get_matches();

    let is_recur = matches.is_present("recur");
    let a = matches.value_of("folder_a").unwrap();
    let b = matches.value_of("folder_b").unwrap();

    let dd = DirCmp::new(a, b, None, None);
    if is_recur {
        dd.report_full_closure();
    } else {
        dd.report();
    }
}

use std::process::exit;

use clap::{CommandFactory, Parser};
use dds::{print_completions, single, threaded, Dds};
use human_panic::setup_panic;

fn main() {
    setup_panic!();

    let opt = Dds::parse();

    if let Some(shell) = opt.generate {
        let mut cmd = Dds::command();
        print_completions(shell, &mut cmd);
        exit(0);
    }

    println!("Are you sure you want to overwrite {}? (y/n)", &opt.output);
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    if input.trim() != "y" {
        eprintln!("Aborting");
        exit(1);
    }

    if opt.threaded {
        threaded::controller(opt).unwrap();
    } else {
        single::controller(opt).unwrap();
    }
}

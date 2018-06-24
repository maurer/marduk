// Macros need to be loaded at root
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod eval_common;
mod stats;
use eval_common::*;

pub const MEMORY_LIMIT: usize = 1024 * 1024 * 1024 * 100; // 100G
pub const TIME_LIMIT: u64 = 60 * 60; // 1 hr

fn log_measure(m: &Measurement) {
    // We won't be logging that many results so just opening the file again every time is fine.
    // This also gives us aggressive flushing, which should be good in our scenario
    use std::fs::OpenOptions;
    use std::io::Write;
    let log_open = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .clone();
    {
        let mut debug_fd = log_open.open("debug.log").unwrap();
        // Technically this could get befuddled by binaries named MEASURE or END_MEASURE, but none
        // of those in my dataset are, and this isn't meant to be robust serialization, more like
        // recovery.
        writeln!(&mut debug_fd, "MEASURE\n{:?}\nEND_MEASURE", m).unwrap();
    }
    {
        let mut display_fd = log_open.open("human.log").unwrap();
        writeln!(&mut display_fd, "{}", m).unwrap();
    }
}

const ALIAS_MODES: &[Config] = &[
    Config::CONTEXT_INSENSITIVE,
    Config::CONTEXT_SENSITIVE,
];

fn measure_uaf(name: &str) -> Vec<Measurement> {
    let names = vec![name.to_string()];
    ALIAS_MODES
        .iter()
        .flat_map(|mode| {
            let ms = measure_mode(&names, mode, &[]);
            if let Some(ref m) = ms {
                log_measure(&m);
            }
            ms
        })
        .collect()
}

fn main() {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    let spec_path = {
        let mut args = ::std::env::args();
        args.next().unwrap(); // Skip own name
        args.next().unwrap() // First argument, assumed specfile
    };
    let spec_fd = File::open(spec_path).unwrap();
    let spec_reader = BufReader::new(spec_fd);
    let mut full = Vec::new();
    for bin_r in spec_reader.lines() {
        full.extend(measure_uaf(&bin_r.unwrap()));
    }
    {
        let mut out = File::create("out.json").unwrap();
        serde_json::to_writer(&mut out, &full).unwrap();
    }
    stats::post_analysis(full);
}

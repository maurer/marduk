// Macros need to be loaded at root
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod eval_common;
mod stats;
use eval_common::*;

pub const MEMORY_LIMIT: usize = 0;
pub const TIME_LIMIT: u64 = 0;

const ALIAS_MODES: &[Config] = &[Config::CONTEXT_INSENSITIVE, Config::CONTEXT_SENSITIVE];

fn main() {
    use std::fs::File;
    let mut in_file = File::open("out.json").unwrap();
    let dat: Vec<Measurement> = serde_json::from_reader(&mut in_file).unwrap();
    stats::post_analysis(dat);
}

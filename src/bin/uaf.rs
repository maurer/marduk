extern crate env_logger;
extern crate marduk;

use std::time::{Duration, Instant};

fn print_state(db: &mut marduk::datalog::Database) {
    println!("Steens:");
    for x in db.query_uaf() {
        println!("{}", x);
    }

    println!("Flow:");
    for x in db.query_uaf_flow() {
        println!("{}", x);
    }
}

fn main() {
    env_logger::init();
    let mut db = marduk::uaf(&::std::env::args().collect::<Vec<_>>()[1..], true);
    let mut step = 0;
    let mut last_round = Vec::new();
    println!("Booting");
    let total = Instant::now();
    let timeout = Duration::new(60 * 60, 0); // 1 hr timeout
    while (!last_round.is_empty() || step == 0) && step < 10000 && total.elapsed() < timeout {
        let mark = Instant::now();
        last_round = db.run_rules_once();
        step += 1;
        println!(
            "Step {} complete, took {:?}, {:?} total",
            step,
            mark.elapsed(),
            total.elapsed()
        );
    }
    print_state(&mut db);
    if !last_round.is_empty() {
        println!("Run did not complete.");
    }
}

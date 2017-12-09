extern crate marduk;

use std::time::{Duration, Instant};

fn print_state(db: &marduk::datalog::Database) {
    println!("---\nflow");
    for f in db.query_flow() {
        println!("{}", f);
    }
    println!("---\nuaf_flow");
    for q in db.query_get_uaf_flow_full() {
        println!("{}", q);
    }
    println!("---\ntrace");
    for t in db.query_trace() {
        println!("{}", t)
    }
    println!("---\nuaf");
    for q in db.query_get_uaf_full() {
        println!("{}", q);
    }
    println!("Defines generated: {}", db.query_defines().len());
    println!("Reaches generated: {}", db.query_reaching().len());
}

fn main() {
    let mut db = marduk::uaf(&::std::env::args().collect::<Vec<_>>()[1..]);
    let mut step = 0;
    let mut last_round = Vec::new();
    println!("Booting");
    let total = Instant::now();
    let timeout = Duration::new(60 * 60, 0); // 1 hr timeout
    while (!last_round.is_empty() || step == 0) && step < 1000 && total.elapsed() < timeout {
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
    print_state(&db);
    let derivs: Vec<_> = last_round.into_iter().map(|d| db.derivation(&d)).collect();
    println!("Last round derivs (if empty, program terminated):");
    for deriv in derivs {
        println!("{}", deriv);
    }
}

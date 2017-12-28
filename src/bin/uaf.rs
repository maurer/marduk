extern crate marduk;

use std::time::Instant;

fn print_state(db: &marduk::datalog::Database) {
    println!("---\nuaf");
    for q in db.query_get_uaf_full() {
        println!("{}", q);
    }
}


fn main() {
    let mut db = marduk::uaf(&::std::env::args().collect::<Vec<_>>()[1..]);
    let mut step = 0;
    let mut last_round = Vec::new();
    println!("Booting");
    let total = Instant::now();
    while (!last_round.is_empty() || step == 0) && step < 1000 {
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

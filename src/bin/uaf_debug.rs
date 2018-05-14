extern crate env_logger;
extern crate marduk;

use marduk::printers::CB;
use std::time::Instant;

fn print_state(db: &mut marduk::datalog::Database) {
    println!("Steens:");
    for x in db.query_uaf() {
        println!("{}", x);
    }

    println!("Flow:");
    for x in db.query_uaf_flow() {
        println!("{}", x);
    }

    println!("PTS:");
    for x in db.query_flow() {
        println!("{}", x);
    }

    println!("PTS OUT:");
    for x in db.query_flow_out() {
        println!("{}:\n{}", x.loc, x.pts);
    }

    for x in db.query_get_malloc_call() {
        println!("malloc {}", x);
    }

    for x in db.query_succ() {
        if x.src.is_stacked() != x.dst.is_stacked() {
            println!("SUCC BUG");
        }
        println!("succ {} -> {}", x.src, x.dst);
    }

    for x in db.query_func() {
        println!("func {}: {}", x.base, x.contains);
    }

    for x in db.query_call_over() {
        if (x.src.is_stacked() != x.dst.is_stacked()) || (x.dst.is_stacked() != x.func.is_stacked())
        {
            println!("CALL_OVER BUG");
        }
        println!("call_over {}-{}->{}", x.src, x.func, x.dst);
    }

    for x in db.query_call_site() {
        if x.call_loc.is_stacked() != x.target_loc.is_stacked() {
            println!("CALL_SITE BUG");
        }
        println!("call_site {} -> {}", x.call_loc, x.target_loc);
    }

    for x in db.query_constraints() {
        println!("constraints {}:", x.loc);
        let constraint_mode = x.loc.is_stacked();
        for y in x.c {
            if y.iter().any(|z| z.has_stacked() != constraint_mode) {
                println!("CONSTRAINT BUG");
            }
            println!("{}", CB(&y));
        }
    }
}

fn main() {
    env_logger::init();
    let mut db = marduk::uaf(
        &::std::env::args().collect::<Vec<_>>()[1..],
        marduk::AliasMode::Both { ctx: true },
    );
    let mut step = 0;
    let mut last_round = Vec::new();
    println!("Booting");
    let total = Instant::now();
    while !last_round.is_empty() || step == 0 {
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

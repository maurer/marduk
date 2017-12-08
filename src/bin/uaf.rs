extern crate marduk;

use std::time::Instant;

fn print_state(db: &marduk::datalog::Database) {
    //    println!("disas: {}", db.query_get_disasms().len());
    //    println!("alias: {}", db.query_get_alias().len());
    //    println!("---\npad");
    //    for pad in db.query_link_pad() {
    //        println!("{}", pad)
    //    }
    //    println!("---\nsucc");
    //    for s in db.query_succ() {
    //        println!("{}", s)
    //    }
    //    println!("---\nfunc");
    //    for f in db.query_func() {
    //        println!("{}", f)
    //    }
    //
    //    println!("---\ncall_site");
    //    for cs in db.query_call_site() {
    //        println!("{}", cs)
    //    }
    //    println!("---\nalias");
    //    for alias in db.query_get_alias() {
    //        println!("{}", alias)
    //    }
    //println!("---\nmalloc_call");
    //for fc in db.query_get_malloc_call() {
    //    println!("{}", fc)
    //}
    //println!("---\nfree_call");
    //for fc in db.query_get_free_call() {
    //    println!("{}", fc)
    //}
    //println!("---\nlive");
    //for fc in db.query_live() {
    //    println!("{}", fc)
    //}
    println!("---\nuaf");
    for flow in db.query_get_uaf_flow_full() {
        println!("{}", flow)
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

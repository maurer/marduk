extern crate env_logger;
extern crate marduk;

use std::collections::BTreeSet;

fn print_state(db: &mut marduk::datalog::Database) {
    println!("Steens:");
    for x in db.query_uaf() {
        println!("{}", x);
    }

    println!("Flow:");
    for x in db.query_uaf_flow() {
        println!("{}", x);
    }

    println!("Ctx:");
    let mut dedup = BTreeSet::new();
    for x in db.query_context_flow() {
        dedup.insert((x.free.addr, x.use_.addr));
        println!("{}->{}", x.free, x.use_);
    }

    println!("Dedup Ctx:");
    for (free, use_) in dedup {
        println!("0x{:x}->0x{:x}", free, use_)
    }
}

fn main() {
    env_logger::init();
    let mut db = marduk::uaf(
        &::std::env::args().collect::<Vec<_>>()[1..],
        marduk::AliasMode::Both { ctx: true },
    );
    db.run_rules();
    print_state(&mut db);
}

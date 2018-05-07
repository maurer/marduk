extern crate env_logger;
extern crate marduk;

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
    for x in db.query_context_flow() {
        println!("{}->{}", x.free, x.use_);
    }
}

fn main() {
    env_logger::init();
    let mut db = marduk::uaf(
        &::std::env::args().collect::<Vec<_>>()[1..],
        marduk::AliasMode::All,
        true,
    );
    db.run_rules();
    print_state(&mut db);
}

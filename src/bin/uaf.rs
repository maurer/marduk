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
}

fn main() {
    env_logger::init();
    let mut db = marduk::uaf(
        &::std::env::args().collect::<Vec<_>>()[1..],
        marduk::AliasMode::All,
    );
    db.run_rules();
    print_state(&mut db);
}

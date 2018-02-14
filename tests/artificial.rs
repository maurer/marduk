extern crate marduk;
use marduk::uaf;

fn run_uaf(names: &[&'static str], insensitive_bugs: usize, flow_bugs: usize) {
    let names: Vec<_> = names
        .iter()
        .map(|x| format!("samples/artificial/{}", x))
        .collect();
    let mut db = uaf(&names, true);
    db.run_rules();
    let found_insensitive_bugs = db.query_uaf().len();
    if found_insensitive_bugs != insensitive_bugs {
        panic!(
            "Found {} insensitive bugs, expected {}",
            found_insensitive_bugs, insensitive_bugs
        );
    }
    let found_flow_bugs = db.query_uaf_flow().len();
    if found_flow_bugs != flow_bugs {
        panic!(
            "Found {} flow bugs, expected {}",
            found_flow_bugs, flow_bugs
        );
    }
}

#[test]
fn func() {
    run_uaf(&["func"], 1, 1);
}

#[test]
fn link() {
    run_uaf(&["link", "external.so"], 2, 2);
}

#[test]
fn simple() {
    run_uaf(&["simple"], 1, 1);
}

#[test]
fn safe() {
    run_uaf(&["safe"], 0, 0);
}

#[test]
fn path_sensitive() {
    run_uaf(&["path_sensitive"], 1, 0);
}

#[test]
fn remalloc() {
    run_uaf(&["remalloc"], 1, 0);
}

#[test]
fn ll() {
    run_uaf(&["ll"], 3, 0);
}

#[test]
fn loop_() {
    run_uaf(&["loop"], 2, 2);
}

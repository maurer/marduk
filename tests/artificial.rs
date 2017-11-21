extern crate marduk;
use marduk::uaf;

fn run_uaf(names: &[&'static str], flow_bugs: usize) {
    let names: Vec<_> = names
        .iter()
        .map(|x| format!("samples/artificial/{}", x))
        .collect();
    let mut db = uaf(&names);
    db.run_rules();
    assert_eq!(db.query_get_uaf_flow().len(), flow_bugs);
}

#[test]
fn func() {
    run_uaf(&["func"], 1);
}

#[test]
fn link() {
    run_uaf(&["link", "external.so"], 1);
}

#[test]
fn simple() {
    run_uaf(&["simple"], 1);
}

#[test]
fn safe() {
    run_uaf(&["safe"], 0);
}

#[test]
fn path_sensitive() {
    run_uaf(&["path_sensitive"], 0);
}

#[test]
fn remalloc() {
    run_uaf(&["remalloc"], 0);
}

#[test]
fn ll() {
    run_uaf(&["ll"], 0);
}

#[test]
fn loop_() {
    run_uaf(&["loop"], 2);
}

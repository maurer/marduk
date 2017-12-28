extern crate marduk;
use marduk::uaf;

fn run_uaf(names: &[&'static str], flow_bugs: usize, trace_bugs: usize) {
    let names: Vec<_> = names
        .iter()
        .map(|x| format!("samples/artificial/{}", x))
        .collect();
    let mut db = uaf(&names);
    db.run_rules();
    let found_flow_bugs = db.query_get_uaf_flow().len();
    if found_flow_bugs != flow_bugs {
        panic!(
            "Found {} flow bugs, expected {}",
            found_flow_bugs, flow_bugs
        );
    }
    let found_trace_bugs = db.query_get_uaf().len();
    if found_trace_bugs != trace_bugs {
        panic!(
            "Found {} trace bugs, expected {}",
            found_trace_bugs, trace_bugs
        );
    }
}

#[test]
fn func() {
    run_uaf(&["func"], 1, 1);
}

#[test]
fn link() {
    run_uaf(&["link", "external.so"], 1, 1);
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
    run_uaf(&["path_sensitive"], 0, 0);
}

#[test]
fn remalloc() {
    run_uaf(&["remalloc"], 0, 0);
}

#[test]
fn ll() {
    // If flow sensitive is made more conservative (e.g. we track two pointer levels deep)
    // one flow sensitive bug will be detected. However, the trace should still clean it up at that
    // point.
    run_uaf(&["ll"], 1, 0);
}

#[test]
fn loop_() {
    run_uaf(&["loop"], 2, 2);
}

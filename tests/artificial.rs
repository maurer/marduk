extern crate marduk;
use marduk::{uaf, Config};

fn run_uaf(names: &[&'static str], expected_flow_bugs: usize, expected_ctx_bugs: usize) {
    let names: Vec<_> = names
        .iter()
        .map(|x| format!("samples/artificial/{}", x))
        .collect();
    {
        let mut flow_mode = Config::CONTEXT_INSENSITIVE;
        flow_mode.undef_hack = true;
        let mut db = uaf(&names, flow_mode);
        db.run_rules();
        let flow_bugs = db.query_uaf_flow();
        let found_flow_bugs = flow_bugs.len();
        if found_flow_bugs != expected_flow_bugs {
            for bug in flow_bugs {
                eprintln!("Bug found: {}", bug);
            }
            panic!(
                "Found {} flow bugs, expected {}",
                found_flow_bugs, expected_flow_bugs
            );
        }
    }
    {
        let mut ctx_mode = Config::CONTEXT_SENSITIVE;
        ctx_mode.undef_hack = true;
        let mut db = uaf(&names, ctx_mode);
        db.run_rules();
        let ctx_bugs = db.query_context_flow();
        let found_ctx_bugs = ctx_bugs.len();
        if found_ctx_bugs != expected_ctx_bugs {
            for bug in ctx_bugs {
                eprintln!("Bug found: {}->{}", bug.free, bug.use_);
            }
            panic!(
                "Found {} ctx bugs, expected {}",
                found_ctx_bugs, expected_ctx_bugs
            );
        }
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
    run_uaf(&["simple"], 2, 2);
}

#[test]
fn safe() {
    run_uaf(&["safe"], 0, 0);
}

#[test]
fn path_sensitive() {
    run_uaf(&["path_sensitive"], 1, 1);
}

#[test]
fn remalloc() {
    run_uaf(&["remalloc"], 0, 0);
}

#[test]
fn ll() {
    run_uaf(&["ll"], 4, 4);
}

#[test]
fn loop_() {
    run_uaf(&["loop"], 2, 2);
}

#[test]
fn reloop() {
    run_uaf(&["reloop"], 0, 0);
}

#[test]
fn restale() {
    run_uaf(&["restale"], 0, 0);
}

// Mostly a test to make sure ctx sensitive doesn't jam
#[test]
fn recurse() {
    run_uaf(&["recurse"], 0, 0);
}

#[test]
fn undef_stack() {
    run_uaf(&["undef_stack"], 1, 1);
}

#[test]
fn undef_edge() {
    run_uaf(&["undef_edge"], 3, 3);
}

#[test]
fn field_overwrite() {
    run_uaf(&["field_overwrite"], 1, 1);
}

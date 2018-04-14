extern crate marduk;
use marduk::uaf;

fn run_uaf(names: &[&'static str], insensitive_bugs: usize, expected_flow_bugs: usize) {
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
    run_uaf(&["remalloc"], 2, 1);
}

#[test]
fn ll() {
    // If we add field sensitivity to the flow, this should drop a bit, but still not zero
    run_uaf(&["ll"], 6, 6);
}

#[test]
fn loop_() {
    run_uaf(&["loop"], 2, 2);
}

#[test]
fn ll_structure() {
    let mut db = uaf(&["samples/artificial/ll".to_string()], true);
    db.run_rules();
    // We're searching for something where a variable can point to itself and a dynamic value,
    // a signature of a linked list
    for flow_record in db.query_flow() {
        for (p, pts) in flow_record.pts {
            if pts.contains(&p) {
                for pt in pts {
                    if pt != p && pt.is_dyn() {
                        return;
                    }
                }
            }
        }
    }
    panic!("a -> {a, b} not found");
}

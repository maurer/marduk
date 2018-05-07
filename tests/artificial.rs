extern crate marduk;
use marduk::points_to::PointsTo;
use marduk::uaf;

fn run_uaf(
    names: &[&'static str],
    insensitive_bugs: usize,
    expected_flow_bugs: usize,
    expected_ctx_bugs: usize,
) {
    let names: Vec<_> = names
        .iter()
        .map(|x| format!("samples/artificial/{}", x))
        .collect();
    {
        let mut db = uaf(&names, marduk::AliasMode::All, false);
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
    {
        let mut db = uaf(&names, marduk::AliasMode::FlowOnly, true);
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
    run_uaf(&["func"], 1, 1, 1);
}

#[test]
fn link() {
    run_uaf(&["link", "external.so"], 2, 2, 2);
}

#[test]
fn simple() {
    run_uaf(&["simple"], 2, 2, 2);
}

#[test]
fn safe() {
    run_uaf(&["safe"], 0, 0, 0);
}

#[test]
fn path_sensitive() {
    run_uaf(&["path_sensitive"], 1, 1, 1);
}

#[test]
fn remalloc() {
    run_uaf(&["remalloc"], 2, 0, 0);
}

#[test]
fn ll() {
    // If we add field sensitivity to the flow, this should drop a bit, but still not zero
    // Since called from main, double contexts
    run_uaf(&["ll"], 8, 8, 16);
}

#[test]
fn loop_() {
    // Since both are called from main, there are two contexts under which the bugs occur
    run_uaf(&["loop"], 2, 2, 4);
}

#[test]
fn reloop() {
    run_uaf(&["reloop"], 1, 0, 0);
}

#[test]
fn restale() {
    run_uaf(&["restale"], 1, 0, 0);
}

#[test]
fn ll_structure() {
    let mut db = uaf(
        &["samples/artificial/ll".to_string()],
        marduk::AliasMode::All,
        false,
    );
    db.run_rules();
    // We're searching for something where a variable can point to itself and a dynamic value,
    // a signature of a linked list
    for flow_record in db.query_flow() {
        for (p, pts) in flow_record.pts.iter() {
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

#[test]
fn seq_call() {
    let mut db = uaf(
        &["samples/artificial/seq_call".to_string()],
        marduk::AliasMode::All,
        false,
    );
    db.run_rules();
    let mut main_exit = 0;
    let mut g_entry = 0;
    for sym in db.query_sym() {
        if sym.name == "main" {
            main_exit = sym.end;
        }
        if sym.name == "g" {
            g_entry = sym.loc.addr;
        }
    }
    if main_exit == 0 {
        panic!("main function not found");
    }
    if g_entry == 0 {
        panic!("g not found");
    }
    let mut main_checked = false;
    let mut g_checked = false;
    for flow in db.query_flow() {
        if flow.loc.addr == main_exit {
            check_rax(&flow.pts, 1, "main_exit");
            main_checked = true;
        }
        if flow.loc.addr == g_entry {
            check_rax(&flow.pts, 0, "g_entry");
            g_checked = true;
        }
    }
    if !main_checked {
        panic!("Points to set for main exit (0x{:x}) not found", main_exit);
    }
    if !g_checked {
        panic!("Points to set for g entry (0x{:x}) not found", g_entry);
    }
}

fn check_rax(pts: &PointsTo, target: usize, msg: &str) {
    use marduk::regs::Reg::RAX;
    use marduk::var::Var;
    let raxes = pts.iter()
        .filter(|(v, _)| match **v {
            Var::Register { register: RAX, .. } => true,
            _ => false,
        })
        .count();
    if raxes != target {
        panic!(
            "{}\nRAX defined {} times rather than {}",
            msg, raxes, target
        );
    }
}

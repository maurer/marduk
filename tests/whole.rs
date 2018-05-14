#[macro_use]
extern crate lazy_static;
extern crate marduk;
extern crate num_traits;

use marduk::uaf;
use num_traits::cast::ToPrimitive;

use std::sync::Mutex;

// Whole program analyses use too much memory to run them in parallel, so add a mutex to prevent
// memory overconsumption.
lazy_static! {
    static ref MEMLOCK: Mutex<()> = Mutex::new(());
}

fn run_uaf(
    names: &[&'static str],
    expected: &[(u64, u64)],
    false_positives_limit: Option<usize>,
    flow_false_positives_limit: Option<usize>,
    flow: bool,
) {
    let _memlock = MEMLOCK.lock().unwrap();
    let names: Vec<_> = names
        .iter()
        .map(|x| format!("samples/whole/{}", x))
        .collect();
    let mut db = uaf(
        &names,
        if flow {
            marduk::AliasMode::All
        } else {
            marduk::AliasMode::SteensOnly
        },
        false,
    );
    db.run_rules();

    {
        let mut false_positives_found = 0;
        let mut expected_not_found = expected.to_vec();
        for uaf in db.query_uaf() {
            let expect = (
                uaf.free.addr.to_u64().unwrap(),
                uaf.use_.addr.to_u64().unwrap(),
            );
            if let Some(pos) = expected_not_found.iter().position(|e| e == &expect) {
                expected_not_found.remove(pos);
            } else {
                false_positives_found += 1;
            }
        }

        if !expected_not_found.is_empty() {
            eprintln!("Expected insensitive bugs not found!");
            for absent in expected_not_found {
                eprintln!("free: 0x{:x} -> use: 0x{:x}", absent.0, absent.1);
            }
            panic!()
        }

        if let Some(false_positives) = false_positives_limit {
            if false_positives_found > false_positives {
                panic!(
                    "Too many insensitive false positives. Found: {} Expected: {}",
                    false_positives_found, false_positives
                );
            }
        }
    }
    if flow {
        // TODO eliminate dup
        let mut false_positives_found = 0;
        let mut expected_not_found = expected.to_vec();
        for uaf in db.query_uaf_flow() {
            let expect = (
                uaf.free.addr.to_u64().unwrap(),
                uaf.use_.addr.to_u64().unwrap(),
            );
            if let Some(pos) = expected_not_found.iter().position(|e| e == &expect) {
                expected_not_found.remove(pos);
            } else {
                false_positives_found += 1;
            }
        }

        if !expected_not_found.is_empty() {
            eprintln!("Expected flow bugs not found!");
            for absent in expected_not_found {
                eprintln!("free: 0x{:x} -> use: 0x{:x}", absent.0, absent.1);
            }
            panic!()
        }

        if let Some(false_positives) = flow_false_positives_limit {
            if false_positives_found > false_positives {
                panic!(
                    "Too many flow false positives. Found: {} Expected: {}",
                    false_positives_found, false_positives
                );
            }
        }
    }
}

#[test]
fn gnome_nettool() {
    run_uaf(
        &["gnome-nettool"],
        &[(0x411ba6, 0x4124d1)],
        None,
        Some(10),
        true,
    );
}

#[test]
fn goaccess() {
    run_uaf(&["goaccess"], &[(0x40b1dc, 0x40b230)], None, None, true);
}

#[test]
fn libarchive() {
    run_uaf(&["bsdcpio_test"], &[(0x40e012, 0x40e021)], None, None, true);
}

#[test]
fn shadowsocks_libev() {
    run_uaf(
        &["ss-server"],
        &[(0x411336, 0x412b57), (0x411336, 0x412b5d)],
        None,
        None,
        false,
    );
}

#[test]
fn isisd() {
    run_uaf(&["isisd"], &[(0x40a84f, 0x40aa1f)], None, None, true);
}

//#[test]
//fn ospf6d() {
//    run_uaf(
//        &["ospf6d"],
//        &[
//        ],
//        None,
//        None,
//        false,
//    );
//}

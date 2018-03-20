extern crate marduk;
extern crate num_traits;
use marduk::uaf;
use num_traits::cast::ToPrimitive;

fn run_uaf(names: &[&'static str], expected: &[(u64, u64)], false_positives_limit: Option<usize>) {
    let names: Vec<_> = names
        .iter()
        .map(|x| format!("samples/chops/{}", x))
        .collect();
    let mut db = uaf(&names, true);
    db.run_rules();

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
        eprintln!("Expected bugs not found!");
        for absent in expected_not_found {
            eprintln!("free: 0x{:x} -> use: 0x{:x}", absent.0, absent.1);
        }
        panic!()
    }

    if let Some(false_positives) = false_positives_limit {
        if false_positives_found > false_positives {
            panic!(
                "Too many false positives. Found: {} Expected: {}",
                false_positives_found, false_positives
            );
        }
    }
}

#[test]
fn color() {
    run_uaf(&["color.so"], &[(0x3f8c, 0x3fe0)], Some(0));
}

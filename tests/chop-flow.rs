extern crate marduk;
extern crate num_traits;
use marduk::uaf;
use num_traits::cast::ToPrimitive;

fn run_uaf(names: &[&'static str], expected: &[(u64, u64)], false_positives_limit: Option<usize>, undef_hack: bool) {
    let names: Vec<_> = names
        .iter()
        .map(|x| format!("samples/chops/{}", x))
        .collect();
    let mut config = marduk::Config::CONTEXT_INSENSITIVE;
    config.undef_hack = undef_hack;
    let mut db = uaf(&names, config);
    db.run_rules();

    let mut false_positives_found = 0;
    let mut expected_not_found = expected.to_vec();
    let mut unexpecteds = Vec::new();
    for uaf in db.query_uaf_flow() {
        let expect = (
            uaf.free.addr.to_u64().unwrap(),
            uaf.use_.addr.to_u64().unwrap(),
        );
        if let Some(pos) = expected_not_found.iter().position(|e| e == &expect) {
            expected_not_found.remove(pos);
        } else {
            false_positives_found += 1;
            unexpecteds.push(expect);
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
            eprintln!(
                "Too many false positives. Found: {} Expected: {}",
                false_positives_found, false_positives
            );
            eprintln!("False positives detected:");
            for unexpected in unexpecteds {
                eprintln!("free: 0x{:x} -> use: 0x{:x}", unexpected.0, unexpected.1);
            }
            panic!()
        } else if false_positives_found < false_positives {
            eprintln!("Found fewer false positives than the limit ({} < {}), consider reducing the limit.", false_positives_found, false_positives);
        }
    } else {
        eprintln!("Found {} false positives. Consider setting the limit.", false_positives_found);
    }
}

// The false positives here could be removed with a liveness filter
#[test]
fn color() {
    run_uaf(&["color.so"], &[(0x3f8c, 0x3fe0)], Some(0), false);
}

#[test]
fn shadowsocks() {
    run_uaf(&["ss.so"], &[(0x4cb6, 0x69e1), (0x4cb6, 0x69da)], Some(0), true);
}

#[test]
fn mdadm() {
    run_uaf(&["mdadm.so"], &[(0x20d4, 0x487d), (0x20d4, 0x46b1)], Some(19), true);
}

#[test]
fn isisd() {
    let use_sites: &[u64] = &[
        // Deletion of deleted adj
        0x34fe,
        0x350f,
        0x3523,
        0x3533,
        0x3553,
        0x3564,
        0x3578,
        0x3589,
        0x359d,
        0x35ae, // Re-use and recursion inside isis_adj_state_change
        0x363c,
        0x364f,
        0x37a2,
        0x380f,
        0x39d8,
        0x3a3d,
        0x3a6b,
        0x3ab2,
        0x3acc,
    ];
    let bugs: Vec<_> = use_sites.iter().map(|x| (0x35d2, *x)).collect();
    // False positive rate here is due to functions passign an adj in to isis_adj_state_change
    // which comes back freed. However, control flow + values in the surrounding code actually
    // guard against continued usage in this case.
    //
    // We're getting more false positives now due to turning on the "undefined variables are
    // self-referential" code. Uncalled functions are now potentially buggy as opposed to
    // automatically clean.
    run_uaf(&["isisd.so"], bugs.as_slice(), Some(93), false);
}

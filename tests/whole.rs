extern crate bap;
extern crate marduk;
use marduk::uaf;
use bap::high::bitvector::BitVector;

fn check_uaf(names: &[&'static str], bugs: &[u64], max_fp: usize) {
    let names: Vec<_> = names
        .iter()
        .map(|x| format!("samples/whole/{}", x))
        .collect();
    let mut db = uaf(&names);
    db.run_rules();
    let ans = db.query_get_uaf();
    // Make sure we found all the bugs
    for bug in bugs {
        assert!(
            ans.iter()
                .any(|found| found.addr == BitVector::from_u64(*bug, 64))
        );
    }

    // Make sure we don't have more false positives than allowed
    assert!(ans.len() <= max_fp + bugs.len());
}

#[test]
fn gnome_nettool() {
    check_uaf(&["gnome-nettool"], &[0x41_14ab], 0);
}

#[test]
fn bsdcpio_test() {
    check_uaf(&["bsdcpio_test"], &[0x406305], 1);
}

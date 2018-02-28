extern crate marduk;
use marduk::uaf;

// Just check whether the steensgaard output would contain the one important alias set, at its full
// extent
#[test]
fn simple_alias() {
    let mut db = uaf(&["samples/artificial/simple".to_string()], false, 0);
    db.run_rules();
    let steens = db.query_steens();
    for pt in steens {
        if format!("{}", pt) == "sp+8@samples/artificial/simple@0x4006d6->[RAX@samples/artificial/simple@0x4006df, sp+8@samples/artificial/simple@0x4006d6, RAX@samples/artificial/simple@0x4006e9, RAX@samples/artificial/simple@0x4006f1, RDI@samples/artificial/simple@0x4006f6, RAX@samples/artificial/simple@0x4006fe, RAX@samples/artificial/simple@0x400706]" {
            return
        }
    }
    panic!("Magic set not found")
}

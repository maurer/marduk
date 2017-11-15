#![feature(proc_macro)]
extern crate mycroft_support;
extern crate mycroft_macros;
extern crate bap;
extern crate mktemp;
extern crate num_traits;

pub mod avar;
pub mod datalog;
pub mod funcs;

pub fn uaf(files: &[&str]) {
    use datalog::Database;
    let mut db = Database::new();
    for file_name in files {
        use std::io::Read;
        use std::fs::File;
        let mut in_raw = Vec::new();
        let mut in_file = File::open(file_name).unwrap();
        in_file.read_to_end(&mut in_raw).unwrap();
        db.insert_file(datalog::File {
            name: file_name.to_string(),
            contents: in_raw,
        });
    }
    db.run_rules();
    println!("disas: {}\n", db.query_get_disasms().len());
    println!("syms: {}\n", db.query_get_syms().len());
}

#[test]
fn artificial() {
    uaf(
        &[
            "samples/artificial/func",
            "samples/artificial/external.so",
            "samples/artificial/link",
            "samples/artificial/loop",
            "samples/artificial/path_sensitive",
            "samples/artificial/remalloc",
            "samples/artificial/safe",
            "samples/artificial/simple",
        ],
    )
}

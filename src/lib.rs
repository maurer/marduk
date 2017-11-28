#![feature(proc_macro)]
extern crate bap;
extern crate mktemp;
extern crate mycroft_macros;
extern crate mycroft_support;
extern crate num_traits;

pub mod avar;
pub mod datalog;
pub mod funcs;
mod printers;

pub use datalog::Database;

pub fn uaf(files: &[String]) -> Database {
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
    db
}

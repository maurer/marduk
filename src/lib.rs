#![feature(proc_macro)]
extern crate bap;
#[macro_use]
extern crate log;
extern crate mktemp;
extern crate mycroft_macros;
extern crate mycroft_support;
extern crate num_traits;

pub mod datalog;
pub mod funcs;
pub mod steensgaard;
pub mod flow;
pub mod printers;

pub use datalog::Database;

pub fn uaf(files: &[String], flow_enable: bool) -> Database {
    let mut db = Database::new();
    for file_name in files {
        use std::io::Read;
        use std::fs::File;
        let mut in_raw = Vec::new();
        let mut in_file = File::open(file_name).unwrap();
        in_file.read_to_end(&mut in_raw).unwrap();
        if flow_enable {
            db.insert_flow_enable(datalog::FlowEnable { arg0: true });
        }
        db.insert_file(datalog::File {
            name: file_name.to_string(),
            contents: in_raw,
        });
    }
    db
}

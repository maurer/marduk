extern crate bap;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate mktemp;
#[macro_use]
extern crate mycroft_macros;
extern crate mycroft_support;
extern crate num_traits;

mod constraints;
pub mod datalog;
pub mod flow;
pub mod fmt_str;
pub mod interned_string;
pub mod load;
pub mod points_to;
pub mod printers;
pub mod regs;
pub mod steensgaard;
mod uaf;
pub mod use_def;
pub mod var;
mod effect;
pub use datalog::Database;

pub fn uaf(files: &[String], flow_enable: bool) -> Database {
    let mut db = Database::new();
    for file_name in files {
        use std::fs::File;
        use std::io::Read;
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

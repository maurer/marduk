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
mod effect;
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
pub use datalog::Database;

#[derive(Eq, Copy, Debug, PartialEq, Clone, Ord, PartialOrd)]
pub enum AliasMode {
    SteensOnly,
    FlowOnly,
    All,
}

pub fn uaf(files: &[String], alias_mode: AliasMode) -> Database {
    use AliasMode::*;
    let mut db = Database::new();
    for file_name in files {
        use std::fs::File;
        use std::io::Read;
        let mut in_raw = Vec::new();
        let mut in_file = File::open(file_name).unwrap();
        in_file.read_to_end(&mut in_raw).unwrap();

        db.insert_file(datalog::File {
            name: file_name.to_string(),
            contents: in_raw,
        });
    }

    match alias_mode {
        SteensOnly => db.insert_steens_enable(datalog::SteensEnable { arg0: true }),
        FlowOnly => db.insert_flow_enable(datalog::FlowEnable { arg0: true }),
        All => {
            db.insert_steens_enable(datalog::SteensEnable { arg0: true });
            db.insert_flow_enable(datalog::FlowEnable { arg0: true })
        }
    };

    db
}

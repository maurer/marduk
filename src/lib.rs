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
#[macro_use]
extern crate serde_derive;
extern crate serde;

mod constraints;
mod context;
pub mod datalog;
mod effect;
pub mod flow;
pub mod fmt_str;
pub mod interned_string;
pub mod load;
pub mod points_to;
pub mod printers;
pub mod regs;
mod uaf;
mod use_def;
pub mod var;
pub use datalog::Database;

#[derive(Eq, Copy, Debug, PartialEq, Clone, Ord, PartialOrd, Serialize, Deserialize)]
pub enum AliasMode {
    SteensOnly { ctx: bool },
    FlowOnly { ctx: bool },
    Both { ctx: bool },
}
use AliasMode::*;

impl AliasMode {
    pub fn uses_steens(&self) -> bool {
        match *self {
            SteensOnly { .. } | Both { .. } => true,
            FlowOnly { .. } => false,
        }
    }
    pub fn uses_flow(&self) -> bool {
        match *self {
            FlowOnly { .. } | Both { .. } => true,
            SteensOnly { .. } => false,
        }
    }
    pub fn uses_ctx(&self) -> bool {
        match *self {
            FlowOnly { ref ctx } | SteensOnly { ref ctx } | Both { ref ctx } => *ctx,
        }
    }
}

pub fn uaf(files: &[String], alias_mode: AliasMode) -> Database {
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

    if alias_mode.uses_steens() {
        panic!("Steens disabled");
    }
    if alias_mode.uses_flow() {
        db.insert_flow_enable(datalog::FlowEnable { arg0: true });
    }
    if alias_mode.uses_ctx() {
        db.insert_context_enable(datalog::ContextEnable { arg0: true });
    }

    db
}

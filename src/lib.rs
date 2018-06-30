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
mod live;
pub mod load;
pub mod points_to;
pub mod printers;
pub mod regs;
mod uaf;
mod use_def;
pub mod var;
pub use datalog::Database;

#[derive(Eq, Copy, Debug, PartialEq, Clone, Ord, PartialOrd, Serialize, Deserialize)]
pub enum LocType {
    Addr,
    AddrAndStack,
}

#[derive(Eq, Copy, Debug, PartialEq, Clone, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Config {
    pub loc_type: LocType,
    pub load_only: bool,
    pub undef_hack: bool,
}

impl Config {
    pub const CONTEXT_SENSITIVE: Self = Self {
        loc_type: LocType::AddrAndStack,
        load_only: false,
        undef_hack: false,
    };

    pub const CONTEXT_INSENSITIVE: Self = Config {
        loc_type: LocType::Addr,
        load_only: false,
        undef_hack: false,
    };

    pub fn uses_flow(self) -> bool {
        !self.load_only
    }
    pub fn uses_ctx(self) -> bool {
        match self.loc_type {
            LocType::AddrAndStack => true,
            LocType::Addr => false,
        }
    }
    pub fn defines_undef(self) -> bool {
        self.undef_hack
    }
}

pub fn uaf(files: &[String], config: Config) -> Database {
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

    if config.uses_flow() {
        db.insert_flow_enable(datalog::FlowEnable { arg0: true });
    }
    if config.uses_ctx() {
        db.insert_context_enable(datalog::ContextEnable { arg0: true });
    }
    if config.defines_undef() {
        db.insert_undef_hack(datalog::UndefHack { arg0: true });
    }

    db
}

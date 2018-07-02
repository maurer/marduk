#![deny(missing_docs)]

//! `marduk` implements variable sensitivity alias analysis and
//! use-after-free analysis over compiled programs in the mycroft dialect
//! of datalog.
//!
//! To use it programatically, call the `uaf` function to produce a
//! mycroft `Database`, run the database, then run queries on it to
//! examine the results.
//!
//! An example can be found in src/bin/uaf.rs, the command line frontend
//! to this library.

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
mod datalog;
mod effect;
mod flow;
mod fmt_str;
mod interned_string;
mod live;
mod load;
mod points_to;
mod printers;
mod regs;
mod uaf;
mod use_def;
mod var;
pub use datalog::*;

#[derive(Eq, Copy, Debug, PartialEq, Clone, Ord, PartialOrd, Serialize, Deserialize)]
/// Describes the kind of location to use in the dataflow analysis.
pub enum LocType {
    /// Parameterize location solely based on instruction pointer
    Addr,
    /// Augment the instruction pointer with a fixed length callstack
    /// (compile time constant configurable, currently 1)
    AddrAndStack,
}

#[derive(Eq, Copy, Debug, PartialEq, Clone, Ord, PartialOrd, Serialize, Deserialize)]
/// How to run the use-after-free analysis
pub struct Config {
    /// Analysis sensitivity
    pub loc_type: LocType,
    /// If true, will perform no analysis, and only load the binary.
    /// Mostly relevant for performance and debugging.
    pub load_only: bool,
    /// If true, functions with arguments which are nowhere defined will be initialized
    /// as though they were non-aliasing structs with width 4 and depth 2.
    /// These values are also compile time constants.
    pub undef_hack: bool,
}

impl Config {
    /// Default config for context sensitive analysis
    pub const CONTEXT_SENSITIVE: Self = Self {
        loc_type: LocType::AddrAndStack,
        load_only: false,
        undef_hack: false,
    };

    /// Default config for context insensitive analysis
    pub const CONTEXT_INSENSITIVE: Self = Config {
        loc_type: LocType::Addr,
        load_only: false,
        undef_hack: false,
    };

    /// Whether the configuration implies flow sensitivity
    pub fn uses_flow(self) -> bool {
        !self.load_only
    }

    /// Whether the configuration implies context sensitivity
    pub fn uses_ctx(self) -> bool {
        match self.loc_type {
            LocType::AddrAndStack => true,
            LocType::Addr => false,
        }
    }

    /// Whether to automatically define undefined arguments to
    /// an arbitrary struct region (width 4, depth 2, nonaliasing)
    pub fn defines_undef(self) -> bool {
        self.undef_hack
    }
}

/// Produces a mycroft database containing your input files and the
/// configured rules. You must run the rules in the database
/// before queries will return any useful output.
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

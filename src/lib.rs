#![feature(proc_macro)]
extern crate bap;
extern crate mktemp;
extern crate mycroft_macros;
extern crate mycroft_support;
extern crate num_traits;

pub mod avar;
pub mod datalog;
pub mod funcs;

impl ::std::fmt::Display for datalog::FuncResult {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        write!(f, "{}@{}:{}", self.file, self.entry, self.addr)
    }
}

impl ::std::fmt::Display for datalog::CallSiteResult {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        write!(
            f,
            "{}@{}->{}@{}",
            self.call_file,
            self.call_addr,
            self.dst_file,
            self.dst_addr
        )
    }
}

impl ::std::fmt::Display for datalog::GetUafFlowResult {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        write!(f, "{}@{}:{}", self.name, self.addr, self.alias)
    }
}

impl ::std::fmt::Display for datalog::SuccResult {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        write!(
            f,
            "{}@{}->{}~call={}",
            self.name,
            self.src,
            self.dst,
            self.call
        )
    }
}

impl ::std::fmt::Display for datalog::LinkPadResult {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        write!(f, "{}@{}: {}", self.name, self.addr, self.pad_name)
    }
}

impl ::std::fmt::Display for datalog::GetFreeCallResult {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        write!(f, "{}@{}", self.name, self.addr)
    }
}

impl ::std::fmt::Display for datalog::GetMallocCallResult {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        write!(f, "{}@{}", self.name, self.addr)
    }
}

impl ::std::fmt::Display for datalog::LiveResult {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        write!(f, "{}@{}", self.name, self.addr)
    }
}

impl ::std::fmt::Display for datalog::GetAliasResult {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        write!(
            f,
            "{}@{}:{} -> {}@{}:{} : {}",
            self.file0,
            self.addr0,
            self.alias_set,
            self.file,
            self.addr,
            self.a_var,
            self.freed
        )
    }
}

pub fn uaf(files: &[String]) -> datalog::Database {
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
    db
}

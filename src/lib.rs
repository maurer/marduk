#![feature(proc_macro)]
extern crate mycroft_support;
extern crate mycroft_macros;
extern crate bap;
extern crate mktemp;
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
fn print_state(db: &datalog::Database) {
    println!("disas: {}", db.query_get_disasms().len());
    println!("alias: {}", db.query_get_alias().len());
    println!("---\npad");
    for pad in db.query_link_pad() {
        println!("{}", pad)
    }
    println!("---\nsucc");
    for s in db.query_succ() {
        println!("{}", s)
    }
    println!("---\nfunc");
    for f in db.query_func() {
        println!("{}", f)
    }

    println!("---\ncall_site");
    for cs in db.query_call_site() {
        println!("{}", cs)
    }
    println!("---\nalias");
    for alias in db.query_get_alias() {
        println!("{}", alias)
    }
    //println!("---\nmalloc_call");
    //for fc in db.query_get_malloc_call() {
    //    println!("{}", fc)
    //}
    //println!("---\nfree_call");
    //for fc in db.query_get_free_call() {
    //    println!("{}", fc)
    //}
    //println!("---\nlive");
    //for fc in db.query_live() {
    //    println!("{}", fc)
    //}
    println!("---\nuaf");
    for flow in db.query_get_uaf_flow() {
        println!("{}", flow)
    }
}

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
    let mut step = 0;
    while db.run_rules_once() {
        println!("step: {}", step);
        step = step + 1;

        print_state(&db);
    }
    //db.run_rules();
    print_state(&db);
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
/*
#[test]
fn simple() {
    uaf(&["samples/artificial/func"])
}
*/

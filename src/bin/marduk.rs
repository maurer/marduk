extern crate clap;
extern crate env_logger;
extern crate marduk;

use marduk::Config;

fn print_results(db: &mut marduk::Database) {
    println!("UaF (free -> use):");
    for x in db.query_uaf_flow() {
        println!("{}", x);
    }
}

fn print_state(db: &mut marduk::Database) {
    println!("PTS:");
    for x in db.query_flow() {
        println!("{}", x);
    }

    println!("PTS OUT:");
    for x in db.query_flow_out() {
        println!("{}:\n{}", x.loc, x.pts);
    }

    for x in db.query_get_malloc_call() {
        println!("malloc {}", x);
    }

    for x in db.query_succ() {
        if x.src.is_stacked() != x.dst.is_stacked() {
            println!("SUCC BUG");
        }
        println!("succ {} -> {}", x.src, x.dst);
    }

    for x in db.query_func() {
        println!("func {}: {}", x.base, x.contains);
    }

    for x in db.query_call_site() {
        if x.call_loc.is_stacked() != x.target_loc.is_stacked() {
            println!("CALL_SITE BUG");
        }
        println!(
            "call_site {} - {} -> {}",
            x.call_loc, x.target_loc, x.ret_loc
        );
    }

    for x in db.query_used_var() {
        println!("{}", x);
    }

    for x in db.query_live_vars() {
        println!("{}", x);
    }

    for x in db.query_constraints() {
        println!("c: {}:", x.loc);
        for c in &x.c {
            println!("{}", c);
        }
    }

    for x in db.query_uncalled() {
        println!("uncalled: {}", x.loc);
    }
}

fn main() {
    use clap::{App, Arg};

    env_logger::init();

    let args = App::new("marduk")
        .version("0.1")
        .about("Uses alias analysis to find use-after-free in compiled code.")
        .author("Matthew Maurer")
        .arg(
            Arg::with_name("INPUTS")
                .help("Which ELF files to load")
                .multiple(true)
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("debug")
                .help(
                    "Dump debugging information in addition to results.\n\
                     Additional detail can be found by setting RUST_LOG=marduk=trace.",
                )
                .short("d")
                .long("debug"),
        )
        .arg(
            Arg::with_name("sensitivity")
                .help("Can be \"flow\" or \"context\". Defaults to \"flow\".")
                .takes_value(true)
                .short("s")
                .long("sensitivity"),
        )
        .arg(
            Arg::with_name("undefined-initialize")
                .help(
                    "Defines a memory structure at every variable which is \
                     live at the entry point of a function, but never defined.\n\
                     This hack is intended to allow approximate analysis of \
                     functions which do not appear to be called, but may be called \
                     through an indirect jump/call or library callback.\n\
                     There is no completeness gaurantee for this feature, as we do \
                     not have the complete control flow graph, it is of practical \
                     use only.",
                )
                .short("u")
                .long("undef-init"),
        )
        .arg(
            Arg::with_name("progress")
                .help(
                    "Dump step count and timestamp as the computation \
                     progresses.\n \
                     This feature is mostly of use to tell the difference between \
                     a stalled computation and one making progress, or to interpret \
                     information from the tracing log for performance purposes.",
                )
                .short("p")
                .long("progress"),
        )
        .get_matches();

    let mut config = match args.value_of("sensitivity").unwrap_or("flow") {
        "flow" => Config::CONTEXT_INSENSITIVE,
        "context" => Config::CONTEXT_SENSITIVE,
        s => panic!("Unknown sensitivity: {}", s),
    };

    config.undef_hack = args.is_present("undefined-initialize");

    let files: Vec<String> = args.values_of("INPUTS")
        .expect("At least one input is required")
        .map(str::to_string)
        .collect();

    run_marduk(
        &files,
        config,
        args.is_present("progress"),
        args.is_present("debug"),
    );
}

fn run_marduk(files: &[String], config: Config, progress: bool, debug: bool) {
    use std::time::Instant;

    let mut db = marduk::uaf(files, config);

    let mut step = 0;
    let mut last_round = Vec::new();
    if progress {
        println!("Booting");
    }
    let total = Instant::now();
    while !last_round.is_empty() || step == 0 {
        let mark = Instant::now();
        last_round = db.run_rules_once();
        step += 1;
        if progress {
            println!(
                "Step {} complete, took {:?}, {:?} total",
                step,
                mark.elapsed(),
                total.elapsed()
            );
        }
    }
    print_results(&mut db);
    if debug {
        print_state(&mut db);
    }
}

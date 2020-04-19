extern crate jemalloc_ctl;
extern crate jemallocator;
extern crate marduk;
extern crate num_traits;

use self::jemallocator::Jemalloc;
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

pub use self::jemalloc_ctl::epoch;
pub use self::jemalloc_ctl::stats::allocated;
pub use self::marduk::{uaf, Config, Database, LocType};
pub use self::num_traits::cast::ToPrimitive;
pub use std::collections::{BTreeMap, BTreeSet};
pub use std::time::{Duration, Instant};

pub use measurement::Measurement;

#[derive(Copy, Clone, Eq, Debug, PartialOrd, PartialEq)]
pub struct Case {
    pub names: &'static [&'static str],
    pub expected: &'static [(u64, u64)],
}

pub struct Run {
    pub db: Database,
    pub time: Duration,
    pub space: usize,
}

fn check_mem() -> usize {
    epoch::advance().unwrap(); // Refresh the allocated statistic.
    allocated::read().unwrap()
}

pub fn marduk(names: &[String], mode: Config) -> Option<Run> {
    let mut db = uaf(names, mode);
    let pre = Instant::now();
    let time_limit = Duration::from_secs(::TIME_LIMIT);
    while !db.run_rules_once().is_empty() {
        if check_mem() > ::MEMORY_LIMIT {
            eprintln!("Over memory on {:?}", names);
            return None;
        }
        if pre.elapsed() > time_limit {
            eprintln!("Out of time on {:?}", names);
            return None;
        }
    }
    let time = pre.elapsed();
    let space = check_mem();
    Some(Run { db, time, space })
}

pub fn uaf_tuple(uaf: &marduk::AllUafResult) -> (u64, u64) {
    (
        uaf.free.addr.to_u64().unwrap(),
        uaf.use_.addr.to_u64().unwrap(),
    )
}

pub fn measure_mode(
    names: &[String],
    mode: Config,
    expected: &[(u64, u64)],
) -> Option<Measurement> {
    let mut run = marduk(names, mode)?;
    let mut false_positives = 0;
    let mut expected_not_found = expected.to_vec();
    let mut found = BTreeSet::new();
    for uaf in run.db.query_all_uaf() {
        let expect = uaf_tuple(&uaf);
        if mode.uses_ctx() && !uaf.free.is_stacked() {
            continue;
        }
        if !found.insert(expect) {
            // We already processed this candidate
            continue;
        }
        if let Some(pos) = expected_not_found.iter().position(|e| e == &expect) {
            expected_not_found.remove(pos);
        } else {
            false_positives += 1;
        }
    }
    Some(Measurement {
        mode,
        artifact: names.to_vec(),
        true_positives: expected.len() - expected_not_found.len(),
        false_positives,
        found: found.iter().cloned().collect(),
        true_negatives: expected_not_found,
        time: run.time,
        space: run.space,
    })
}

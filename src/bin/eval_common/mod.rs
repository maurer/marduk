extern crate jemalloc_ctl;
extern crate marduk;
extern crate num_traits;
extern crate serde;

pub use self::jemalloc_ctl::stats::Allocated;
pub use self::jemalloc_ctl::Epoch;
pub use self::marduk::{uaf, AliasMode, Database};
pub use self::num_traits::cast::ToPrimitive;
pub use std::collections::{BTreeMap, BTreeSet};
pub use std::time::{Duration, Instant};

#[derive(Clone, Eq, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct Measurement {
    pub mode: AliasMode,
    pub artifact: Vec<String>,
    pub true_positives: usize,
    pub false_positives: usize,
    // Which bugs were missed
    pub true_negatives: Vec<(u64, u64)>,
    // What was found at all, mostly for use in debug or triage later.
    pub found: Vec<(u64, u64)>,
    pub time: Duration,
    pub space: usize, // bytes
}

#[derive(Copy, Clone, Eq, Debug, PartialOrd, PartialEq)]
pub struct Case {
    pub names: &'static [&'static str],
    pub expected: &'static [(u64, u64)],
}

mod printers {
    use super::marduk::printers::fmt_vec;
    use std::fmt::{Display, Formatter, Result};
    use Measurement;
    fn fmt_space(f: &mut Formatter, space: usize) -> Result {
        const GIGA: usize = 1024 * MEGA;
        const MEGA: usize = 1024 * KILO;
        const KILO: usize = 1024;
        if space > GIGA {
            write!(f, "{}G", space / GIGA)
        } else if space > MEGA {
            write!(f, "{}M", space / MEGA)
        } else {
            write!(f, "{}k", space / KILO)
        }
    }

    impl Display for Measurement {
        fn fmt(&self, f: &mut Formatter) -> Result {
            fmt_vec(f, &self.artifact)?;
            writeln!(
                f,
                "~{}\n+{} -{}",
                self.mode, self.true_positives, self.false_positives
            )?;

            let time_mins = self.time.as_secs() / 60;
            let time_secs = self.time.as_secs() % 60;
            if time_mins > 0 {
                write!(f, "{}m", time_mins)?;
            }
            writeln!(f, "{}s", time_secs)?;

            fmt_space(f, self.space)?;
            writeln!(f)?;

            for tn in &self.true_negatives {
                writeln!(f, "Missed bug! 0x{:x}->0x{:x}", tn.0, tn.1)?;
            }

            Ok(())
        }
    }
}

pub struct Run {
    pub db: Database,
    pub time: Duration,
    pub space: usize,
}

fn check_mem() -> usize {
    let epoch = Epoch::new().unwrap();
    let allocated = Allocated::new().unwrap();
    epoch.advance().unwrap(); // Refresh the allocated statistic.
    allocated.get().unwrap()
}

pub fn marduk(names: &[String], mode: AliasMode) -> Option<Run> {
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

pub fn uaf_tuple(uaf: &marduk::datalog::AllUafResult) -> (u64, u64) {
    (
        uaf.free.addr.to_u64().unwrap(),
        uaf.use_.addr.to_u64().unwrap(),
    )
}

pub fn measure_mode(
    names: &[String],
    mode: AliasMode,
    expected: &[(u64, u64)],
) -> Option<Measurement> {
    let mut run = marduk(names, mode)?;
    let mut false_positives = 0;
    let mut expected_not_found = expected.to_vec();
    let mut found = BTreeSet::new();
    for uaf in run.db.query_all_uaf() {
        let expect = uaf_tuple(&uaf);
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

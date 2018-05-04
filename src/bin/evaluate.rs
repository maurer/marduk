extern crate jemalloc_ctl;
extern crate marduk;
extern crate num_traits;

use jemalloc_ctl::stats::Allocated;
use jemalloc_ctl::Epoch;
use marduk::{uaf, AliasMode};
use num_traits::cast::ToPrimitive;
use std::time::{Duration, Instant};

#[derive(Clone, Eq, Debug, PartialOrd, PartialEq)]
struct Measurement {
    mode: AliasMode,
    artifact: Vec<String>,
    true_positives: u64,
    false_positives: u64,
    // Which bugs were missed
    true_negatives: Vec<(u64, u64)>,
    time: Duration,
    space: usize, // bytes
}

#[derive(Copy, Clone, Eq, Debug, PartialOrd, PartialEq)]
struct Case {
    names: &'static [&'static str],
    expected: &'static [(u64, u64)],
}

mod printers {
    use marduk::printers::fmt_vec;
    use std::fmt::{Display, Formatter, Result};
    use Measurement;
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

            writeln!(f, "{}G", self.space / (1024 * 1024 * 1024))?;

            for tn in &self.true_negatives {
                writeln!(f, "Missed bug! {}->{}", tn.0, tn.1)?;
            }

            Ok(())
        }
    }
}

fn measure_uaf(names: &[&'static str], expected: &[(u64, u64)]) -> Vec<Measurement> {
    let names: Vec<_> = names
        .iter()
        .map(|x| format!("samples/whole/{}", x))
        .collect();
    [AliasMode::SteensOnly, AliasMode::FlowOnly]
        .iter()
        .map(|mode| measure_mode(&names, *mode, expected))
        .collect()
}

fn measure_mode(names: &[String], mode: AliasMode, expected: &[(u64, u64)]) -> Measurement {
    let mut db = uaf(names, mode);
    let pre = Instant::now();
    db.run_rules();
    let time = pre.elapsed();
    let space = {
        let epoch = Epoch::new().unwrap();
        let allocated = Allocated::new().unwrap();
        epoch.advance().unwrap(); // Refresh the allocated statistic.
        allocated.get().unwrap()
    };
    let mut false_positives = 0;
    let mut expected_not_found = expected.to_vec();
    for uaf in db.query_all_uaf() {
        let expect = (
            uaf.free.addr.to_u64().unwrap(),
            uaf.use_.addr.to_u64().unwrap(),
        );
        if let Some(pos) = expected_not_found.iter().position(|e| e == &expect) {
            expected_not_found.remove(pos);
        } else {
            false_positives += 1;
        }
    }
    Measurement {
        mode,
        artifact: names.to_vec(),
        true_positives: (expected.len() - expected_not_found.len()) as u64,
        false_positives,
        true_negatives: expected_not_found,
        time,
        space,
    }
}

const GNOME_NETTOOL: Case = Case {
    names: &["gnome-nettool"],
    expected: &[(0x411ba6, 0x4124d1)],
};

const KNOWN_BUGS: &[Case] = &[GNOME_NETTOOL];

fn main() {
    let known_measures: Vec<_> = KNOWN_BUGS
        .iter()
        .flat_map(|case| measure_uaf(case.names, case.expected))
        .collect();
    for measure in known_measures {
        println!("{}", measure)
    }
}

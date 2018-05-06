// Addresses are considered long literals, but I'd prefer not to insert an underscore every four
// characters
#![cfg_attr(feature = "cargo-clippy", allow(unreadable_literal))]
extern crate jemalloc_ctl;
extern crate marduk;
extern crate num_traits;

use jemalloc_ctl::stats::Allocated;
use jemalloc_ctl::Epoch;
use marduk::{uaf, AliasMode, Database};
use num_traits::cast::ToPrimitive;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

#[derive(Clone, Eq, Debug, PartialOrd, PartialEq)]
struct Measurement {
    mode: AliasMode,
    artifact: Vec<String>,
    true_positives: usize,
    false_positives: usize,
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
                writeln!(f, "Missed bug! {}->{}", tn.0, tn.1)?;
            }

            Ok(())
        }
    }
}

fn measure_individual_juliet(juliet_tp: &BTreeMap<String, usize>) -> Vec<Measurement> {
    let mut out = Vec::new();
    for (name, tps) in juliet_tp {
        let path = format!("samples/Juliet-1.3/CWE416/individuals/{}", name);
        let mut steens_run = marduk(&[path.clone()], AliasMode::SteensOnly);
        let mut flow_run = marduk(&[path.clone()], AliasMode::FlowOnly);

        // Check that Steens contains all of Flow. Since Flow has all the TPs, this means Steens
        // does too. Additionally, it's a bug if Steens doesn't contain something Flow does.

        let steens_set: BTreeSet<_> = steens_run
            .db
            .query_all_uaf()
            .iter()
            .map(uaf_tuple)
            .collect();
        let flow_set: BTreeSet<_> = flow_run.db.query_all_uaf().iter().map(uaf_tuple).collect();
        assert_eq!(flow_set.difference(&steens_set).count(), 0);

        out.push(Measurement {
            mode: AliasMode::SteensOnly,
            artifact: vec![path.to_string()],
            true_positives: *tps,
            false_positives: steens_set.len() - tps,
            true_negatives: Vec::new(),
            time: steens_run.time,
            space: steens_run.space,
        });
        out.push(Measurement {
            mode: AliasMode::FlowOnly,
            artifact: vec![path.to_string()],
            true_positives: *tps,
            false_positives: flow_set.len() - tps,
            true_negatives: Vec::new(),
            time: flow_run.time,
            space: flow_run.space,
        });
    }
    out
}

fn measure_bad_juliet() -> Vec<Measurement> {
    use std::fs;
    let mut out = Vec::new();
    // I checked over these manually, and it took forever, but they were all real, and I'm unlikely
    // to generate spurious reports on flow mode for small snippets of only bad code on an update.
    //
    // That said, if the flow algorithm changes nontrivially before publication, I have to check
    // these manually. Again.
    for entry in fs::read_dir("samples/Juliet-1.3/CWE416/omit_good_individuals").unwrap() {
        let path = entry
            .unwrap()
            .path()
            .as_os_str()
            .to_str()
            .unwrap()
            .to_string();
        let mode = AliasMode::FlowOnly;
        let mut run = marduk(&[path.clone()], mode);
        out.push(Measurement {
            mode,
            artifact: vec![path.to_string()],
            true_positives: run.db.query_all_uaf().len(),
            false_positives: 0,
            true_negatives: Vec::new(),
            time: run.time,
            space: run.space,
        })
    }
    out
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

struct Run {
    db: Database,
    time: Duration,
    space: usize,
}

fn marduk(names: &[String], mode: AliasMode) -> Run {
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
    Run { db, time, space }
}

fn uaf_tuple(uaf: &marduk::datalog::AllUafResult) -> (u64, u64) {
    (
        uaf.free.addr.to_u64().unwrap(),
        uaf.use_.addr.to_u64().unwrap(),
    )
}

fn measure_whole_juliet(mode: AliasMode, tps: usize) -> Measurement {
    let mut run = marduk(&["samples/Juliet-1.3/CWE416/CWE416".to_string()], mode);
    Measurement {
        mode,
        artifact: vec!["CWE416".to_string()],
        true_positives: tps,
        false_positives: run.db.query_all_uaf().len() - tps,
        true_negatives: Vec::new(),
        time: run.time,
        space: run.space,
    }
}

fn measure_mode(names: &[String], mode: AliasMode, expected: &[(u64, u64)]) -> Measurement {
    let mut run = marduk(names, mode);
    let mut false_positives = 0;
    let mut expected_not_found = expected.to_vec();
    for uaf in run.db.query_all_uaf() {
        let expect = uaf_tuple(&uaf);
        if let Some(pos) = expected_not_found.iter().position(|e| e == &expect) {
            expected_not_found.remove(pos);
        } else {
            false_positives += 1;
        }
    }
    Measurement {
        mode,
        artifact: names.to_vec(),
        true_positives: expected.len() - expected_not_found.len(),
        false_positives,
        true_negatives: expected_not_found,
        time: run.time,
        space: run.space,
    }
}

const GNOME_NETTOOL: Case = Case {
    names: &["gnome-nettool"],
    expected: &[(0x411ba6, 0x4124d1)],
};

const GOACCESS: Case = Case {
    names: &["goaccess"],
    expected: &[(0x40b1dc, 0x40b230)],
};

const LIBARCHIVE: Case = Case {
    names: &["bsdcpio_test"],
    expected: &[(0x40e012, 0x40e021)],
};

const SHADOWSOCKS: Case = Case {
    names: &["ss-server"],
    expected: &[(0x411336, 0x412b57), (0x411336, 0x412b5d)],
};

const ISISD: Case = Case {
    names: &["isisd"],
    expected: &[(0x40a84f, 0x40aa1f)],
};

const OSPF6D: Case = Case {
    names: &["ospf6d"],
    expected: &[
        (0x42de10, 0x436c59),
        (0x42de10, 0x437cf1),
        (0x42de10, 0x437d05),
    ],
};

const KNOWN_BUGS: &[Case] = &[
    GNOME_NETTOOL,
    GOACCESS,
    LIBARCHIVE,
    SHADOWSOCKS,
    ISISD,
    OSPF6D,
];

fn main() {
    let known_measures: Vec<_> = KNOWN_BUGS
        .iter()
        .flat_map(|case| measure_uaf(case.names, case.expected))
        .collect();
    println!("Known bugs:");
    for measure in known_measures {
        println!("{}", measure)
    }

    let juliet_tp = {
        use std::path::PathBuf;
        let mut juliet_tp = BTreeMap::new();
        for m in measure_bad_juliet() {
            juliet_tp.insert(
                PathBuf::from(&m.artifact[0])
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
                m.true_positives,
            );
        }
        juliet_tp
    };

    println!("Juliet Individuals:");
    for measure in measure_individual_juliet(&juliet_tp) {
        println!("{}", measure)
    }

    println!("Juliet Whole:");
    let juliet_tp_sum = juliet_tp.values().sum();
    println!(
        "{}",
        measure_whole_juliet(AliasMode::FlowOnly, juliet_tp_sum)
    );
    println!(
        "{}",
        measure_whole_juliet(AliasMode::SteensOnly, juliet_tp_sum)
    );
}

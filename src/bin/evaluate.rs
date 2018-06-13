// Addresses are considered long literals, but I'd prefer not to insert an underscore every four
// characters
#![cfg_attr(feature = "cargo-clippy", allow(unreadable_literal))]

// Macros need to be loaded at root
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod eval_common;
use eval_common::*;

pub const MEMORY_LIMIT: usize = 1024 * 1024 * 1024 * 100; // 100G
pub const TIME_LIMIT: u64 = 24 * 60 * 60; // 24hrs, effectively infinite

fn measure_individual_juliet(juliet_tp: &BTreeMap<String, usize>) -> Vec<Measurement> {
    let mut out = Vec::new();
    for (name, tps) in juliet_tp {
        let path = format!("samples/Juliet-1.3/CWE416/individuals/{}", name);
        let mut flow_run = marduk(&[path.clone()], AliasMode::FlowOnly { ctx: false }).unwrap();

        // Check that Steens contains all of Flow. Since Flow has all the TPs, this means Steens
        // does too. Additionally, it's a bug if Steens doesn't contain something Flow does.

        let flow_set: BTreeSet<_> = flow_run.db.query_all_uaf().iter().map(uaf_tuple).collect();
        out.push(Measurement {
            mode: AliasMode::FlowOnly { ctx: false },
            artifact: vec![path.to_string()],
            true_positives: *tps,
            false_positives: flow_set.len() - tps,
            found: flow_set.iter().cloned().collect(),
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
        let mode = AliasMode::FlowOnly { ctx: false };
        let mut run = marduk(&[path.clone()], mode).unwrap();
        let out_set: BTreeSet<_> = run.db.query_all_uaf().iter().map(uaf_tuple).collect();
        out.push(Measurement {
            mode,
            artifact: vec![path.to_string()],
            true_positives: out_set.len(),
            found: out_set.iter().cloned().collect(),
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
    [
        AliasMode::FlowOnly { ctx: false },
        AliasMode::FlowOnly { ctx: true },
    ].iter()
        .flat_map(|mode| measure_mode(&names, *mode, expected))
        .collect()
}

fn measure_whole_juliet(mode: AliasMode, tps: usize) -> Measurement {
    let mut run = marduk(&["samples/Juliet-1.3/CWE416/CWE416".to_string()], mode).unwrap();
    let out_set: BTreeSet<_> = run.db.query_all_uaf().iter().map(uaf_tuple).collect();
    Measurement {
        mode,
        artifact: vec!["CWE416".to_string()],
        true_positives: tps,
        false_positives: out_set.len() - tps,
        found: out_set.iter().cloned().collect(),
        true_negatives: Vec::new(),
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
    expected: &[(0x410f16, 0x412c97), (0x410f16, 0x412c90)],
};

const ISISD: Case = Case {
    names: &["isisd"],
    expected: &[(0x40a84f, 0x40aa1f)],
};

const OSPF6D: Case = Case {
    names: &["ospf6d"],
    expected: &[
        (0x42ed00, 0x437506),
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
    let mut known = Vec::new();
    println!("Known bugs:");
    for measure in KNOWN_BUGS
        .iter()
        .flat_map(|case| measure_uaf(case.names, case.expected))
    {
        println!("{}", measure);
        known.push(measure);
    }

    {
        let mut out = ::std::fs::File::create("eval.json").unwrap();
        serde_json::to_writer(&mut out, &known).unwrap();
    }

    println!("Processing juliet");

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
        measure_whole_juliet(AliasMode::FlowOnly { ctx: false }, juliet_tp_sum)
    );
}

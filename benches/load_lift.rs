extern crate criterion;
extern crate marduk;

use criterion::{criterion_group, criterion_main, Bencher, BenchmarkId, Criterion};
use marduk::{uaf, Config};

fn load_lift_bin(b: &mut Bencher, bin: &&str) {
    let bin_path = format!("samples/artificial/{}", bin);
    b.iter(|| uaf(&[bin_path.clone()], Config::LOAD_ONLY).run_rules());
}

fn load_lift_benches(c: &mut Criterion) {
    let bins = &[
        "field_overwrite",
        "func",
        "link",
        "ll",
        "loop",
        "path_sensitive",
        "recurse",
        "reloop",
        "remalloc",
        "restale",
        "safe",
        "seq_call",
        "simple",
        "undef_edge",
        "undef_stack",
    ];
    for bin in bins {
        c.bench_with_input(BenchmarkId::new("lift_load", bin), bin, &load_lift_bin);
    }
}

criterion_group!(benches, load_lift_benches);
criterion_main!(benches);

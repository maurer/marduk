extern crate marduk;

use self::marduk::Config;
use super::ALIAS_MODES;
use measurement::Measurement;
use std::collections::BTreeMap;
use std::f64::EPSILON;

struct Stats {
    avg: f64,
    med: f64,
    stdev: f64,
}

impl Stats {
    fn print<P: Fn(f64) -> ()>(&self, p: P) {
        print!(" & ");
        p(self.avg);
        print!(" & ");
        p(self.med);
        print!(" & ");
        p(self.stdev);
    }
}

fn time_formatter(mut t: f64) {
    let h: usize = (t / 60.0 / 60.0).floor() as usize;
    t -= (h * 60 * 60) as f64;
    let m: usize = (t / 60.0).floor() as usize;
    t -= (m * 60) as f64;
    let s: f64 = t;
    if h > 0 {
        print!("{}h", h);
    }
    if m > 0 {
        print!("{}m", m);
    }
    print!("{:.1}s", s);
}

const KILO: f64 = 1024.0;
const MEGA: f64 = KILO * 1024.0;
const GIGA: f64 = MEGA * 1024.0;

fn space_formatter(t: f64) {
    if t > GIGA {
        print!("{:.1}G", t / GIGA);
    } else if t > MEGA {
        print!("{:.1}M", t / MEGA);
    } else {
        print!("{:.1}k", t / KILO);
    }
}

fn fp_formatter(t: f64) {
    if (t.floor() * 10.0 - (t * 10.0).floor()).abs() < EPSILON {
        print!("{}", t.floor() as usize);
    } else {
        print!("{:.1}", t);
    }
}

fn avg(dat: &[f64]) -> f64 {
    let sum: f64 = dat.iter().sum();
    sum / (dat.len() as f64)
}

fn stats(dat: &[f64]) -> Stats {
    let dat_avg = avg(dat);
    let sqs: Vec<_> = dat.iter().map(|d| (d - dat_avg).powi(2)).collect();
    let dat_stdev = avg(&sqs).sqrt();
    let mut sorting = dat.to_vec();
    sorting.sort_by(|x, y| x.partial_cmp(y).unwrap());
    let dat_med = if dat.len() % 2 == 0 {
        let left = (dat.len() - 1) / 2;
        let right = left + 1;
        (sorting[left] + sorting[right]) / 2.0
    } else {
        sorting[(dat.len() - 1) / 2]
    };
    Stats {
        avg: dat_avg,
        stdev: dat_stdev,
        med: dat_med,
    }
}

fn compare_modes(big: Config, small: Config, dat: &[Measurement]) {
    let mut bigs = BTreeMap::new();
    let mut smalls = BTreeMap::new();
    for m in dat {
        if m.mode == big {
            if m.false_positives != 0 {
                bigs.insert(m.artifact.clone(), m.false_positives);
            }
        } else if m.mode == small {
            smalls.insert(m.artifact.clone(), m.false_positives);
        }
    }
    let mut bugs_removed_prop: Vec<f64> = Vec::new();
    for k in bigs.keys() {
        if smalls.contains_key(k) {
            let big = bigs[k] as f64;
            let small = smalls[k] as f64;
            bugs_removed_prop.push((big - small) / big);
        }
    }
    println!(
        "{} -> {} power: {:.1}%",
        big,
        small,
        100.0 * avg(&bugs_removed_prop)
    );
}

pub fn post_analysis(dat: &[Measurement]) {
    println!("BEGIN_TABLE");
    for mode in ALIAS_MODES {
        let mut fps = Vec::new();
        let mut spaces = Vec::new();
        let mut times = Vec::new();
        for m in dat.iter().filter(|m| &m.mode == mode).cloned() {
            fps.push(m.false_positives as f64);
            spaces.push(m.space as f64);
            let time = m.time.as_secs() as f64 + (f64::from(m.time.subsec_nanos()) / 1_000_000.0);
            times.push(time);
        }
        let fp_stats = stats(&fps);
        let space_stats = stats(&spaces);
        let time_stats = stats(&times);
        print!("{} ", mode);
        time_stats.print(time_formatter);
        space_stats.print(space_formatter);
        fp_stats.print(fp_formatter);
        println!("\\\\ \\hline");
    }
    println!("END_TABLE");
    compare_modes(ALIAS_MODES[0], ALIAS_MODES[1], &dat);
}

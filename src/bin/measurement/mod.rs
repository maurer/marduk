extern crate marduk;
extern crate serde;

use self::marduk::Config;
use std::time::Duration;

#[derive(Clone, Eq, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct Measurement {
    pub mode: Config,
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

mod printers {
    use super::marduk::printers::fmt_vec;
    use super::Measurement;
    use std::fmt::{Display, Formatter, Result};
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

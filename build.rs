fn main() {
    for myc_file in &[
        "defs",
        "flow",
        "fmt_str",
        "load",
        "queries",
        "schema",
        "uaf",
        "steensgaard",
    ] {
        println!("cargo:rerun-if-changed=mycroft/{}.my", myc_file)
    }
}

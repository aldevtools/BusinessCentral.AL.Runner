//! The equality check of the workspace layer (`al_test_cli::workspace_equality`), with the dumps written to files. Exits
//! 1 when the read compile's dump differs from the fresh one or the third compile did not read the entry.
//!
//! `cargo run --release -p al-test-cli --example workspace_cache_equality -- ROOT MAJOR OUT_DIR [APP_DIR]`
//!
//! Run it twice to cover the thread counts: as is, and with `AL_CHECK_THREADS=1 AL_LOWER_THREADS=1`. With `APP_DIR`, one
//! app folder instead of the corpus apps (the measurement of issue 8143's app).

use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = PathBuf::from(&args[1]);
    let major: u16 = args[2].parse().expect("a BC major");
    let out = PathBuf::from(&args[3]);
    std::fs::create_dir_all(&out).unwrap();
    let app = args.get(4).map(PathBuf::from);
    let outcome =
        al_test_cli::workspace_equality::run(&root, major, &out.join("cache"), app.as_deref(), &mut |name, dump| {
            std::fs::write(out.join(format!("{name}.txt")), dump).unwrap()
        })
        .unwrap();
    for s in &outcome.steps {
        eprintln!(
            "{}: compile {:.3} s, workspace layer {:?}, dump {} lines",
            s.name,
            s.took.as_secs_f64(),
            s.report
                .as_ref()
                .map(|r| (r.state, r.bytes, r.open, r.store, r.notes.clone())),
            s.dump_lines
        );
    }
    match &outcome.difference {
        Some(d) => eprintln!("FAIL: the read dump differs from the fresh one at {d}"),
        None if outcome.ok() => eprintln!(
            "OK: the read compile's dump equals the fresh one ({} bytes)",
            outcome.bytes
        ),
        None => eprintln!("FAIL: the second compile did not build the entry the third read"),
    }
    std::process::exit(if outcome.ok() { 0 } else { 1 });
}

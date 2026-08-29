// SPDX-License-Identifier: GPL-2.0-or-later
//! Micro-benchmarks for the walker + text formatter. Baselines:
//! `cargo bench -- --save-baseline main`, then `-- --baseline main`.

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use criterion::{Criterion, criterion_group, criterion_main};

use utree::options::Options;
use utree::output::{Formatter, Totals, text::TextFormatter};
use utree::walk::Walker;

fn fixture(name: &str, build: impl Fn(&PathBuf)) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/bench-fixtures")
        .join(name);
    if !dir.join(".ready").exists() {
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        build(&dir);
        fs::write(dir.join(".ready"), b"").unwrap();
    }
    dir
}

fn wide() -> PathBuf {
    fixture("wide", |dir| {
        for i in 0..2000 {
            fs::write(dir.join(format!("file-{i:04}")), b"").unwrap();
        }
    })
}

fn deep() -> PathBuf {
    fixture("deep", |dir| {
        let mut current = dir.clone();
        for i in 0..256 {
            current = current.join(format!("d{i:03}"));
            fs::create_dir(&current).unwrap();
            fs::write(current.join("leaf"), b"").unwrap();
        }
    })
}

fn bushy() -> PathBuf {
    fixture("bushy", |dir| {
        fn level(dir: &std::path::Path, depth: u32) {
            for i in 0..4 {
                let sub = dir.join(format!("sub{i}"));
                fs::create_dir(&sub).unwrap();
                for j in 0..8 {
                    fs::write(sub.join(format!("f{j}")), b"").unwrap();
                }
                if depth > 0 {
                    level(&sub, depth - 1);
                }
            }
        }
        level(dir, 4);
    })
}

fn run(opts: &Options, root: &std::path::Path) -> Totals {
    let mut walker = Walker::new(opts).unwrap();
    let root = walker.walk_root(&OsString::from(root.as_os_str()));
    let totals = root.totals();
    let mut formatter = TextFormatter::new(opts);
    let mut sink = std::io::sink();
    formatter.emit_root(&mut sink, &root, opts, false).unwrap();
    formatter.emit_report(&mut sink, &totals, opts).unwrap();
    totals
}

fn bench_walk(c: &mut Criterion) {
    let opts = Options::default();
    for (name, path) in [("wide", wide()), ("deep", deep()), ("bushy", bushy())] {
        c.bench_function(name, |b| b.iter(|| run(&opts, &path)));
    }
}

criterion_group!(benches, bench_walk);
criterion_main!(benches);

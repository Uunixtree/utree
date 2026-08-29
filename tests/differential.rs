// SPDX-License-Identifier: GPL-2.0-or-later
//! Differential fuzzing: random trees and option combinations through
//! both binaries, requiring identical stdout and exit codes.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use proptest::prelude::*;

fn reference_tree() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("UTREE_REF_TREE") {
        return Some(PathBuf::from(path));
    }
    let default = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("reference/tree");
    default.is_file().then_some(default)
}

#[derive(Debug, Clone)]
enum Entry {
    File(String),
    Dir(String, Vec<Entry>),
    Symlink(String, String),
}

fn name_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-e.][a-e0-9. _-]{0,6}")
        .unwrap()
        .prop_filter("no . or .. or trailing space", |s| {
            s != "." && s != ".." && !s.ends_with(' ') && !s.ends_with('.')
        })
}

fn entry_strategy(depth: u32) -> BoxedStrategy<Entry> {
    let file = name_strategy().prop_map(Entry::File);
    let link = (name_strategy(), name_strategy()).prop_map(|(n, t)| Entry::Symlink(n, t));
    if depth == 0 {
        prop_oneof![4 => file, 1 => link].boxed()
    } else {
        let dir = (
            name_strategy(),
            proptest::collection::vec(entry_strategy(depth - 1), 0..5),
        )
            .prop_map(|(n, c)| Entry::Dir(n, c));
        prop_oneof![4 => file, 2 => dir, 1 => link].boxed()
    }
}

fn tree_strategy() -> impl Strategy<Value = Vec<Entry>> {
    proptest::collection::vec(entry_strategy(3), 0..8)
}

fn args_strategy() -> impl Strategy<Value = Vec<String>> {
    let flags = proptest::collection::vec(
        prop_oneof![
            Just("-a".to_string()),
            Just("-d".to_string()),
            Just("-f".to_string()),
            Just("-l".to_string()),
            Just("--prune".to_string()),
            Just("--noreport".to_string()),
            Just("--matchdirs".to_string()),
            Just("--ignore-case".to_string()),
            Just("-v".to_string()),
            Just("-t".to_string()),
            Just("-c".to_string()),
            Just("-r".to_string()),
            Just("-U".to_string()),
            Just("--dirsfirst".to_string()),
            Just("--filesfirst".to_string()),
            Just("--sort=size".to_string()),
            Just("--sort=version".to_string()),
            Just("-p".to_string()),
            Just("-s".to_string()),
            Just("-h".to_string()),
            Just("--si".to_string()),
            Just("-u".to_string()),
            Just("-g".to_string()),
            Just("-D".to_string()),
            Just("-F".to_string()),
            Just("-q".to_string()),
            Just("-Q".to_string()),
            Just("-N".to_string()),
            Just("--inodes".to_string()),
            Just("--device".to_string()),
            Just("--du".to_string()),
            Just("-C".to_string()),
            Just("-n".to_string()),
            Just("-i".to_string()),
            Just("-A".to_string()),
            Just("-S".to_string()),
            Just("-J".to_string()),
            Just("-X".to_string()),
            Just("--charset=Shift_JIS".to_string()),
            Just("--charset=EUC-JP".to_string()),
            Just("--charset=KOI8-R".to_string()),
            Just("--charset=latin1".to_string()),
        ],
        0..6,
    );
    let level = proptest::option::of((1u32..4).prop_map(|l| l.to_string()));
    let pattern = proptest::option::of(prop_oneof![
        Just("*.c".to_string()),
        Just("a*|b*".to_string()),
        Just("[a-c]?*".to_string()),
        Just("d*".to_string()),
    ]);
    let html = proptest::option::of(prop_oneof![
        Just(vec!["-H".to_string(), "http://x".to_string()]),
        Just(vec!["-H".to_string(), "-http://x/base".to_string()]),
        Just(vec![
            "-H".to_string(),
            "h".to_string(),
            "-T".to_string(),
            "T i&t<le".to_string()
        ]),
    ]);
    (flags, level, pattern, html).prop_map(|(mut args, level, pattern, html)| {
        if let Some(level) = level {
            args.push("-L".to_string());
            args.push(level);
        }
        if let Some(pattern) = pattern {
            args.push("-P".to_string());
            args.push(pattern);
        }
        if let Some(html) = html {
            args.extend(html);
        }
        args
    })
}

fn build(dir: &Path, entries: &[Entry]) {
    for entry in entries {
        match entry {
            Entry::File(name) => {
                // Vary sizes and permission bits for the size sort and
                // the -p/-F columns.
                let path = dir.join(name);
                if fs::write(&path, vec![b'x'; name.len() * 3 % 17]).is_ok() {
                    let mode = 0o400 | (name.len() as u32 * 0o111) & 0o377;
                    let _ = fs::set_permissions(
                        &path,
                        std::os::unix::fs::PermissionsExt::from_mode(mode),
                    );
                }
            }
            Entry::Dir(name, children) => {
                let path = dir.join(name);
                if fs::create_dir(&path).is_ok() {
                    build(&path, children);
                    // Sticky/other-writable variety for tw/ow/st colors.
                    let mode = [0o755, 0o777, 0o1777, 0o1755][name.len() % 4];
                    let _ = fs::set_permissions(
                        &path,
                        std::os::unix::fs::PermissionsExt::from_mode(mode),
                    );
                }
            }
            Entry::Symlink(name, target) => {
                let _ = std::os::unix::fs::symlink(target, dir.join(name));
            }
        }
    }
}

fn run(bin: &Path, args: &[String], cwd: &Path) -> (Vec<u8>, i32) {
    let output = Command::new(bin)
        .args(args)
        .arg(".")
        .current_dir(cwd)
        .env("LC_ALL", "C")
        .env("TREE_CHARSET", "UTF-8")
        .env("TERM", "xterm")
        .env(
            "LS_COLORS",
            "rs=0:di=01;34:ln=01;36:pi=40;33:so=01;35:or=40;31;01:mi=01;37;41:\
             ex=01;32:su=37;41:sg=30;43:tw=30;42:ow=34;42:st=37;44:*a=04;35:*.c=01;33",
        )
        .env_remove("TREE_COLORS")
        .env_remove("NO_COLOR")
        .env_remove("CLICOLOR")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("GIT_DIR")
        .output()
        .expect("failed to run binary");
    (output.stdout, output.status.code().unwrap_or(-1))
}

#[test]
fn differential_against_reference_tree() {
    let Some(reference) = reference_tree() else {
        eprintln!(
            "skipping: no reference tree (set UTREE_REF_TREE or run `make -C testsuite reference`)"
        );
        return;
    };
    let utree = PathBuf::from(env!("CARGO_BIN_EXE_utree"));
    let scratch = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/it-scratch")
        .join(format!("differential-{}", std::process::id()));

    let mut runner = proptest::test_runner::TestRunner::new(proptest::test_runner::Config {
        cases: std::env::var("UTREE_DIFF_CASES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(128),
        ..Default::default()
    });

    let result = runner.run(&(tree_strategy(), args_strategy()), |(entries, args)| {
        let _ = fs::remove_dir_all(&scratch);
        fs::create_dir_all(&scratch).unwrap();
        build(&scratch, &entries);

        let (ref_out, ref_code) = run(&reference, &args, &scratch);
        let (our_out, our_code) = run(&utree, &args, &scratch);

        prop_assert_eq!(
            String::from_utf8_lossy(&ref_out),
            String::from_utf8_lossy(&our_out),
            "stdout differs for args {:?}",
            args
        );
        prop_assert_eq!(ref_code, our_code, "exit code differs for args {:?}", args);
        Ok(())
    });
    let _ = fs::remove_dir_all(&scratch);

    if let Err(err) = result {
        panic!("differential test failed:\n{err}");
    }
}

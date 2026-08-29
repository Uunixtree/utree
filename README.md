# utree

A Rust reimplementation of the classic Unix [`tree`](https://github.com/Old-Man-Programmer/tree) command, in the spirit of [uutils](https://github.com/uutils/coreutils). What sets it apart from the C original is safety: utree is written entirely in safe Rust, handles file names as raw bytes so that non-UTF-8 names cannot crash it, and is backed by a differential testsuite and fuzzer — a tree that is safe for its users.

The behavior of a pinned reference build of tree is the specification, verified mechanically. For supported options the output is byte-identical to the reference; the few options utree does not implement fail with an explicit error rather than silently producing different output. See the man page for the option list and [COMPATIBILITY.md](COMPATIBILITY.md) for where the reference is pinned and every known difference.

## Building and testing

```console
$ cargo build --release
$ cargo test                      # unit + integration tests
$ make -C testsuite check         # the compat suite
```

All compatibility checking is differential: the specification is the pinned reference binary itself. The suite in [`testsuite/`](testsuite/) runs every case under the reference tree and under utree and diffs the two output directories.

Without the reference binary, `cargo test` skips the differential parts and still runs the unit and integration tests.

## Fuzzing

A differential fuzzer generates random directory trees and option combinations, runs them through both binaries, and requires identical output.

```console
$ cargo test --test differential  # 128 cases (UTREE_DIFF_CASES=N)
```

## Benchmarks

Performance is tracked with criterion micro-benchmarks (`cargo bench`, baselines via `--save-baseline`) and `scripts/bench-compare.sh`, which compares wall-clock (hyperfine) and syscall counts (strace) against the reference tree on deterministic fixtures.

## License

GPL-2.0-or-later, the same license as tree. See [LICENSE](LICENSE).

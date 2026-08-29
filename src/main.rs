// SPDX-License-Identifier: GPL-2.0-or-later
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::io::{BufWriter, Write};
use std::process::ExitCode;

use utree::options::{self, Options, OutputFormat};
use utree::output::html::HtmlFormatter;
use utree::output::json::JsonFormatter;
use utree::output::text::TextFormatter;
use utree::output::xml::XmlFormatter;
use utree::output::{Formatter, Totals};
use utree::walk::{self, Node, RootKind, join_path};

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let opts = match options::parse(&args) {
        Ok(opts) => opts,
        Err(options::CliError::Usage(msg)) => {
            eprintln!("{msg}");
            return ExitCode::from(1);
        }
        Err(options::CliError::Help(msg)) => {
            println!("{msg}");
            return ExitCode::SUCCESS;
        }
    };

    match run(&opts) {
        Ok(errors) => {
            if errors > 0 {
                ExitCode::from(2)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(err) => {
            eprintln!("utree: {err}");
            ExitCode::from(2)
        }
    }
}

/// Returns the number of runtime errors, which drives the exit status
/// like tree's `errors` counter.
fn run(opts: &Options) -> std::io::Result<u64> {
    let Some(mut walker) = walk::Walker::new(opts) else {
        eprintln!("utree: Could not load gitignore file");
        std::process::exit(1);
    };
    run_paths(&mut walker, opts, &opts.paths, opts.output_file.clone())?;
    Ok(walker.errors())
}

fn run_paths(
    walker: &mut walk::Walker,
    opts: &Options,
    paths: &[OsString],
    out_file: Option<OsString>,
) -> std::io::Result<()> {
    // The output file is created before the walk, like tree's
    // setoutput(): a -R sub-listing therefore sees its own 00Tree.html.
    let out: Box<dyn Write> = match &out_file {
        Some(path) => Box::new(std::fs::File::create(path)?),
        None => Box::new(std::io::stdout().lock()),
    };
    let mut out = BufWriter::new(out);

    let mut roots = Vec::new();
    for path in paths {
        roots.push(walker.walk_root(path));
    }

    let mut totals = Totals::default();
    for root in &roots {
        totals.add(&root.totals());
    }

    let mut formatter: Box<dyn Formatter> = match opts.output {
        OutputFormat::Text => Box::new(TextFormatter::new(opts)),
        OutputFormat::Json => Box::new(JsonFormatter::new(opts)),
        OutputFormat::Xml => Box::new(XmlFormatter::new(opts)),
        OutputFormat::Html => Box::new(HtmlFormatter::new(opts)),
    };
    formatter.intro(&mut out, opts)?;
    for (i, root) in roots.iter().enumerate() {
        formatter.emit_root(&mut out, root, opts, i + 1 < roots.len())?;
    }
    if !opts.no_report {
        formatter.emit_report(&mut out, &totals, opts)?;
    }
    formatter.outro(&mut out, opts)?;
    out.flush()?;
    drop(out);

    // -R: every boundary directory gets its own full run written to
    // <dir>/00Tree.html, recursively (tree re-runs emit_tree there).
    if opts.rerun {
        let mut boundaries = Vec::new();
        for root in &roots {
            if let RootKind::Opened(children) = &root.kind {
                collect_boundaries(&root.name, children, &mut boundaries);
            }
        }
        for boundary in boundaries {
            use std::os::unix::ffi::OsStrExt;
            let path = OsString::from(std::ffi::OsStr::from_bytes(&boundary));
            let out = OsString::from(std::ffi::OsStr::from_bytes(&join_path(
                &boundary,
                b"00Tree.html",
            )));
            run_paths(walker, opts, &[path], Some(out))?;
        }
    }
    Ok(())
}

fn collect_boundaries(parent: &[u8], nodes: &[Node], out: &mut Vec<Vec<u8>>) {
    for node in nodes {
        let path = join_path(parent, &node.name);
        if node.rerun_link {
            out.push(path.clone());
        }
        if let Some(children) = &node.children {
            collect_boundaries(&path, children, out);
        }
    }
}

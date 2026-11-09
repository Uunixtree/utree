// SPDX-License-Identifier: GPL-2.0-or-later
//! Hand-written CLI parser mirroring tree's argument handling
//! (combinable short flags, `--opt=value` and `--opt value`).

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum SortKey {
    #[default]
    Name,
    Version,
    Size,
    Mtime,
    Ctime,
    /// -U / --sort=none: keep readdir order.
    Unsorted,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum TopSort {
    #[default]
    None,
    DirsFirst,
    FilesFirst,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    #[default]
    Text,
    Json, // -J
    Xml,  // -X
    Html, // -H
}

#[derive(Debug, Default)]
pub struct Options {
    pub paths: Vec<OsString>,
    pub all_files: bool,               // -a
    pub dirs_only: bool,               // -d
    pub full_path: bool,               // -f
    pub follow_links: bool,            // -l
    pub xdev: bool,                    // -x
    pub level: Option<usize>,          // -L
    pub file_limit: Option<i64>,       // --filelimit
    pub no_report: bool,               // --noreport
    pub output_file: Option<OsString>, // -o
    pub patterns: Vec<Vec<u8>>,        // -P
    pub ipatterns: Vec<Vec<u8>>,       // -I
    pub sort: SortKey,
    pub top: TopSort,
    pub reverse: bool, // -r
    /// -c: sort by ctime and make -D display ctime (tree's flag.c).
    pub use_ctime: bool,
    pub show_perms: bool, // -p
    pub show_uid: bool,   // -u
    pub show_gid: bool,   // -g
    pub show_size: bool,  // -s
    pub human: bool,      // -h
    pub si: bool,         // --si
    pub show_date: bool,  // -D
    pub timefmt: Option<Vec<u8>>,
    pub classify: bool,    // -F
    pub qmark: bool,       // -q
    pub no_escape: bool,   // -N
    pub quote: bool,       // -Q
    pub show_inode: bool,  // --inodes
    pub show_device: bool, // --device
    pub du: bool,          // --du
    pub force_color: bool, // -C
    pub no_color: bool,    // -n
    pub noindent: bool,    // -i
    pub ansilines: bool,   // -A
    pub output: OutputFormat,
    /// -H's baseHREF; a leading '-' strips the local path prefix.
    pub host: Option<Vec<u8>>,
    pub htmloffset: bool,
    pub title: Option<Vec<u8>>,   // -T
    pub nolinks: bool,            // --nolinks
    pub hintro: Option<OsString>, // --hintro
    pub houtro: Option<OsString>, // --houtro
    pub rerun: bool,              // -R
    pub ignore_case: bool,
    pub prune: bool,
    pub matchdirs: bool,
    pub gitignore: bool,
    /// --gitfile: an explicit file in .gitignore syntax (implies
    /// --gitignore).
    pub gitfile: Option<OsString>,
    pub info: bool,
    pub infofile: Option<OsString>,
    pub charset: Option<Vec<u8>>,
}

pub enum CliError {
    /// Message for stderr; exit status 1.
    Usage(String),
    /// Message for stdout; exit status 0 (--help, --version).
    Help(String),
}

const VERSION_TEXT: &str = concat!(
    "utree v",
    env!("CARGO_PKG_VERSION"),
    " - a Rust reimplementation of tree"
);

const USAGE_TEXT: &str = "usage: utree [-acdfghilnpqrstuvxACDFJNQRSUX] [-L level] [-P pattern]\n\
             \t[-I pattern] [-o filename] [-H baseHREF] [-T title] [--noreport]\n\
             \t[--charset[=]X] [--filelimit[=]#] [--ignore-case] [--matchdirs]\n\
             \t[--gitignore] [--info] [--infofile[=]file] [--prune] [--inodes]\n\
             \t[--device] [--du] [--si] [--timefmt[=]fmt] [--sort[=]X]\n\
             \t[--dirsfirst] [--filesfirst] [--nolinks] [--hintro[=]file]\n\
             \t[--houtro[=]file] [--help] [--version] [--] [directory ...]";

/// tree options not implemented yet: an explicit error, never a no-op.
const UNSUPPORTED_SHORT: &[u8] = b"";
const UNSUPPORTED_LONG: &[&str] = &[
    "--fromfile",
    "--fromtabfile",
    "--metafirst",
    "--condense",
    "--compress",
    "--hyperlink",
    "--scheme",
    "--authority",
    "--acl",
    "--selinux",
    "--fflinks",
    "--opt-toggle",
];

pub fn parse(args: &[OsString]) -> Result<Options, CliError> {
    let mut opts = Options::default();
    let mut only_paths = false;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let bytes = arg.as_bytes();

        if only_paths || !bytes.starts_with(b"-") || bytes == b"-" {
            opts.paths.push(arg.clone());
            continue;
        }

        if bytes == b"--" {
            only_paths = true;
            continue;
        }

        if bytes.starts_with(b"--") {
            parse_long(arg, &mut iter, &mut opts)?;
        } else {
            parse_short(arg, &mut iter, &mut opts)?;
        }
    }

    if opts.paths.is_empty() {
        opts.paths.push(OsString::from("."));
    }
    // tree.c: "if (flag.d) flag.prune = false;"
    if opts.dirs_only {
        opts.prune = false;
    }
    // tree.c: "if (basesort == NULL) topsort = NULL;" (-U wins)
    if opts.sort == SortKey::Unsorted {
        opts.top = TopSort::None;
    }
    // tree.c: "if (flag.R && (Level == -1)) flag.R = false;"
    if opts.level.is_none() {
        opts.rerun = false;
    }
    Ok(opts)
}

fn parse_long(
    arg: &OsString,
    iter: &mut std::slice::Iter<'_, OsString>,
    opts: &mut Options,
) -> Result<(), CliError> {
    let bytes = arg.as_bytes();
    let (name, inline) = match bytes.iter().position(|&b| b == b'=') {
        Some(eq) => (&bytes[..eq], Some(bytes[eq + 1..].to_vec())),
        None => (bytes, None),
    };
    let value = |iter: &mut std::slice::Iter<'_, OsString>| -> Result<Vec<u8>, CliError> {
        match inline.clone() {
            // long_arg: an empty `--opt=` is a missing argument.
            Some(v) if v.is_empty() => Err(CliError::Usage(format!(
                "utree: Missing argument to {}=",
                String::from_utf8_lossy(name)
            ))),
            Some(v) => Ok(v),
            None => iter
                .next()
                .map(|v| v.as_bytes().to_vec())
                .ok_or_else(|| missing_argument(&String::from_utf8_lossy(name))),
        }
    };

    match name {
        b"--help" => {
            return Err(CliError::Help(format!("{VERSION_TEXT}\n{USAGE_TEXT}")));
        }
        b"--version" => return Err(CliError::Help(VERSION_TEXT.to_string())),
        b"--noreport" => opts.no_report = true,
        b"--inodes" => opts.show_inode = true,
        b"--device" => opts.show_device = true,
        b"--si" => {
            opts.si = true;
            opts.human = true;
            opts.show_size = true;
        }
        b"--du" => {
            opts.du = true;
            opts.show_size = true;
        }
        b"--timefmt" => {
            opts.timefmt = Some(value(iter)?);
            opts.show_date = true;
        }
        b"--nolinks" => opts.nolinks = true,
        b"--hintro" => opts.hintro = Some(OsStr::from_bytes(&value(iter)?).to_os_string()),
        b"--houtro" => opts.houtro = Some(OsStr::from_bytes(&value(iter)?).to_os_string()),
        b"--dirsfirst" => opts.top = TopSort::DirsFirst,
        b"--filesfirst" => opts.top = TopSort::FilesFirst,
        b"--sort" => {
            let v = value(iter)?;
            opts.sort = match v.to_ascii_lowercase().as_slice() {
                b"name" => SortKey::Name,
                b"version" => SortKey::Version,
                b"size" => SortKey::Size,
                b"mtime" => SortKey::Mtime,
                b"ctime" => SortKey::Ctime,
                b"none" => SortKey::Unsorted,
                _ => {
                    return Err(CliError::Usage(format!(
                        "utree: Sort type '{}' not valid, should be one of: \
                         name,version,size,mtime,ctime,none",
                        String::from_utf8_lossy(&v)
                    )));
                }
            };
            opts.use_ctime = opts.sort == SortKey::Ctime;
        }
        b"--prune" => opts.prune = true,
        b"--matchdirs" => opts.matchdirs = true,
        b"--gitignore" => opts.gitignore = true,
        b"--gitfile" => {
            opts.gitignore = true;
            opts.gitfile = Some(OsStr::from_bytes(&value(iter)?).to_os_string());
        }
        b"--info" => opts.info = true,
        b"--infofile" => {
            opts.infofile = Some(OsStr::from_bytes(&value(iter)?).to_os_string());
        }
        b"--ignore-case" => opts.ignore_case = true,
        b"--filelimit" => {
            // atoi: garbage parses to 0, and a non-positive limit
            // disables the check entirely (flag.flimit > 0).
            let limit = atoi(&value(iter)?);
            opts.file_limit = (limit > 0).then_some(limit);
        }
        b"--charset" => opts.charset = Some(value(iter)?),
        _ => {
            let display = String::from_utf8_lossy(name).into_owned();
            if UNSUPPORTED_LONG.contains(&display.as_str()) {
                return Err(not_supported(&display));
            }
            return Err(CliError::Usage(format!(
                "utree: Invalid argument `{display}'.\n{USAGE_TEXT}"
            )));
        }
    }
    Ok(())
}

/// C strtoul(s, NULL, 0): optional whitespace, then a base prefix
/// (0x hex, 0 octal), then digits; 0 when nothing parses.
fn strtoul0(s: &[u8]) -> u64 {
    let ws = s.iter().take_while(|c| c.is_ascii_whitespace()).count();
    let s = &s[ws..];
    let (digits, radix) = if let Some(hex) = s.strip_prefix(b"0x").or_else(|| s.strip_prefix(b"0X"))
    {
        (hex, 16)
    } else if s.len() > 1 && s[0] == b'0' {
        (&s[1..], 8)
    } else {
        (s, 10)
    };
    let mut value: u64 = 0;
    for &c in digits {
        let Some(d) = (c as char).to_digit(radix) else {
            break;
        };
        value = value.wrapping_mul(radix as u64).wrapping_add(d as u64);
    }
    value
}

/// C atoi(): optional whitespace and sign, decimal digits, else 0.
fn atoi(s: &[u8]) -> i64 {
    let ws = s.iter().take_while(|c| c.is_ascii_whitespace()).count();
    let s = &s[ws..];
    let (s, neg) = match s.first() {
        Some(b'-') => (&s[1..], true),
        Some(b'+') => (&s[1..], false),
        _ => (s, false),
    };
    let mut value: i64 = 0;
    for &c in s {
        if !c.is_ascii_digit() {
            break;
        }
        value = value.wrapping_mul(10).wrapping_add((c - b'0') as i64);
    }
    if neg { -value } else { value }
}

fn parse_level(text: &[u8]) -> Result<usize, CliError> {
    // tree: Level = strtoul(sLevel, NULL, 0) - 1; error when < 0.
    let level = strtoul0(text);
    if level == 0 {
        return Err(CliError::Usage(
            "utree: Invalid level, must be greater than 0.".to_string(),
        ));
    }
    Ok(level as usize)
}

fn parse_short(
    arg: &OsString,
    iter: &mut std::slice::Iter<'_, OsString>,
    opts: &mut Options,
) -> Result<(), CliError> {
    let bytes = arg.as_bytes();
    let mut j = 1;
    while j < bytes.len() {
        let flag = bytes[j];
        j += 1;
        let value = |iter: &mut std::slice::Iter<'_, OsString>| -> Result<OsString, CliError> {
            iter.next()
                .cloned()
                .ok_or_else(|| missing_argument(&format!("-{}", flag as char)))
        };

        match flag {
            b'a' => opts.all_files = true,
            b'd' => opts.dirs_only = true,
            b'p' => opts.show_perms = true,
            b'u' => opts.show_uid = true,
            b'g' => opts.show_gid = true,
            b's' => opts.show_size = true,
            b'h' => {
                // tree.c: "Assume they also want -s".
                opts.human = true;
                opts.show_size = true;
            }
            b'D' => opts.show_date = true,
            b'C' => opts.force_color = true,
            b'J' => opts.output = OutputFormat::Json,
            b'X' => opts.output = OutputFormat::Xml,
            b'H' => {
                opts.output = OutputFormat::Html;
                let mut host = value(iter)?.as_bytes().to_vec();
                if host.first() == Some(&b'-') {
                    opts.htmloffset = true;
                    host.remove(0);
                }
                opts.host = Some(host);
            }
            b'T' => opts.title = Some(value(iter)?.as_bytes().to_vec()),
            b'R' => opts.rerun = true,
            b'n' => opts.no_color = true,
            b'i' => opts.noindent = true,
            b'A' => opts.ansilines = true,
            b'S' => opts.charset = Some(b"IBM437".to_vec()),
            b'F' => opts.classify = true,
            b'q' => opts.qmark = true,
            b'N' => opts.no_escape = true,
            b'Q' => opts.quote = true,
            b'v' => opts.sort = SortKey::Version,
            b't' => opts.sort = SortKey::Mtime,
            b'c' => {
                opts.sort = SortKey::Ctime;
                opts.use_ctime = true;
            }
            b'U' => opts.sort = SortKey::Unsorted,
            b'r' => opts.reverse = true,
            b'f' => opts.full_path = true,
            b'l' => opts.follow_links = true,
            b'x' => opts.xdev = true,
            b'L' => {
                // Attached digits are consumed in place (-L2d == -L 2 -d);
                // otherwise the next argument is taken.
                let digits = bytes[j..].iter().take_while(|c| c.is_ascii_digit()).count();
                if digits > 0 {
                    opts.level = Some(parse_level(&bytes[j..j + digits])?);
                    j += digits;
                } else {
                    opts.level = Some(parse_level(value(iter)?.as_bytes())?);
                }
            }
            b'P' => opts.patterns.push(value(iter)?.as_bytes().to_vec()),
            b'I' => opts.ipatterns.push(value(iter)?.as_bytes().to_vec()),
            b'o' => opts.output_file = Some(value(iter)?),
            _ => {
                if UNSUPPORTED_SHORT.contains(&flag) {
                    return Err(not_supported(&format!("-{}", flag as char)));
                }
                return Err(CliError::Usage(format!(
                    "utree: Invalid argument -`{}'.\n{USAGE_TEXT}",
                    flag as char
                )));
            }
        }
    }
    Ok(())
}

fn missing_argument(option: &str) -> CliError {
    CliError::Usage(format!("utree: Missing argument to {option} option."))
}

fn not_supported(option: &str) -> CliError {
    CliError::Usage(format!(
        "utree: tree option {option} is not supported yet (see COMPATIBILITY.md)."
    ))
}

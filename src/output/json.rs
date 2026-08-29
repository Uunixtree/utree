// SPDX-License-Identifier: GPL-2.0-or-later
//! -J backend: a port of tree's json.c.

use std::io::{self, Write};

use super::meta::{self, Ids};
use super::{Formatter, Totals};
use crate::options::Options;
use crate::walk::{Meta, Node, Root, RootKind, join_path};

pub struct JsonFormatter {
    ids: Ids,
    now: i64,
}

fn type_of(mode: u32) -> &'static str {
    match mode & 0o170000 {
        0o100000 => "file",
        0o040000 => "directory",
        0o120000 => "link",
        0o020000 => "char",
        0o060000 => "block",
        0o140000 => "socket",
        0o010000 => "fifo",
        _ => "unknown",
    }
}

/// json_encode(): C0 controls as \b\t\n\f\r or \u, quote and backslash
/// escaped, everything else (bytes >= 128 included) raw.
fn encode(out: &mut dyn Write, s: &[u8]) -> io::Result<()> {
    const CTRL: &[u8; 32] = b"0-------btn-fr------------------";
    for &c in s {
        if c < 32 {
            if CTRL[c as usize] != b'-' {
                out.write_all(&[b'\\', CTRL[c as usize]])?;
            } else {
                write!(out, "\\u{c:04x}")?;
            }
        } else if c == b'"' || c == b'\\' {
            out.write_all(&[b'\\', c])?;
        } else {
            out.write_all(&[c])?;
        }
    }
    Ok(())
}

impl JsonFormatter {
    pub fn new(_opts: &Options) -> Self {
        JsonFormatter {
            ids: Ids::default(),
            now: super::now_epoch(),
        }
    }

    fn nl<'a>(&self, opts: &Options) -> &'a str {
        if opts.noindent { "" } else { "\n" }
    }

    fn indent(&self, out: &mut dyn Write, level: i32, opts: &Options) -> io::Result<()> {
        if opts.noindent {
            return Ok(());
        }
        // json_indent(-1) still prints its one leading unit.
        for _ in 0..=level.max(0) {
            out.write_all(b"    ")?;
        }
        Ok(())
    }

    fn fillinfo(&mut self, out: &mut dyn Write, m: &Meta, opts: &Options) -> io::Result<()> {
        if opts.show_inode {
            write!(out, ",\"inode\":{}", m.fino)?;
        }
        if opts.show_device {
            write!(out, ",\"dev\":{}", m.fdev as i32)?;
        }
        if opts.show_perms {
            write!(out, ",\"mode\":\"{:04o}\",\"prot\":\"", m.mode & 0o7777)?;
            out.write_all(&meta::prot(m.mode))?;
            out.write_all(b"\"")?;
        }
        if opts.show_uid {
            out.write_all(b",\"user\":\"")?;
            out.write_all(&self.ids.user(m.uid))?;
            out.write_all(b"\"")?;
        }
        if opts.show_gid {
            out.write_all(b",\"group\":\"")?;
            out.write_all(&self.ids.group(m.gid))?;
            out.write_all(b"\"")?;
        }
        if opts.show_size {
            if opts.human || opts.si {
                let sized = meta::psize(m.size, opts);
                let trimmed: Vec<u8> = sized
                    .iter()
                    .skip_while(|c| c.is_ascii_whitespace())
                    .copied()
                    .collect();
                out.write_all(b",\"size\":\"")?;
                out.write_all(&trimmed)?;
                out.write_all(b"\"")?;
            } else {
                write!(out, ",\"size\":{}", m.size)?;
            }
        }
        if opts.show_date {
            let t = if opts.use_ctime { m.ctime } else { m.mtime };
            out.write_all(b",\"time\":\"")?;
            out.write_all(&meta::do_date(t, opts, self.now))?;
            out.write_all(b"\"")?;
        }
        Ok(())
    }

    fn emit_node(
        &mut self,
        out: &mut dyn Write,
        node: &Node,
        parent: &[u8],
        level: i32,
        more: bool,
        opts: &Options,
    ) -> io::Result<()> {
        self.indent(out, level, opts)?;
        write!(out, "{{\"type\":\"{}\"", type_of(node.meta.mode))?;
        out.write_all(b",\"name\":\"")?;
        if opts.full_path {
            encode(out, &join_path(parent, &node.name))?;
        } else {
            encode(out, &node.name)?;
        }
        out.write_all(b"\"")?;
        if let Some(comment) = &node.comment {
            out.write_all(b",\"info\":\"")?;
            for (i, line) in comment.iter().enumerate() {
                encode(out, line)?;
                if i + 1 < comment.len() {
                    out.write_all(b"\\n")?;
                }
            }
            out.write_all(b"\"")?;
        }
        if let Some(target) = &node.link {
            out.write_all(b",\"target\":\"")?;
            encode(out, target)?;
            out.write_all(b"\"")?;
        }
        self.fillinfo(out, &node.meta, opts)?;

        let descend = node.children.is_some();
        let contents = descend || node.err.is_some();
        if contents {
            out.write_all(b",\"contents\":[")?;
        } else {
            out.write_all(b"}")?;
        }
        if let Some(err) = &node.err {
            write!(out, "{{\"error\": \"{err}\"}}")?;
        }

        if descend {
            write!(out, "{}", self.nl(opts))?;
            let children = node.children.as_deref().unwrap_or(&[]);
            let child_parent = join_path(parent, &node.name);
            for (i, child) in children.iter().enumerate() {
                self.emit_node(
                    out,
                    child,
                    &child_parent,
                    level + 1,
                    i + 1 < children.len(),
                    opts,
                )?;
            }
        }
        if contents {
            // close: lc.close(descend? lev : -1) — and C's descend is
            // also -1 (truthy) for recursive symlinks outside
            // full-tree mode, so those close at the entry's level.
            let full_tree = opts.prune || opts.matchdirs || opts.du;
            let recursive = node.err.as_deref() == Some("recursive, not followed") && !full_tree;
            self.indent(out, if descend || recursive { level } else { -1 }, opts)?;
            write!(out, "]}}{}{}", if more { "," } else { "" }, self.nl(opts))?;
        } else {
            write!(out, "{}{}", if more { "," } else { "" }, self.nl(opts))?;
        }
        Ok(())
    }
}

impl Formatter for JsonFormatter {
    fn intro(&mut self, out: &mut dyn Write, opts: &Options) -> io::Result<()> {
        write!(out, "[{}", self.nl(opts))
    }

    fn outro(&mut self, out: &mut dyn Write, opts: &Options) -> io::Result<()> {
        write!(out, "{}", self.nl(opts))?;
        out.write_all(b"]\n")
    }

    fn emit_root(
        &mut self,
        out: &mut dyn Write,
        root: &Root,
        opts: &Options,
        more_roots: bool,
    ) -> io::Result<()> {
        self.indent(out, 0, opts)?;
        let mode = root.meta.as_ref().map_or(0, |m| m.mode);
        write!(out, "{{\"type\":\"{}\"", type_of(mode))?;
        out.write_all(b",\"name\":\"")?;
        encode(out, &root.name)?;
        out.write_all(b"\"")?;
        if let Some(m) = &root.meta {
            self.fillinfo(out, m, opts)?;
        }

        match &root.kind {
            RootKind::Missing | RootKind::Unreadable => {
                out.write_all(b",\"contents\":[")?;
                write!(out, "{{\"error\": \"error opening dir\"}}")?;
                write!(out, "{}", self.nl(opts))?;
                self.indent(out, 0, opts)?;
                write!(
                    out,
                    "]}}{}{}",
                    if more_roots { "," } else { "" },
                    self.nl(opts)
                )?;
            }
            RootKind::OverLimit(entries) => {
                out.write_all(b",\"contents\":[")?;
                write!(
                    out,
                    "{{\"error\": \"{entries} entries exceeds filelimit, not opening dir\"}}"
                )?;
                write!(out, "{}", self.nl(opts))?;
                self.indent(out, 0, opts)?;
                write!(
                    out,
                    "]}}{}{}",
                    if more_roots { "," } else { "" },
                    self.nl(opts)
                )?;
            }
            RootKind::Opened(children) if children.is_empty() => {
                write!(
                    out,
                    "}}{}{}",
                    if more_roots { "," } else { "" },
                    self.nl(opts)
                )?;
            }
            RootKind::Opened(children) => {
                out.write_all(b",\"contents\":[")?;
                write!(out, "{}", self.nl(opts))?;
                for (i, child) in children.iter().enumerate() {
                    self.emit_node(out, child, &root.name, 1, i + 1 < children.len(), opts)?;
                }
                self.indent(out, 0, opts)?;
                write!(
                    out,
                    "]}}{}{}",
                    if more_roots { "," } else { "" },
                    self.nl(opts)
                )?;
            }
        }
        Ok(())
    }

    fn emit_report(
        &mut self,
        out: &mut dyn Write,
        totals: &Totals,
        opts: &Options,
    ) -> io::Result<()> {
        out.write_all(b",")?;
        self.indent(out, 0, opts)?;
        out.write_all(b"{\"type\":\"report\"")?;
        if opts.du {
            write!(out, ",\"size\":{}", totals.size)?;
        }
        write!(out, ",\"directories\":{}", totals.dirs)?;
        if !opts.dirs_only {
            write!(out, ",\"files\":{}", totals.files)?;
        }
        out.write_all(b"}")?;
        Ok(())
    }
}

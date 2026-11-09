// SPDX-License-Identifier: GPL-2.0-or-later
//! -X backend: a port of tree's xml.c.

use std::io::{self, Write};

use super::meta::{self, Ids};
use super::{Formatter, Totals, linedraw};
use crate::options::Options;
use crate::walk::{Meta, Node, Root, RootKind, join_path};

pub struct XmlFormatter {
    ids: Ids,
    now: i64,
}

fn tag_of(mode: u32) -> &'static str {
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

/// html_encode(), which xml.c reuses for attribute values.
pub fn html_encode(out: &mut dyn Write, s: &[u8]) -> io::Result<()> {
    for &c in s {
        match c {
            b'<' => out.write_all(b"&lt;")?,
            b'>' => out.write_all(b"&gt;")?,
            b'&' => out.write_all(b"&amp;")?,
            b'"' => out.write_all(b"&quot;")?,
            _ => out.write_all(&[c])?,
        }
    }
    Ok(())
}

impl XmlFormatter {
    pub fn new(_opts: &Options) -> Self {
        XmlFormatter {
            ids: Ids::default(),
            now: super::now_epoch(),
        }
    }

    fn nl<'a>(&self, opts: &Options) -> &'a str {
        if opts.noindent { "" } else { "\n" }
    }

    fn indent(&self, out: &mut dyn Write, level: i32, opts: &Options) -> io::Result<()> {
        if opts.noindent || level < 0 {
            return Ok(());
        }
        for _ in 0..=level {
            out.write_all(b"    ")?;
        }
        Ok(())
    }

    fn fillinfo(&mut self, out: &mut dyn Write, m: &Meta, opts: &Options) -> io::Result<()> {
        if opts.show_inode {
            write!(out, " inode=\"{}\"", m.fino)?;
        }
        if opts.show_device {
            write!(out, " dev=\"{}\"", m.fdev as i32)?;
        }
        if opts.show_perms {
            write!(out, " mode=\"{:04o}\" prot=\"", m.mode & 0o7777)?;
            out.write_all(&meta::prot(m.mode))?;
            out.write_all(b"\"")?;
        }
        if opts.show_uid {
            out.write_all(b" user=\"")?;
            out.write_all(&self.ids.user(m.uid))?;
            out.write_all(b"\"")?;
        }
        if opts.show_gid {
            out.write_all(b" group=\"")?;
            out.write_all(&self.ids.group(m.gid))?;
            out.write_all(b"\"")?;
        }
        if opts.show_size {
            write!(out, " size=\"{}\"", m.size)?;
        }
        if opts.show_date {
            let t = if opts.use_ctime { m.ctime } else { m.mtime };
            out.write_all(b" time=\"")?;
            html_encode(out, &meta::do_date(t, opts, self.now))?;
            out.write_all(b"\"")?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn open_tag(
        &mut self,
        out: &mut dyn Write,
        tag: &str,
        name: &[u8],
        node: Option<&Node>,
        meta_: Option<&Meta>,
        level: i32,
        opts: &Options,
    ) -> io::Result<()> {
        self.indent(out, level, opts)?;
        write!(out, "<{tag}")?;
        out.write_all(b" name=\"")?;
        html_encode(out, name)?;
        out.write_all(b"\"")?;
        if let Some(node) = node {
            if let Some(comment) = &node.comment {
                out.write_all(b" info=\"")?;
                for (i, line) in comment.iter().enumerate() {
                    html_encode(out, line)?;
                    if i + 1 < comment.len() {
                        write!(out, "{}", self.nl(opts))?;
                    }
                }
                out.write_all(b"\"")?;
            }
            if let Some(target) = &node.link {
                out.write_all(b" target=\"")?;
                html_encode(out, target)?;
                out.write_all(b"\"")?;
            }
        }
        if let Some(m) = meta_ {
            self.fillinfo(out, m, opts)?;
        }
        out.write_all(b">")?;
        Ok(())
    }

    fn close_tag(
        &mut self,
        out: &mut dyn Write,
        tag: &str,
        level: i32,
        opts: &Options,
    ) -> io::Result<()> {
        self.indent(out, level, opts)?;
        write!(out, "</{tag}>{}", self.nl(opts))
    }

    fn emit_node(
        &mut self,
        out: &mut dyn Write,
        node: &Node,
        parent: &[u8],
        level: i32,
        opts: &Options,
    ) -> io::Result<()> {
        let tag = tag_of(node.meta.mode);
        let full;
        let display: &[u8] = if opts.full_path {
            full = join_path(parent, &node.name);
            &full
        } else {
            &node.name
        };
        self.open_tag(out, tag, display, Some(node), Some(&node.meta), level, opts)?;
        if let Some(err) = &node.err {
            write!(out, "<error>{err}</error>")?;
        }
        if let Some(children) = &node.children {
            write!(out, "{}", self.nl(opts))?;
            let child_parent = join_path(parent, &node.name);
            for child in children {
                self.emit_node(out, child, &child_parent, level + 1, opts)?;
            }
            self.close_tag(out, tag, level, opts)?;
        } else {
            let full_tree = opts.prune || opts.matchdirs || opts.du;
            let recursive = node.err.as_deref() == Some("recursive, not followed") && !full_tree;
            self.close_tag(out, tag, if recursive { level } else { -1 }, opts)?;
        }
        Ok(())
    }
}

impl Formatter for XmlFormatter {
    fn intro(&mut self, out: &mut dyn Write, opts: &Options) -> io::Result<()> {
        out.write_all(b"<?xml version=\"1.0\"")?;
        if let Some(charset) = linedraw::charset_name(opts) {
            out.write_all(b" encoding=\"")?;
            out.write_all(&charset)?;
            out.write_all(b"\"")?;
        }
        write!(out, "?>{}<tree>{}", self.nl(opts), self.nl(opts))
    }

    fn outro(&mut self, out: &mut dyn Write, opts: &Options) -> io::Result<()> {
        write!(out, "</tree>{}", self.nl(opts))
    }

    fn emit_root(
        &mut self,
        out: &mut dyn Write,
        root: &Root,
        opts: &Options,
        _more_roots: bool,
    ) -> io::Result<()> {
        let tag = root.meta.as_ref().map_or("unknown", |m| tag_of(m.mode));
        self.open_tag(out, tag, &root.name, None, root.meta.as_ref(), 0, opts)?;
        match &root.kind {
            RootKind::Missing | RootKind::Unreadable => {
                write!(out, "<error>error opening dir</error>")?;
                write!(out, "{}", self.nl(opts))?;
                self.close_tag(out, tag, 0, opts)?;
            }
            RootKind::OverLimit(entries) => {
                write!(
                    out,
                    "<error>{entries} entries exceeds filelimit, not opening dir</error>"
                )?;
                write!(out, "{}", self.nl(opts))?;
                self.close_tag(out, tag, 0, opts)?;
            }
            RootKind::Opened(children) => {
                write!(out, "{}", self.nl(opts))?;
                for child in children {
                    self.emit_node(out, child, &root.name, 1, opts)?;
                }
                self.close_tag(out, tag, 0, opts)?;
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
        let nl = self.nl(opts);
        self.indent(out, 0, opts)?;
        write!(out, "<report>{nl}")?;
        if opts.du {
            self.indent(out, 1, opts)?;
            write!(out, "<size>{}</size>{nl}", totals.size)?;
        }
        self.indent(out, 1, opts)?;
        write!(out, "<directories>{}</directories>{nl}", totals.dirs)?;
        if !opts.dirs_only {
            self.indent(out, 1, opts)?;
            write!(out, "<files>{}</files>{nl}", totals.files)?;
        }
        self.indent(out, 0, opts)?;
        write!(out, "</report>{nl}")
    }
}

// SPDX-License-Identifier: GPL-2.0-or-later
//! -H backend: a port of tree's html.c. Spaces become &nbsp;, every
//! indented line starts with a tab, and entries are <a> elements whose
//! href is baseHREF + the path (-R boundaries link to 00Tree.html).

use std::io::{self, Write};

use super::linedraw::{self, LineDraw};
use super::meta::{self, Ids};
use super::xml::html_encode;
use super::{Formatter, Totals};
use crate::options::Options;
use crate::walk::{Meta, Node, Root, RootKind, join_path};

pub struct HtmlFormatter {
    glyphs: &'static LineDraw,
    ids: Ids,
    now: i64,
    /// strlen of the current root argument (tree's htmldirlen).
    dirlen: usize,
}

const VERSION_BANNER: &[&[u8]] = &[
    b"tree v2.3.2 ",
    b" 1996 - 2026 by Steve Baker, Thomas Moore, Francesc Rocher, Florian Sesser, Kyosuke Tokoro",
];

fn url_encode(out: &mut dyn Write, s: &[u8]) -> io::Result<bool> {
    let mut slash = false;
    for &c in s {
        if c.is_ascii_alphanumeric() || b"/-._~".contains(&c) {
            out.write_all(&[c])?;
        } else {
            write!(out, "%{c:02X}")?;
        }
        slash = c == b'/';
    }
    Ok(slash)
}

/// html_print(): spaces as &nbsp;, plus the two trailing &nbsp;.
fn html_print(out: &mut dyn Write, s: &[u8]) -> io::Result<()> {
    for &c in s {
        if c == b' ' {
            out.write_all(b"&nbsp;")?;
        } else {
            out.write_all(&[c])?;
        }
    }
    out.write_all(b"&nbsp;&nbsp;")
}

impl HtmlFormatter {
    pub fn new(opts: &Options) -> Self {
        HtmlFormatter {
            glyphs: linedraw::select(opts),
            ids: Ids::default(),
            now: super::now_epoch(),
            dirlen: 0,
        }
    }

    fn banner(&self, out: &mut dyn Write) -> io::Result<()> {
        out.write_all(VERSION_BANNER[0])?;
        out.write_all(self.glyphs.copy)?;
        out.write_all(VERSION_BANNER[1])
    }

    fn emit_info(&mut self, out: &mut dyn Write, m: &Meta, opts: &Options) -> io::Result<()> {
        if let Some(info) = meta::fillinfo(Some(m), opts, &mut self.ids, self.now)
            && info.first() == Some(&b'[')
        {
            html_print(out, &info)?;
            out.write_all(b"&nbsp;&nbsp;")?;
        }
        Ok(())
    }

    fn indent(&self, out: &mut dyn Write, stack: &[bool], last: Option<bool>) -> io::Result<()> {
        out.write_all(b"\t")?;
        for &more in stack {
            if more {
                out.write_all(self.glyphs.vert)?;
            } else {
                out.write_all(b"&nbsp;&nbsp;&nbsp;")?;
            }
            out.write_all(b"&nbsp;")?;
        }
        if let Some(last) = last {
            out.write_all(if last {
                self.glyphs.corner
            } else {
                self.glyphs.tee
            })?;
            out.write_all(b"&nbsp;")?;
        }
        Ok(())
    }

    /// The <a> element: class under -C, title from --info, href unless
    /// --nolinks. `descend` follows tree's encoding: >1 links to
    /// 00Tree.html, directories otherwise get a trailing slash.
    #[allow(clippy::too_many_arguments)]
    fn anchor(
        &mut self,
        out: &mut dyn Write,
        dirname: Option<&[u8]>,
        filename: &[u8],
        node_bits: Option<(&Node, &Meta)>,
        class_of: Option<(bool, u32)>,
        has_file: bool,
        is_dir: bool,
        descend: i32,
        opts: &Options,
    ) -> io::Result<()> {
        out.write_all(b"<a")?;
        if has_file {
            // class() checks the followed stat: isdir, isexe, isfifo,
            // issok, in that order.
            if opts.force_color
                && let Some((followed_is_dir, followed_mode)) = class_of
            {
                let class = if followed_is_dir {
                    "DIR"
                } else if followed_mode & 0o111 != 0 {
                    "EXEC"
                } else if followed_mode & 0o170000 == 0o010000 {
                    "FIFO"
                } else if followed_mode & 0o170000 == 0o140000 {
                    "SOCK"
                } else {
                    "NORM"
                };
                write!(out, " class=\"{class}\"")?;
            }
            if let Some((node, _)) = &node_bits
                && let Some(comment) = &node.comment
            {
                out.write_all(b" title=\"")?;
                for (i, line) in comment.iter().enumerate() {
                    html_encode(out, line)?;
                    if i + 1 < comment.len() {
                        out.write_all(b"\n")?;
                    }
                }
                out.write_all(b"\"")?;
            }
            if !opts.nolinks {
                out.write_all(b" href=\"")?;
                let host: &[u8] = opts.host.as_deref().unwrap_or(b"");
                out.write_all(host)?;
                if let Some(dirname) = dirname {
                    let off = if opts.htmloffset && dirname.len() >= self.dirlen {
                        self.dirlen
                    } else {
                        0
                    };
                    url_encode(out, &dirname[off..])?;
                    if dirname != filename {
                        if dirname.last() != Some(&b'/') {
                            out.write_all(b"/")?;
                        }
                        url_encode(out, filename)?;
                    }
                    if descend > 1 {
                        out.write_all(b"/00Tree.html")?;
                    }
                    if is_dir && descend < 2 {
                        out.write_all(b"/")?;
                    }
                } else {
                    if host.last() != Some(&b'/') {
                        out.write_all(b"/")?;
                    }
                    url_encode(out, filename)?;
                    if descend > 1 {
                        out.write_all(b"/00Tree.html")?;
                    }
                }
                out.write_all(b"\"")?;
            }
        }
        out.write_all(b">")?;
        html_encode(out, filename)?;
        out.write_all(b"</a>")
    }

    fn emit_children(
        &mut self,
        out: &mut dyn Write,
        nodes: &[Node],
        parent_path: &[u8],
        stack: &mut Vec<bool>,
        opts: &Options,
    ) -> io::Result<()> {
        for (index, node) in nodes.iter().enumerate() {
            let last = index + 1 == nodes.len();
            if !opts.noindent {
                self.indent(out, stack, Some(last))?;
            }
            self.emit_info(out, &node.meta, opts)?;

            let full_path;
            let display: &[u8] = if opts.full_path {
                full_path = join_path(parent_path, &node.name);
                &full_path
            } else {
                &node.name
            };
            let descend = if node.rerun_link {
                11
            } else {
                i32::from(node.children.is_some())
            };
            let followed_mode = if node.link.is_some() {
                node.meta.lnk_mode
            } else {
                node.meta.mode
            };
            self.anchor(
                out,
                Some(parent_path),
                display,
                Some((node, &node.meta)),
                Some((node.is_dir, followed_mode)),
                true,
                node.is_dir,
                descend,
                opts,
            )?;
            if let Some(err) = &node.err {
                write!(out, "  [{err}]")?;
            }
            out.write_all(b"<br>\n")?;

            if let Some(children) = &node.children {
                stack.push(!last);
                let child_parent = join_path(parent_path, &node.name);
                self.emit_children(out, children, &child_parent, stack, opts)?;
                stack.pop();
            }
        }
        Ok(())
    }
}

impl Formatter for HtmlFormatter {
    fn intro(&mut self, out: &mut dyn Write, opts: &Options) -> io::Result<()> {
        if let Some(path) = &opts.hintro {
            if let Ok(content) = std::fs::read(path) {
                out.write_all(&content)?;
            }
            return Ok(());
        }
        let charset = linedraw::charset_name(opts).unwrap_or_else(|| b"iso-8859-1".to_vec());
        let title: &[u8] = opts.title.as_deref().unwrap_or(b"Directory Tree");
        out.write_all(
            b"<!DOCTYPE html>\n<html>\n<head>\n \
              <meta http-equiv=\"Content-Type\" content=\"text/html; charset=",
        )?;
        out.write_all(&charset)?;
        out.write_all(b"\">\n <meta name=\"Author\" content=\"Made by 'tree'\">\n <meta name=\"GENERATOR\" content=\"")?;
        self.banner(out)?;
        out.write_all(b"\">\n <title>")?;
        out.write_all(title)?;
        out.write_all(
            b"</title>\n <style type=\"text/css\">\n\
              \x20 BODY { font-family : monospace, sans-serif;  color: black;}\n\
              \x20 P { font-family : monospace, sans-serif; color: black; margin:0px; padding: 0px;}\n\
              \x20 A:visited { text-decoration : none; margin : 0px; padding : 0px;}\n\
              \x20 A:link    { text-decoration : none; margin : 0px; padding : 0px;}\n\
              \x20 A:hover   { text-decoration: underline; background-color : yellow; margin : 0px; padding : 0px;}\n\
              \x20 A:active  { margin : 0px; padding : 0px;}\n\
              \x20 .VERSION { font-size: small; font-family : arial, sans-serif; }\n\
              \x20 .NORM  { color: black;  }\n\
              \x20 .FIFO  { color: purple; }\n\
              \x20 .CHAR  { color: yellow; }\n\
              \x20 .DIR   { color: blue;   }\n\
              \x20 .BLOCK { color: yellow; }\n\
              \x20 .LINK  { color: aqua;   }\n\
              \x20 .SOCK  { color: fuchsia;}\n\
              \x20 .EXEC  { color: green;  }\n\
              \x20</style>\n</head>\n<body>\n\t<h1>",
        )?;
        out.write_all(title)?;
        out.write_all(b"</h1><p>\n")?;
        Ok(())
    }

    fn outro(&mut self, out: &mut dyn Write, opts: &Options) -> io::Result<()> {
        if let Some(path) = &opts.houtro {
            if let Ok(content) = std::fs::read(path) {
                out.write_all(&content)?;
            }
            return Ok(());
        }
        out.write_all(b"\t<hr>\n\t<p class=\"VERSION\">\n")?;
        // hversion, with the copy glyph substituted four times.
        out.write_all(b"\t\t tree v2.3.2 ")?;
        out.write_all(self.glyphs.copy)?;
        out.write_all(b" 1996 - 2026 by Steve Baker and Thomas Moore <br>\n\t\t HTML output hacked and copyleft ")?;
        out.write_all(self.glyphs.copy)?;
        out.write_all(b" 1998 by Francesc Rocher <br>\n\t\t JSON output hacked and copyleft ")?;
        out.write_all(self.glyphs.copy)?;
        out.write_all(b" 2014 by Florian Sesser <br>\n\t\t Charsets / OS/2 support ")?;
        out.write_all(self.glyphs.copy)?;
        out.write_all(b" 2001 by Kyosuke Tokoro\n")?;
        out.write_all(b"\t</p>\n</body>\n</html>\n")
    }

    fn emit_root(
        &mut self,
        out: &mut dyn Write,
        root: &Root,
        opts: &Options,
        _more_roots: bool,
    ) -> io::Result<()> {
        self.dirlen = root.name.len();
        if !opts.noindent {
            out.write_all(b"\t")?;
        }
        if let Some(m) = &root.meta {
            self.emit_info(out, m, opts)?;
        }
        let descend = match &root.kind {
            RootKind::Opened(children) if children.is_empty() => 0,
            _ => 1,
        };
        // The root <a> has no class/title; dirname == filename.
        // A failed lstat (meta None) leaves a bare <a>name</a>.
        let is_dir = root
            .meta
            .as_ref()
            .is_some_and(|m| m.mode & 0o170000 == 0o040000);
        let name = root.name.clone();
        let class_of = root.meta.as_ref().map(|m| (is_dir, m.mode));
        self.anchor(
            out,
            Some(&name),
            &name,
            None,
            class_of,
            root.meta.is_some(),
            is_dir,
            descend,
            opts,
        )?;
        match &root.kind {
            RootKind::Missing | RootKind::Unreadable => {
                out.write_all(b"  [error opening dir]")?;
            }
            RootKind::OverLimit(entries) => {
                write!(
                    out,
                    "  [{entries} entries exceeds filelimit, not opening dir]"
                )?;
            }
            RootKind::Opened(_) => {}
        }
        out.write_all(b"<br>\n")?;
        if let RootKind::Opened(children) = &root.kind {
            let mut stack = Vec::new();
            self.emit_children(out, children, &root.name, &mut stack, opts)?;
        }
        Ok(())
    }

    fn emit_report(
        &mut self,
        out: &mut dyn Write,
        totals: &Totals,
        opts: &Options,
    ) -> io::Result<()> {
        out.write_all(b"<br><br><p>\n\n")?;
        if opts.du {
            out.write_all(&meta::psize(totals.size, opts))?;
            let unit = if opts.human || opts.si { "" } else { " bytes" };
            write!(out, "{unit} used in ")?;
        }
        let dir_suffix = if totals.dirs == 1 { "y" } else { "ies" };
        if opts.dirs_only {
            writeln!(out, "{} director{dir_suffix}", totals.dirs)?;
        } else {
            let file_suffix = if totals.files == 1 { "" } else { "s" };
            writeln!(
                out,
                "{} director{dir_suffix}, {} file{file_suffix}",
                totals.dirs, totals.files
            )?;
        }
        out.write_all(b"\n</p>\n")
    }
}

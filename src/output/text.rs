// SPDX-License-Identifier: GPL-2.0-or-later
//! Plain-text backend: the classic tree rendering (unix.c equivalent).

use std::io::{self, Write};

use super::color::Colors;
use super::linedraw::{self, LineDraw};
use super::{Formatter, Totals, meta};
use crate::options::Options;
use crate::walk::{Node, Root, RootKind, join_path};

pub struct TextFormatter {
    glyphs: &'static LineDraw,
    colors: Colors,
    ids: meta::Ids,
    now: i64,
}

/// What decides an entry's color: mode, bare name (for suffix rules),
/// orphan state, and whether this is the target after " -> ".
struct Paint<'a> {
    mode: u32,
    name: &'a [u8],
    orphan: bool,
    islink: bool,
}

impl TextFormatter {
    pub fn new(opts: &Options) -> Self {
        TextFormatter {
            glyphs: linedraw::select(opts),
            colors: Colors::parse(opts),
            ids: meta::Ids::default(),
            now: super::now_epoch(),
        }
    }

    /// unix_printfile's coloring: the name takes the entry's color
    /// (or the target's, under ln=target).
    fn write_colored_name(
        &mut self,
        out: &mut dyn Write,
        display: &[u8],
        paint: Paint<'_>,
        opts: &Options,
    ) -> io::Result<()> {
        let seq = if self.colors.enabled {
            self.colors
                .for_entry(paint.mode, paint.name, paint.orphan, paint.islink)
        } else {
            None
        };
        if let Some(seq) = &seq {
            out.write_all(seq)?;
        }
        write_name(out, display, opts)?;
        if seq.is_some() {
            out.write_all(self.colors.end())?;
        }
        Ok(())
    }

    fn emit_info(
        &mut self,
        out: &mut dyn Write,
        m: Option<&crate::walk::Meta>,
        opts: &Options,
    ) -> io::Result<()> {
        if let Some(info) = meta::fillinfo(m, opts, &mut self.ids, self.now)
            && info.first() == Some(&b'[')
        {
            out.write_all(&info)?;
            out.write_all(b"  ")?;
        }
        Ok(())
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
                for &more in stack.iter() {
                    out.write_all(if more { self.glyphs.vert } else { b"   " })?;
                    out.write_all(b" ")?;
                }
                out.write_all(if last {
                    self.glyphs.corner
                } else {
                    self.glyphs.tee
                })?;
                out.write_all(b" ")?;
            }
            self.emit_info(out, Some(&node.meta), opts)?;

            let full_path;
            let display: &[u8] = if opts.full_path {
                full_path = join_path(parent_path, &node.name);
                &full_path
            } else {
                &node.name
            };
            // ln=target colors the link name by its target's mode.
            let name_mode = if node.link.is_some() && self.colors.link_target {
                node.meta.lnk_mode
            } else {
                node.meta.mode
            };
            self.write_colored_name(
                out,
                display,
                Paint {
                    mode: name_mode,
                    name: &node.name,
                    orphan: node.orphan,
                    islink: false,
                },
                opts,
            )?;
            if let Some(target) = &node.link {
                out.write_all(b" -> ")?;
                self.write_colored_name(
                    out,
                    target,
                    Paint {
                        mode: node.meta.lnk_mode,
                        name: target,
                        orphan: node.orphan,
                        islink: true,
                    },
                    opts,
                )?;
                if opts.classify
                    && let Some(c) = meta::ftype(node.meta.lnk_mode, opts)
                {
                    out.write_all(&[c])?;
                }
            } else if opts.classify
                && let Some(c) = meta::ftype(node.meta.mode, opts)
            {
                out.write_all(&[c])?;
            }
            if let Some(err) = &node.err {
                write!(out, "  [{err}]")?;
            }
            out.write_all(b"\n")?;

            if let Some(comment) = &node.comment {
                self.emit_comment(out, comment, stack, last, opts.noindent)?;
            }

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

impl TextFormatter {
    /// --info comment lines under an entry (tree's printcomment).
    fn emit_comment(
        &self,
        out: &mut dyn Write,
        lines: &[Vec<u8>],
        stack: &[bool],
        last: bool,
        noindent: bool,
    ) -> io::Result<()> {
        for (index, line) in lines.iter().enumerate() {
            if !noindent {
                for &more in stack {
                    out.write_all(if more { self.glyphs.vert } else { b"   " })?;
                    out.write_all(b" ")?;
                }
                out.write_all(if last { b"   " } else { self.glyphs.vert })?;
                out.write_all(b" ")?;
            }

            let glyph = if lines.len() == 1 {
                self.glyphs.csingle
            } else if index == 0 {
                self.glyphs.ctop
            } else if index == 1 {
                if lines.len() == 2 {
                    self.glyphs.cbot
                } else {
                    self.glyphs.cmid
                }
            } else if index == lines.len() - 1 {
                self.glyphs.cbot
            } else {
                self.glyphs.cext
            };
            out.write_all(glyph)?;
            out.write_all(b" ")?;
            out.write_all(line)?;
            out.write_all(b"\n")?;
        }
        Ok(())
    }
}

impl Formatter for TextFormatter {
    fn emit_root(
        &mut self,
        out: &mut dyn Write,
        root: &Root,
        opts: &Options,
        _more_roots: bool,
    ) -> io::Result<()> {
        self.emit_info(out, root.meta.as_ref(), opts)?;
        let root_mode = root.meta.as_ref().map_or(0, |m| m.mode);
        self.write_colored_name(
            out,
            &root.name,
            Paint {
                mode: root_mode,
                name: b"",
                orphan: false,
                islink: false,
            },
            opts,
        )?;
        if opts.classify
            && let Some(m) = &root.meta
            && let Some(c) = meta::ftype(m.mode, opts)
        {
            out.write_all(&[c])?;
        }
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
        out.write_all(b"\n")?;

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
        out.write_all(b"\n")?;
        if opts.du {
            out.write_all(&meta::psize(totals.size, opts))?;
            let unit = if opts.human || opts.si { "" } else { " bytes" };
            write!(out, "{unit} used in ")?;
        }
        let dir_suffix = if totals.dirs == 1 { "y" } else { "ies" };
        if opts.dirs_only {
            writeln!(out, "{} director{dir_suffix}", totals.dirs)
        } else {
            let file_suffix = if totals.files == 1 { "" } else { "s" };
            writeln!(
                out,
                "{} director{dir_suffix}, {} file{file_suffix}",
                totals.dirs, totals.files
            )
        }
    }
}

/// tree's printit() under a single-byte locale, honoring -N/-Q/-q.
fn write_name(out: &mut dyn Write, bytes: &[u8], opts: &Options) -> io::Result<()> {
    if opts.no_escape {
        if opts.quote {
            out.write_all(b"\"")?;
        }
        out.write_all(bytes)?;
        if opts.quote {
            out.write_all(b"\"")?;
        }
        return Ok(());
    }
    if opts.quote {
        out.write_all(b"\"")?;
    }
    for &byte in bytes {
        if (7..=13).contains(&byte)
            || byte == b'\\'
            || (byte == b'"' && opts.quote)
            || (byte == b' ' && !opts.quote)
        {
            out.write_all(b"\\")?;
            if byte > 13 {
                out.write_all(&[byte])?;
            } else {
                out.write_all(&[b"abtnvfr"[(byte - 7) as usize]])?;
            }
        } else if (0x20..=0x7e).contains(&byte) {
            out.write_all(&[byte])?;
        } else if opts.qmark {
            out.write_all(b"?")?;
        } else {
            write!(out, "\\{byte:03o}")?;
        }
    }
    if opts.quote {
        out.write_all(b"\"")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::write_name;

    fn esc(input: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        write_name(&mut out, input, &crate::options::Options::default()).unwrap();
        out
    }

    #[test]
    fn escaping_matches_tree_printit() {
        assert_eq!(esc(b"plain"), b"plain");
        assert_eq!(esc(b"with space"), b"with\\ space");
        assert_eq!(esc(b"tab\there"), b"tab\\there");
        assert_eq!(esc(b"back\\slash"), b"back\\\\slash");
        assert_eq!(esc(&[0xff, b'x']), b"\\377x");
        assert_eq!(esc(&[0x01]), b"\\001");
    }
}

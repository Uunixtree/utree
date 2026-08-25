// SPDX-License-Identifier: GPL-2.0-or-later
//! Directory traversal, mirroring the behavior of tree's
//! getinfo/read_dir/listdir/getfulltree but not their structure.

use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use crate::filter::FilterStack;
use crate::info::InfoStack;
use crate::options::{Options, OutputFormat, SortKey, TopSort};
use crate::output::Totals;
use crate::pattern;

/// lstat-side metadata (tree fills _info from lstat; only lnk_mode
/// comes from the followed target).
#[derive(Clone, Copy, Default)]
pub struct Meta {
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: i64,
    pub mtime: i64,
    pub ctime: i64,
    /// lstat inode/device (tree's linode/ldev, for --inodes/--device).
    pub ino: u64,
    pub dev: u64,
    /// Followed inode/device (tree's inode/dev; what -J/-X print).
    /// Zero for roots, mirroring stat2info leaving them unset.
    pub fino: u64,
    pub fdev: u64,
    /// Followed mode for symlinks; 0 for orphans.
    pub lnk_mode: u32,
}

pub struct Node {
    /// Entry name as raw bytes (no encoding assumed).
    pub name: Vec<u8>,
    /// True also for symlinks pointing at directories.
    pub is_dir: bool,
    pub link: Option<Vec<u8>>,
    /// Trailing annotation such as "error opening dir".
    pub err: Option<String>,
    /// None means nothing below: empty, filtered out, or never opened.
    pub children: Option<Vec<Node>>,
    /// --info description lines.
    pub comment: Option<Vec<Vec<u8>>>,
    /// Symlink whose target could not be stat'ed.
    pub orphan: bool,
    /// -R boundary: a dir at the deepest -L level that gets its own
    /// 00Tree.html (and a link pointing into it).
    pub rerun_link: bool,
    pub meta: Meta,
    dev: u64,
    ino: u64,
}

pub enum RootKind {
    /// lstat failed; counts as nothing.
    Missing,
    /// Cannot be listed (e.g. a plain file); tree counts one file.
    Unreadable,
    /// Over --filelimit; counts as one directory.
    OverLimit(usize),
    /// An empty listing counts zero, like tree.
    Opened(Vec<Node>),
}

pub struct Root {
    /// The path argument; trailing slashes stripped under -f.
    pub name: Vec<u8>,
    pub kind: RootKind,
    /// None when lstat failed.
    pub meta: Option<Meta>,
}

impl Root {
    pub fn totals(&self) -> Totals {
        let mut totals = match &self.kind {
            RootKind::Missing => Totals::default(),
            RootKind::Unreadable => Totals {
                files: 1,
                ..Totals::default()
            },
            RootKind::OverLimit(_) => Totals {
                dirs: 1,
                ..Totals::default()
            },
            RootKind::Opened(children) => {
                let mut totals = count(children);
                totals.dirs += 1;
                totals
            }
        };
        // tree: "if (flag.du) tot.size += info? info->size : 0;"
        totals.size = self.meta.map_or(0, |m| m.size);
        totals
    }
}

fn count(nodes: &[Node]) -> Totals {
    let mut totals = Totals::default();
    for node in nodes {
        if node.is_dir {
            totals.dirs += 1;
        } else {
            totals.files += 1;
        }
        if let Some(children) = &node.children {
            totals.add(&count(children));
        }
    }
    totals
}

pub struct Walker<'a> {
    opts: &'a Options,
    /// Symlink loop detection; tree's saveino/findino registry.
    visited: HashSet<(u64, u64)>,
    filters: FilterStack,
    infos: InfoStack,
    errors: u64,
}

impl<'a> Walker<'a> {
    /// Returns None when --gitfile names an unloadable file (tree
    /// exits 1 with "Could not load gitignore file").
    pub fn new(opts: &'a Options) -> Option<Self> {
        let mut infos = InfoStack::default();
        // A global --infofile stays at the bottom of the stack all run.
        if let Some(file) = &opts.infofile {
            infos.push_file(Path::new(file));
        }
        let mut filters = FilterStack::default();
        if let Some(file) = &opts.gitfile
            && !filters.push_explicit(Path::new(file))
        {
            return None;
        }
        Some(Walker {
            opts,
            visited: HashSet::new(),
            filters,
            infos,
            errors: 0,
        })
    }

    fn show_info(&self) -> bool {
        self.opts.info || self.opts.infofile.is_some()
    }

    pub fn errors(&self) -> u64 {
        self.errors
    }

    /// The pattern-disabling behavior of --matchdirs/--prune/--du is
    /// tied to tree's build-the-whole-tree-first mode.
    fn full_tree_mode(&self) -> bool {
        self.opts.prune || self.opts.matchdirs || self.opts.du
    }

    fn level_limit(&self) -> usize {
        self.opts.level.unwrap_or(usize::MAX)
    }

    pub fn walk_root(&mut self, arg: &OsString) -> Root {
        let mut name = arg.as_bytes().to_vec();
        if self.opts.full_path {
            while name.len() > 1 && name.last() == Some(&b'/') {
                name.pop();
            }
        }
        let fs_path = PathBuf::from(OsStr::from_bytes(&name));

        let Ok(meta) = fs::symlink_metadata(&fs_path) else {
            self.errors += 1;
            return Root {
                name,
                kind: RootKind::Missing,
                meta: None,
            };
        };
        self.visited.insert((meta.dev(), meta.ino()));
        let root_dev = meta.dev();
        let root_meta = meta_of(&meta, meta.mode(), 0, 0);

        let filter_mark = self.filters.mark();
        let mut flush_filters = false;
        if self.opts.gitignore {
            flush_filters = self.filters.push_root(&fs_path);
        }
        let info_mark = self.infos.mark();
        let info_top = self.show_info() && self.infos.push_dir(&fs_path, true);

        // tree.c: "if the directory name matches, turn off pattern
        // matching for contents".
        let pattern_active = !(self.full_tree_mode()
            && !self.opts.patterns.is_empty()
            && (pattern::any_match(
                &name,
                &self.opts.patterns,
                true,
                self.opts.ignore_case,
                true,
            ) || pattern::any_match(
                last_component(&name),
                &self.opts.patterns,
                true,
                self.opts.ignore_case,
                false,
            )));

        let kind = match self.read_entries(&fs_path, pattern_active, info_top) {
            Err(_) => RootKind::Unreadable,
            Ok(mut nodes) => {
                if let Some(limit) = self.opts.file_limit
                    && nodes.len() as i64 > limit
                {
                    RootKind::OverLimit(nodes.len())
                } else {
                    self.descend(&mut nodes, &fs_path, 1, root_dev, pattern_active);
                    RootKind::Opened(nodes)
                }
            }
        };
        let mut root_meta = root_meta;
        if self.opts.du
            && let RootKind::Opened(nodes) = &kind
        {
            root_meta.size += nodes.iter().map(|n| n.meta.size).sum::<i64>();
        }
        // tree flushes the whole stack — a --gitfile entry included —
        // after any root whose upward search found ignore files.
        if flush_filters {
            self.filters.flush();
        } else {
            self.filters.truncate(filter_mark);
        }
        self.infos.truncate(info_mark);
        Root {
            name,
            kind,
            meta: Some(root_meta),
        }
    }

    /// List one directory, applying getinfo's filters in its order.
    fn read_entries(
        &self,
        dir: &Path,
        pattern_active: bool,
        info_top: bool,
    ) -> std::io::Result<Vec<Node>> {
        let ic = self.opts.ignore_case;
        let dir_bytes = dir.as_os_str().as_bytes();
        let mut nodes = Vec::new();

        for entry in fs::read_dir(dir)? {
            let Ok(entry) = entry else { continue };
            let name = entry.file_name().as_bytes().to_vec();
            // read_dir hides its own output files under -H.
            if self.opts.output == OutputFormat::Html && name == b"00Tree.html" {
                continue;
            }
            if !self.opts.all_files && name.first() == Some(&b'.') {
                continue;
            }
            let path = entry.path();
            let Ok(lmeta) = fs::symlink_metadata(&path) else {
                continue;
            };
            let lstat_is_dir = lmeta.file_type().is_dir();

            let mut link: Option<Vec<u8>> = None;
            let (mut dev, mut ino, mut is_dir, mut lnk_mode, mut orphan) =
                (0u64, 0u64, false, 0u32, false);
            if lmeta.file_type().is_symlink() {
                let readlink = fs::read_link(&path);
                let read_ok = readlink.is_ok();
                link = Some(match readlink {
                    Ok(target) => target.as_os_str().as_bytes().to_vec(),
                    Err(_) => b"[Error reading symbolic link information]".to_vec(),
                });
                match fs::metadata(&path) {
                    Ok(target) => {
                        dev = target.dev();
                        ino = target.ino();
                        is_dir = target.is_dir();
                        lnk_mode = target.mode();
                    }
                    Err(_) => {
                        // tree zeroes the target stat of orphaned links.
                        orphan = read_ok;
                    }
                }
            } else {
                dev = lmeta.dev();
                ino = lmeta.ino();
                is_dir = lstat_is_dir;
                lnk_mode = lmeta.mode();
            }

            let full_path = join_path(dir_bytes, &name);
            if self.opts.gitignore && self.filters.check(&full_path, &name, is_dir) {
                continue;
            }
            if !lstat_is_dir
                && !(self.opts.follow_links && is_dir)
                && pattern_active
                && !self.opts.patterns.is_empty()
                && !pattern::any_match(&name, &self.opts.patterns, is_dir, ic, false)
                && !pattern::any_match(&full_path, &self.opts.patterns, is_dir, ic, true)
            {
                continue;
            }
            if !self.opts.ipatterns.is_empty()
                && (pattern::any_match(&name, &self.opts.ipatterns, is_dir, ic, false)
                    || pattern::any_match(&full_path, &self.opts.ipatterns, is_dir, ic, true))
            {
                continue;
            }
            if self.opts.dirs_only && !is_dir {
                continue;
            }

            let comment = if self.show_info() {
                self.infos.check(&full_path, &name, info_top, is_dir)
            } else {
                None
            };

            nodes.push(Node {
                name,
                is_dir,
                link,
                err: None,
                children: None,
                comment,
                orphan,
                rerun_link: false,
                meta: meta_of(&lmeta, lnk_mode, dev, ino),
                dev,
                ino,
            });
        }
        Ok(nodes)
    }

    /// tree's comparators: reverse applies to the base sort (including
    /// its name tie-break) but never to the dirsfirst/filesfirst split.
    fn compare(&self, a: &Node, b: &Node) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        match self.opts.top {
            TopSort::DirsFirst if a.is_dir != b.is_dir => {
                return if a.is_dir {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }
            TopSort::FilesFirst if a.is_dir != b.is_dir => {
                return if a.is_dir {
                    Ordering::Greater
                } else {
                    Ordering::Less
                };
            }
            _ => {}
        }

        let by_name = |a: &Node, b: &Node| a.name.cmp(&b.name);
        let ord = match self.opts.sort {
            SortKey::Name => by_name(a, b),
            SortKey::Version => crate::verscmp::strverscmp(&a.name, &b.name).cmp(&0),
            // Size sorts descending in tree.
            SortKey::Size => b.meta.size.cmp(&a.meta.size).then_with(|| by_name(a, b)),
            SortKey::Mtime => a.meta.mtime.cmp(&b.meta.mtime).then_with(|| by_name(a, b)),
            SortKey::Ctime => a.meta.ctime.cmp(&b.meta.ctime).then_with(|| by_name(a, b)),
            SortKey::Unsorted => Ordering::Equal,
        };
        if self.opts.reverse {
            ord.reverse()
        } else {
            ord
        }
    }

    fn sort_nodes(&self, nodes: &mut [Node]) {
        if self.opts.sort == SortKey::Unsorted {
            return;
        }
        nodes.sort_by(|a, b| self.compare(a, b));
    }

    /// One level of the walk. listdir sorts before opening the
    /// directories; getfulltree (--prune/--matchdirs/--du) opens them
    /// in readdir order and sorts afterwards — "sorting needs to be
    /// deferred for --du", and it also changes which of several
    /// symlinks to one target is tagged recursive.
    fn descend(
        &mut self,
        nodes: &mut Vec<Node>,
        parent: &Path,
        depth: usize,
        root_dev: u64,
        pattern_active: bool,
    ) {
        if self.full_tree_mode() {
            self.process(nodes, parent, depth, root_dev, pattern_active);
            self.sort_nodes(nodes);
        } else {
            self.sort_nodes(nodes);
            self.process(nodes, parent, depth, root_dev, pattern_active);
        }
    }

    fn process(
        &mut self,
        nodes: &mut Vec<Node>,
        parent: &Path,
        depth: usize,
        root_dev: u64,
        pattern_active: bool,
    ) {
        let mut i = 0;
        while i < nodes.len() {
            let node = &mut nodes[i];
            let mut pruned = false;

            if node.is_dir && !(self.opts.xdev && node.dev != root_dev) {
                let link_ok = node.link.is_none() || self.opts.follow_links;
                if self.opts.rerun && link_ok && depth >= self.level_limit() {
                    node.rerun_link = true;
                }
                let mut do_descend = true;
                if node.link.is_some() {
                    if !self.opts.follow_links {
                        do_descend = false;
                    } else if self.visited.contains(&(node.dev, node.ino)) {
                        // The tag is suppressed at the deepest -L level
                        // in both of tree's modes ("not actually a
                        // problem if we weren't going to descend").
                        if depth < self.level_limit() {
                            node.err = Some("recursive, not followed".to_string());
                        }
                        do_descend = false;
                    } else {
                        self.visited.insert((node.dev, node.ino));
                    }
                } else {
                    self.visited.insert((node.dev, node.ino));
                }

                if do_descend && depth < self.level_limit() {
                    // getfulltree descends symlinks via parent/<target>
                    // (absolute targets used as-is); listdir descends
                    // via parent/<name>. The path matters: pattern
                    // disabling, .gitignore and .info lookups use it.
                    let child_path = match &node.link {
                        Some(target) if self.full_tree_mode() => {
                            if target.first() == Some(&b'/') {
                                PathBuf::from(OsStr::from_bytes(target))
                            } else {
                                parent.join(OsStr::from_bytes(target))
                            }
                        }
                        _ => parent.join(OsStr::from_bytes(&node.name)),
                    };
                    let filter_mark = self.filters.mark();
                    if self.opts.gitignore {
                        self.filters.push_dir(&child_path);
                    }
                    let info_mark = self.infos.mark();
                    let info_top = self.show_info() && self.infos.push_dir(&child_path, false);
                    // getfulltree checks the full path (and its
                    // /-suffixes) as well as the bare name; '?' in a
                    // pattern can even match the '/' itself.
                    let child_pattern_active = pattern_active
                        && !(self.full_tree_mode()
                            && !self.opts.patterns.is_empty()
                            && (pattern::any_match(
                                child_path.as_os_str().as_bytes(),
                                &self.opts.patterns,
                                true,
                                self.opts.ignore_case,
                                true,
                            ) || pattern::any_match(
                                last_component(child_path.as_os_str().as_bytes()),
                                &self.opts.patterns,
                                true,
                                self.opts.ignore_case,
                                false,
                            )));
                    match self.read_entries(&child_path, child_pattern_active, info_top) {
                        Err(_) => {
                            node.err = Some("error opening dir".to_string());
                            if !self.full_tree_mode() {
                                self.errors += 1;
                            }
                        }
                        Ok(mut children) => {
                            if let Some(limit) = self.opts.file_limit
                                && children.len() as i64 > limit
                            {
                                node.err = Some(format!(
                                    "{} entries exceeds filelimit, not opening dir",
                                    children.len()
                                ));
                                if !self.full_tree_mode() {
                                    self.errors += 1;
                                }
                            } else if !children.is_empty() {
                                self.descend(
                                    &mut children,
                                    &child_path,
                                    depth + 1,
                                    root_dev,
                                    child_pattern_active,
                                );
                                if self.opts.du {
                                    node.meta.size +=
                                        children.iter().map(|c| c.meta.size).sum::<i64>();
                                }
                                if !children.is_empty() {
                                    node.children = Some(children);
                                }
                            }
                        }
                    }
                    self.filters.truncate(filter_mark);
                    self.infos.truncate(info_mark);
                }

                // tree.c: "prune empty folders, unless they match the
                // requested pattern or sit at the -L cutoff".
                if self.opts.prune
                    && node.children.is_none()
                    && depth < self.level_limit()
                    && !(self.opts.matchdirs
                        && pattern_active
                        && !self.opts.patterns.is_empty()
                        && pattern::any_match(
                            &node.name,
                            &self.opts.patterns,
                            node.is_dir,
                            self.opts.ignore_case,
                            false,
                        ))
                {
                    pruned = true;
                }
            }

            if pruned {
                nodes.remove(i);
            } else {
                i += 1;
            }
        }
    }
}

fn meta_of(lmeta: &fs::Metadata, lnk_mode: u32, fdev: u64, fino: u64) -> Meta {
    Meta {
        mode: lmeta.mode(),
        uid: lmeta.uid(),
        gid: lmeta.gid(),
        size: lmeta.size() as i64,
        mtime: lmeta.mtime(),
        ctime: lmeta.ctime(),
        ino: lmeta.ino(),
        dev: lmeta.dev(),
        fino,
        fdev,
        lnk_mode,
    }
}

fn last_component(path: &[u8]) -> &[u8] {
    match path.iter().rposition(|&c| c == b'/') {
        Some(pos) => &path[pos + 1..],
        None => path,
    }
}

pub fn join_path(dir: &[u8], name: &[u8]) -> Vec<u8> {
    if dir == b"/" {
        [b"/" as &[u8], name].concat()
    } else {
        [dir, b"/", name].concat()
    }
}

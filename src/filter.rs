// SPDX-License-Identifier: GPL-2.0-or-later
//! --gitignore support: a byte-level port of tree's filter.c.

use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use crate::pattern::patmatch;
use crate::walk::join_path;

struct GitPattern {
    pattern: Vec<u8>,
    /// Bare-name pattern (no inner slash), vs anchored at its base dir.
    relative: bool,
}

struct IgnoreFile {
    base: Vec<u8>,
    remove: Vec<GitPattern>,
    reverse: Vec<GitPattern>,
}

#[derive(Default)]
pub struct FilterStack {
    stack: Vec<IgnoreFile>,
}

/// tree's gittrim(); also used for .info parsing, like the C original.
pub(crate) fn gittrim(s: &[u8]) -> Vec<u8> {
    let mut e = s.len();
    // `e > 1` (not 0) leaves a lone first character in place, matching
    // the C index arithmetic.
    while e > 1 && (s[e - 1] == b'\n' || s[e - 1] == b'\r') {
        e -= 1;
    }
    while e > 1 && s[e - 1] == b' ' {
        if s[e - 2] == b'\\' {
            break;
        }
        e -= 1;
    }
    let mut out = Vec::with_capacity(e);
    let mut i = 0;
    while i < e {
        if s[i] == b'\\' && i + 1 < e {
            i += 1;
        }
        out.push(s[i]);
        i += 1;
    }
    out
}

fn new_pattern(pattern: &[u8]) -> GitPattern {
    let stripped = if pattern.first() == Some(&b'/') {
        &pattern[1..]
    } else {
        pattern
    };
    // A trailing slash is patmatch's directory-only marker, not a path.
    let relative = match pattern.iter().position(|&c| c == b'/') {
        None => true,
        Some(pos) => pos + 1 == pattern.len(),
    };
    GitPattern {
        pattern: stripped.to_vec(),
        relative,
    }
}

fn parse_ignore_file(base: &Path, file: &Path) -> Option<IgnoreFile> {
    let content = std::fs::read(file).ok()?;
    let mut remove = Vec::new();
    let mut reverse = Vec::new();
    for line in content.split(|&c| c == b'\n') {
        if line.first() == Some(&b'#') {
            continue;
        }
        let rev = line.first() == Some(&b'!');
        let trimmed = gittrim(if rev { &line[1..] } else { line });
        if trimmed.is_empty() {
            continue;
        }
        let target = if rev { &mut reverse } else { &mut remove };
        target.push(new_pattern(&trimmed));
    }
    Some(IgnoreFile {
        base: base.as_os_str().as_bytes().to_vec(),
        remove,
        reverse,
    })
}

impl FilterStack {
    pub fn mark(&self) -> usize {
        self.stack.len()
    }

    pub fn truncate(&mut self, mark: usize) {
        self.stack.truncate(mark);
    }

    fn push_file(&mut self, base: &Path, file: &Path) -> bool {
        if let Some(ig) = parse_ignore_file(base, file) {
            self.stack.push(ig);
            return true;
        }
        false
    }

    /// --gitfile: the named file itself, with its path as the anchor
    /// base (tree's new_ignorefile(arg, arg, false)).
    pub fn push_explicit(&mut self, file: &Path) -> bool {
        file.is_file() && self.push_file(file, file)
    }

    /// Push `dir/.gitignore` if it exists.
    pub fn push_dir(&mut self, dir: &Path) {
        let file = dir.join(".gitignore");
        if file.is_file() {
            self.push_file(dir, &file);
        }
    }

    /// Drop everything, including a --gitfile entry: tree's
    /// flush_filterstack, run after a root that found ignore files.
    pub fn flush(&mut self) {
        self.stack.clear();
    }

    /// $GIT_DIR/info/exclude, then every .gitignore from the repository
    /// root down to the walk root (tree's gitignore_search). Returns
    /// whether the search found anything, which decides the post-root
    /// flush.
    pub fn push_root(&mut self, dir: &Path) -> bool {
        if let Some(git_dir) = std::env::var_os("GIT_DIR") {
            let git_dir = Path::new(&git_dir);
            let exclude = git_dir.join("info/exclude");
            if exclude.is_file() {
                self.push_file(git_dir, &exclude);
            }
        }
        self.search_upward(dir, 0)
    }

    fn search_upward(&mut self, start: &Path, depth: u32) -> bool {
        let mut found = false;
        let git = start.join(".git");
        if git.is_dir() {
            // A .git directory is assumed to be the repository root.
            let exclude = start.join(".git/info/exclude");
            if exclude.is_file() {
                found |= self.push_file(start, &exclude);
            }
        } else if let Ok(real) = std::fs::canonicalize(start)
            && real != Path::new("/")
            && depth < 2048
        {
            found |= self.search_upward(&start.join(".."), depth + 1);
        }
        let file = start.join(".gitignore");
        if file.is_file() {
            found |= self.push_file(start, &file);
        }
        found
    }

    /// tree's filtercheck: some remove pattern matches and no `!`
    /// pattern does. The exact `== 1` excludes syntax errors, like C.
    pub fn check(&self, path: &[u8], name: &[u8], isdir: bool) -> bool {
        let hit = self.stack.iter().any(|ig| {
            ig.remove.iter().any(|p| {
                if p.relative {
                    patmatch(name, &p.pattern, isdir, false) == 1
                } else {
                    patmatch(path, &join_path(&ig.base, &p.pattern), isdir, false) == 1
                }
            })
        });
        if !hit {
            return false;
        }
        !self.stack.iter().any(|ig| {
            ig.reverse.iter().any(|p| {
                if p.relative {
                    patmatch(name, &p.pattern, isdir, false) == 1
                } else {
                    patmatch(path, &join_path(&ig.base, &p.pattern), isdir, false) == 1
                }
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::gittrim;

    #[test]
    fn gittrim_strips_terminators_and_trailing_spaces() {
        assert_eq!(gittrim(b"foo\n"), b"foo");
        assert_eq!(gittrim(b"foo\r\n"), b"foo");
        assert_eq!(gittrim(b"foo   \n"), b"foo");
        // An escaped trailing space survives, unescaped.
        assert_eq!(gittrim(b"foo\\ \n"), b"foo ");
        assert_eq!(gittrim(b"a\\#b"), b"a#b");
        assert_eq!(gittrim(b"foo\\\n"), b"foo\\");
        assert_eq!(gittrim(b"foo\\\\\n"), b"foo\\");
    }
}

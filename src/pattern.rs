// SPDX-License-Identifier: GPL-2.0-or-later
//! Byte-level port of tree's `patmatch()` glob matcher.

/// Returns tree's convention: 1 match, 0 no match, -1 syntax error.
pub fn patmatch(buf: &[u8], pat: &[u8], isdir: bool, ignore_case: bool) -> i32 {
    if let Some(bar) = pat.iter().position(|&c| c == b'|') {
        if bar == 0 || bar + 1 == pat.len() {
            return -1;
        }
        let m = patmatch(buf, &pat[..bar], isdir, ignore_case);
        if m == 0 {
            return patmatch(buf, &pat[bar + 1..], isdir, ignore_case);
        }
        return m;
    }

    // Emulate C's NUL-terminated strings: reading past the end yields 0.
    let at = |s: &[u8], i: usize| -> u8 { if i < s.len() { s[i] } else { 0 } };
    fn sub(s: &[u8], i: usize) -> &[u8] {
        if i < s.len() { &s[i..] } else { &[] }
    }
    let low = |c: u8| -> u8 {
        if ignore_case {
            c.to_ascii_lowercase()
        } else {
            c
        }
    };

    let (mut b, mut p) = (0usize, 0usize);
    let mut mtch: i32 = 1;
    let mut pprev: u8 = 0;

    while at(pat, p) != 0 && mtch != 0 {
        match at(pat, p) {
            b'[' => {
                p += 1;
                let n: i32;
                if at(pat, p) != b'^' {
                    n = 1;
                    mtch = 0;
                } else {
                    p += 1;
                    n = 0;
                }
                while at(pat, p) != b']' {
                    if at(pat, p) == b'\\' {
                        p += 1;
                    }
                    if at(pat, p) == 0 {
                        return -1;
                    }
                    if at(pat, p + 1) == b'-' {
                        let m = at(pat, p);
                        p += 2;
                        if at(pat, p) == b'\\' && at(pat, p) != 0 {
                            p += 1;
                        }
                        if low(at(buf, b)) >= low(m) && low(at(buf, b)) <= low(at(pat, p)) {
                            mtch = n;
                        }
                        if at(pat, p) == 0 {
                            p -= 1;
                        }
                    } else if low(at(buf, b)) == low(at(pat, p)) {
                        mtch = n;
                    }
                    p += 1;
                }
                b += 1;
            }
            b'*' => {
                p += 1;
                if at(pat, p) == 0 {
                    return if sub(buf, b).contains(&b'/') { 0 } else { 1 };
                }
                mtch = 0;
                // "Support" ** for .gitignore support, mostly the same as *:
                if at(pat, p) == b'*' {
                    p += 1;
                    if at(pat, p) == 0 {
                        return 1;
                    }
                    while at(buf, b) != 0 {
                        mtch = patmatch(sub(buf, b), sub(pat, p), isdir, ignore_case);
                        if mtch != 0 {
                            break;
                        }
                        // ** between two /'s is allowed to match a null /:
                        if pprev == b'/' && at(pat, p) == b'/' && at(pat, p + 1) != 0 {
                            let m = patmatch(sub(buf, b), sub(pat, p + 1), isdir, ignore_case);
                            if m != 0 {
                                return m;
                            }
                        }
                        b += 1;
                        while at(buf, b) != 0 && at(buf, b) != b'/' {
                            b += 1;
                        }
                    }
                } else {
                    while at(buf, b) != 0 {
                        mtch = patmatch(sub(buf, b), sub(pat, p), isdir, ignore_case);
                        b += 1;
                        if mtch != 0 {
                            break;
                        }
                        if at(buf, b) == b'/' {
                            break;
                        }
                    }
                }
                if mtch == 0 && (at(buf, b) == 0 || at(buf, b) == b'/') {
                    mtch = patmatch(sub(buf, b), sub(pat, p), isdir, ignore_case);
                }
                return mtch;
            }
            b'?' => {
                if at(buf, b) == 0 {
                    return 0;
                }
                b += 1;
            }
            b'/' => {
                if at(pat, p + 1) == 0 && at(buf, b) == 0 {
                    return isdir as i32;
                }
                mtch = i32::from(at(buf, b) == at(pat, p));
                b += 1;
            }
            _ => {
                let mut c = at(pat, p);
                if c == b'\\' {
                    p += 1;
                    c = at(pat, p);
                }
                mtch = i32::from(low(at(buf, b)) == low(c));
                b += 1;
            }
        }
        pprev = at(pat, p);
        p += 1;
        if mtch < 1 {
            return mtch;
        }
    }
    if at(buf, b) == 0 { mtch } else { 0 }
}

/// tree's patinclude/patignore. With `check_paths`, every path
/// component suffix is also tried.
pub fn any_match(
    name: &[u8],
    patterns: &[Vec<u8>],
    isdir: bool,
    ignore_case: bool,
    check_paths: bool,
) -> bool {
    for pat in patterns {
        if patmatch(name, pat, isdir, ignore_case) == 1 {
            return true;
        }
        if check_paths {
            let mut rest = name;
            while let Some(sep) = rest.iter().position(|&c| c == b'/') {
                rest = &rest[sep + 1..];
                if patmatch(rest, pat, isdir, ignore_case) == 1 {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::patmatch;

    fn m(buf: &str, pat: &str) -> i32 {
        patmatch(buf.as_bytes(), pat.as_bytes(), false, false)
    }

    #[test]
    fn literal() {
        assert_eq!(m("i", "i"), 1);
        assert_eq!(m("i", "j"), 0);
        assert_eq!(m("", ""), 1);
        assert_eq!(m("abc", "ab"), 0);
    }

    #[test]
    fn star() {
        assert_eq!(m("main.c", "*.c"), 1);
        assert_eq!(m("main.h", "*.c"), 0);
        assert_eq!(m("anything", "*"), 1);
        assert_eq!(m("a.tar.gz", "*.gz"), 1);
        // A single * does not cross path separators.
        assert_eq!(patmatch(b"a/b", b"*", false, false), 0);
    }

    #[test]
    fn question_and_class() {
        assert_eq!(m("cat", "c?t"), 1);
        assert_eq!(m("ct", "c?t"), 0);
        assert_eq!(m("a1", "a[0-9]"), 1);
        assert_eq!(m("ax", "a[0-9]"), 0);
        assert_eq!(m("ax", "a[^0-9]"), 1);
    }

    #[test]
    fn alternation() {
        assert_eq!(m("foo.c", "*.c|*.h"), 1);
        assert_eq!(m("foo.h", "*.c|*.h"), 1);
        assert_eq!(m("foo.o", "*.c|*.h"), 0);
        // Leading/trailing bar is a syntax error: -1.
        assert_eq!(m("x", "|x"), -1);
    }

    #[test]
    fn ignore_case() {
        assert_eq!(patmatch(b"README", b"readme", false, true), 1);
        assert_eq!(patmatch(b"README", b"readme", false, false), 0);
    }

    #[test]
    fn dir_slash() {
        // A trailing slash in the pattern matches only directories.
        assert_eq!(patmatch(b"src", b"src/", true, false), 1);
        assert_eq!(patmatch(b"src", b"src/", false, false), 0);
    }
}

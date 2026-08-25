# Compatibility with tree

The reference implementation is the pinned commit of the [`ref-v2.3.2` branch of Uunixtree/reference-tree](https://github.com/Uunixtree/reference-tree/tree/ref-v2.3.2), which stacks **tree v2.3.2** plus the bug fixes we have submitted upstream — the branch's commit log, one commit per upstream request, is the list of those fixes; the pin moves back to the upstream repository once they are merged and released. For supported options, utree's stdout is byte-identical to the reference under `LC_ALL=C` — that is what `testsuite/` verifies. This file records every way utree and the *upstream* tree can behave differently.

## Unimplemented

tree leans on C library facilities that utree does not reimplement, and carries a few features utree has left out. Each entry names what is missing and the visible consequence. An entry stays here only for one of two reasons: implementing it faithfully is not possible within safe Rust and std, or its implementation cost is out of proportion to the rest of the port.

### Locale collation: sorting is always byte order

tree sorts names with `strcoll()`, so its output order depends on the current locale. utree always sorts by byte value, which equals tree's behavior under `LC_ALL=C`. The testsuite pins `LC_ALL=C` for both binaries.

### nl_langinfo: charset locale detection reads the environment

utree carries tree's full line-drawing table — all 16 charset families (Shift_JIS, EUC-JP, KOI8-R, ...) with their aliases, byte for byte. The divergence is only in the last step of the selection priority `--charset` → `TREE_CHARSET` → locale: tree asks `nl_langinfo(CODESET)`, which libc is free to derive from locale data, while utree looks for a UTF-8 marker in the `LC_ALL`/`LC_CTYPE`/`LANG` variables themselves. A locale whose codeset is not literally spelled in the variable (say `ja_JP` resolving to EUC-JP) therefore falls back to ASCII where tree would pick that charset's table. An explicit `--charset` or `TREE_CHARSET` always agrees with tree.

### NSS: uid/gid names come from /etc/passwd and /etc/group

tree resolves -u/-g names through getpwuid/getgrgid, which honors NSS (LDAP, sssd, ...). utree parses /etc/passwd and /etc/group directly and falls back to the numeric id, so names served only by NSS sources print numerically.

### libc strftime: -D dates and --timefmt use chrono

Local-time conversion and strftime come from the chrono crate rather than libc. Timezone rules are read from the same tzdata, but a --timefmt with conversion specifiers chrono does not know prints the format string verbatim instead of libc's implementation-defined output.

### Unsupported options fail loudly

tree options that utree recognizes but does not implement yet print an error to stderr and exit 1. utree never silently ignores an option or produces output that differs from what tree would print for the same invocation. They are:

#### --fromfile, --fromtabfile

- function: build the tree from a text listing (`find` output, or a tab-indented file) instead of walking the filesystem
- why unimplemented: an entire second input mode that bypasses the walker — a parser, a tree builder and its own quirk set, roughly the size of the walking core

#### --metafirst

- function: print the metadata column before the indentation lines
- why unimplemented: reorders the line layout in every output backend; cost spans the whole output path rather than one module

#### --condense

- function: collapse directory "singletons" — a directory whose only entry is another directory — onto one line, recursively
- why unimplemented: forces every walk into full-tree mode and restructures emission and the directory counts; the cost cuts across the walker and every backend

#### --compress

- function: compress the indentation lines (levels 1 to 3, negative values also removing the per-level space)
- why unimplemented: changes the indent geometry of the text backend and the indent-level arithmetic of the JSON and XML backends; a cross-backend layout change like --metafirst

#### --hyperlink, --scheme, --authority

- function: wrap names in OSC 8 terminal hyperlinks (`file://` URLs with configurable scheme and host)
- why unimplemented: the default authority is the hostname, which std does not expose (a crate or unsafe libc call), plus realpath/URL-escape plumbing through every name-printing site

#### --acl

- function: mark files carrying POSIX ACLs with `+` in the permission column
- why unimplemented: requires `listxattr`, which std does not expose; would cost unsafe code or an xattr dependency

#### --selinux

- function: print SELinux security contexts
- why unimplemented: same xattr constraint

#### --fflinks

- function: process symlinked files' information
- why unimplemented: barely documented even upstream; the intended semantics would have to be reverse-engineered from the C source before a faithful port is possible

#### --opt-toggle

- function: make a repeated option toggle off instead of staying set
- why unimplemented: turns every flag assignment in the parser into a toggle; a parser-wide semantics rewrite

The one silent exception is tree's STDDATA_FD handshake (Linux: JSON is emitted automatically when file descriptor 3 passes a stat test); utree does not detect it, so the listing simply stays in text form. Probing and adopting an arbitrary inherited descriptor has no safe-Rust route (`File::from_raw_fd` is unsafe), and erroring out would break the normal case, so this one stays silent.

## Deliberate differences

### Error messages and --help/--version name utree

Diagnostics are prefixed `utree:` instead of `tree:`, and the `--help`/`--version` text is utree's own. Trailing-line output (the `N directories, M files` report) and in-tree annotations (`[error opening dir]`, `[N entries exceeds filelimit, not opening dir]`, `[recursive, not followed]`) are byte-identical to tree. The one place stdout keeps tree's name is `-H`: the HTML header and footer identify the generator as tree v2.3.2 verbatim, banner and all, because the HTML output is byte-compared against the reference.


## tree quirks utree reproduces

Surprising upstream behavior that may or may not be intended; utree reproduces it pending clarification.

- -R sub-listings list their own 00Tree.html (the output file is created before the walk, like tree's setoutput()).
- Which of several symlinks to one target gets tagged `[recursive, not followed]` depends on visit order, and tree's two walking modes differ: plain listings register in sorted order, `--prune`/`--matchdirs`/`--du` in `readdir()` order. utree mirrors both.
- Unreadable subdirectories drive the exit status to 2 in the plain walk but not under `--du`/`--prune`/`--matchdirs`. The maintainer has said the counting itself will be removed ([#51](https://github.com/Old-Man-Programmer/tree/pull/51)); utree mirrors the current behavior until that lands.

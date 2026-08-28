# Open upstream pull requests and utree

Every open pull request on [Old-Man-Programmer/tree](https://github.com/Old-Man-Programmer/tree/pulls) and merge request on [OldManProgrammer/unix-tree](https://gitlab.com/OldManProgrammer/unix-tree/-/merge_requests), with utree's position on each. Rows disappear when the upstream request is merged or closed. Status values:

- **fixed** — the fix is applied to utree and carried by the reference pin (the [ref-v2.3.2 commit log](https://github.com/Uunixtree/reference-tree/commits/ref-v2.3.2) is the list of carried fixes)
- **mirrors** — utree currently reproduces the upstream behavior
- **n/a** — does not concern utree (feature proposal, C-specific cleanup, build/infra, platforms, unimplemented options)

## GitHub pull requests

| PR | Subject | utree |
|---|---|---|
| [#54](https://github.com/Old-Man-Programmer/tree/pull/54) | --info comment lines ignore -i | fixed |
| [#53](https://github.com/Old-Man-Programmer/tree/pull/53) | --prune hides directories at the -L cutoff | fixed |
| [#52](https://github.com/Old-Man-Programmer/tree/pull/52) | -R sub-listing indent state | fixed |
| [#50](https://github.com/Old-Man-Programmer/tree/pull/50) | glob syntax error counts as match | fixed |
| [#49](https://github.com/Old-Man-Programmer/tree/pull/49) | -J trailing comma, unopenable root | fixed |
| [#48](https://github.com/Old-Man-Programmer/tree/pull/48) | -J missing comma after empty root | fixed |
| [#47](https://github.com/Old-Man-Programmer/tree/pull/47) | -J spurious "contents" after error | fixed |
| [#42](https://github.com/Old-Man-Programmer/tree/pull/42) | test framework | n/a — utree has its own differential suite |
| [#41](https://github.com/Old-Man-Programmer/tree/pull/41) | --infofile absolute-path patterns | fixed |
| [#39](https://github.com/Old-Man-Programmer/tree/pull/39) | --stats flag | n/a — feature proposal |
| [#37](https://github.com/Old-Man-Programmer/tree/pull/37) | remove C99 code | n/a — C cleanup |
| [#36](https://github.com/Old-Man-Programmer/tree/pull/36) | const qualifier warning | n/a — C cleanup |
| [#28](https://github.com/Old-Man-Programmer/tree/pull/28) | --focus-root feature | n/a — feature proposal |
| [#19](https://github.com/Old-Man-Programmer/tree/pull/19) | GitHub CI | n/a — infra |
| [#12](https://github.com/Old-Man-Programmer/tree/pull/12) | z/OS support | n/a — platform |
| [#6](https://github.com/Old-Man-Programmer/tree/pull/6) | Android NDK build | n/a — platform |

## GitLab merge requests

| MR | Subject | utree |
|---|---|---|
| [!33](https://gitlab.com/OldManProgrammer/unix-tree/-/merge_requests/33) | empty root missing from the report totals | fixed |
| [!32](https://gitlab.com/OldManProgrammer/unix-tree/-/merge_requests/32) | full-tree ignore/info stack leak | fixed |
| [!31](https://gitlab.com/OldManProgrammer/unix-tree/-/merge_requests/31) | README https links | n/a — docs |
| [!30](https://gitlab.com/OldManProgrammer/unix-tree/-/merge_requests/30) | exit status documentation | n/a — docs/refactor; the behavior half is #51 |
| [!29](https://gitlab.com/OldManProgrammer/unix-tree/-/merge_requests/29) | macros to inline functions | n/a — C cleanup |
| [!28](https://gitlab.com/OldManProgrammer/unix-tree/-/merge_requests/28) | time_t portability | n/a — no behavior change; utree uses chrono |
| [!23](https://gitlab.com/OldManProgrammer/unix-tree/-/merge_requests/23) | individual headers | n/a — C cleanup |
| [!21](https://gitlab.com/OldManProgrammer/unix-tree/-/merge_requests/21) | POSIX.1-2001 standardization | n/a — C cleanup |
| [!20](https://gitlab.com/OldManProgrammer/unix-tree/-/merge_requests/20) | remove unnecessary code | n/a — dead code only |
| [!19](https://gitlab.com/OldManProgrammer/unix-tree/-/merge_requests/19) | test framework | n/a — utree has its own differential suite |
| [!13](https://gitlab.com/OldManProgrammer/unix-tree/-/merge_requests/13) | --fromfile buffer overflow | n/a — --fromfile unimplemented; the bug class cannot occur in safe Rust |

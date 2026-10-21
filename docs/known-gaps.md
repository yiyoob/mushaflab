# Known gaps

Everything unmatched, unaligned, unverified or deferred is listed here. Never hide a gap
by omission. Remove an entry only when its resolution is committed, and cite the commit.

| ID | Area | Gap | Since | Resolve by |
|---|---|---|---|---|
| G-001 | Approvals | CI does not yet verify that changes under `data/approvals/` are in owner-signed commits. Only CODEOWNERS is in place. The owner has no git signing key configured yet. The directory is empty. | T0.1 | Before the first approval is written (before GATE G2) |
| G-002 | Licensing | `license_check` treats every tracked file under `data/sources/raw/` and `fonts/` as a violation. It does not yet read `LICENSES.md` to allow cleared fonts. | T0.1 | T0.7 |

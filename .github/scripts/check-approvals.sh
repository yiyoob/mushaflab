#!/usr/bin/env bash
# Owner-only paths (CLAUDE.md rule 4) may change only in commits signed by an allowed signer.
#
#   .github/scripts/check-approvals.sh <git revision range>
#
# Each commit is verified against .github/allowed_signers as it was in that commit's
# parent, so a commit cannot add a signer and use it at the same time. The commit that
# first creates the file is verified against the file it adds (bootstrap, owner-signed).
# Commits that are ancestors of the bootstrap predate the scheme and are skipped; any later
# commit that touches owner-only paths with no signer list is rejected.
set -euo pipefail

range="$1"
signers=".github/allowed_signers"
protected=(data/approvals "$signers" .github/scripts/check-approvals.sh)
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
fail=0
bootstrap="$(git log --diff-filter=A --format=%H -- "$signers" | tail -n 1)"

for sha in $(git rev-list --reverse "$range"); do
    touched="$(git diff-tree --root --no-commit-id --name-only -r "$sha" -- "${protected[@]}")"
    [ -z "$touched" ] && continue
    if git cat-file -e "$sha^:$signers" 2>/dev/null; then
        git show "$sha^:$signers" > "$tmp"
    elif git cat-file -e "$sha:$signers" 2>/dev/null; then
        git show "$sha:$signers" > "$tmp"   # bootstrap: the commit that creates the file
    elif [ -n "$bootstrap" ] && git merge-base --is-ancestor "$sha" "$bootstrap"; then
        echo "skip $sha: before the signing scheme existed"
        continue
    else
        echo "FAIL $sha: changes owner-only paths but no $signers exists" >&2
        fail=1
        continue
    fi
    if git -c gpg.ssh.allowedSignersFile="$tmp" verify-commit "$sha" >/dev/null 2>&1; then
        echo "ok   $sha: owner-signed ($(echo "$touched" | tr '\n' ' '))"
    else
        echo "FAIL $sha: changes owner-only paths without an owner signature: $(echo "$touched" | tr '\n' ' ')" >&2
        fail=1
    fi
done
exit "$fail"

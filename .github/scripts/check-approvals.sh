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
#
# Paths in `protected` are owner-only from the bootstrap on. Paths in `added` were made
# owner-only later (G-130): each is protected from the commit that first lists it in `added`
# in this script (its protecting commit, found in the history of HEAD). A commit that is a
# strict ancestor of a path's protecting commit predates its protection and is skipped for
# that path only; the protecting commit itself and every commit not in its ancestry are
# checked. A path whose protecting commit is not in HEAD's history (the list was edited but
# not committed yet) is not protected yet. Each `added` entry stays on its own line, exactly
# four spaces then the path, because that line is how the protecting commit is found.
set -euo pipefail

range="$1"
signers=".github/allowed_signers"
script=".github/scripts/check-approvals.sh"
protected=(data/approvals "$signers" "$script")
added=(
    data/encodings/hafs-v1.owner-rows.json
    data/encodings/hafs-v1.owner-words.json
    data/encodings/hafs-v1.riwayat-owner.json
    data/render/kfgqpc-rule6-exceptions.json
    data/layouts/uthmani-1441h.corrections.json
)
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
fail=0
bootstrap="$(git log --diff-filter=A --format=%H -- "$signers" | tail -n 1)"

# Protecting commit of each `added` path: the oldest commit of this script that lists it.
since=()
for p in "${added[@]}"; do
    first=""
    for c in $(git log --reverse --format=%H HEAD -- "$script"); do
        if git show "$c:$script" 2>/dev/null | grep -qxF "    $p"; then
            first="$c"
            break
        fi
    done
    since+=("$first")
done

for sha in $(git rev-list --reverse "$range"); do
    touched="$(git diff-tree --root --no-commit-id --name-only -r "$sha" -- "${protected[@]}")"
    for i in "${!added[@]}"; do
        p="${added[$i]}" s="${since[$i]}"
        [ -z "$(git diff-tree --root --no-commit-id --name-only -r "$sha" -- "$p")" ] && continue
        if [ -z "$s" ]; then
            continue
        elif [ "$sha" != "$s" ] && git merge-base --is-ancestor "$sha" "$s"; then
            echo "skip $sha: $p before its protection (${s:0:7})"
            continue
        fi
        touched="$(printf '%s\n%s' "$touched" "$p" | sed '/^$/d')"
    done
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

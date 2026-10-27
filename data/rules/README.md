# Rules (T6.1, ADR D38)

A rule is a declarative pattern over the clusters of the Ḥafṣ spine, possibly across
words. Its actions are patch templates. `mushaf-rules` expands every match into ordinary
patches, and the resolver applies them like any patch: same layers, same conflict and orphan
reports. A riwāyah profile lists its rule files in `rule_stack`.

Rules are data. Like every Qurʾānic fact, a rule is written only from a cited source
(CLAUDE.md rule 1). Each rule has an entry in `register.yaml` with its description, its
source (book, volume, page) and its status (`rule-derived`, or `hypothesis` when it is not
yet cited).

## File format (`rules/<name>.yaml`, ASCII, codepoints as `U+XXXX`)

```yaml
spine: "hafs/v1"
rules:
  - id: "warsh.example"                    # lowercase, digits, . _ -
    owner: {level: "riwayah", id: "warsh"}   # qiraah | riwayah | tariq
    when: {option: "group.id", choice: "x"}  # optional: the option layer (list the rule id in the group's affects)
    pattern:                               # starts and ends with a cluster
      - {base: ["U+0646"], marks_any: ["U+06E1"], at: "last", capture: "n"}
      - {boundary: "word"}                 # word | ayah_end | any
      - {base: ["U+0628"], capture: "b"}
    scope: [{surah: 2, from: 1, to: 20}]   # optional: where a match may start
    except: [{cluster: "2:5:3:1", reason: "cited exception"}]
    actions:                               # op and payload exactly as in a patch file
      - {op: "set_marks", target: "n", payload: {remove: ["U+06E1"]}}
      - {op: "replace", target: "b.item", payload: {sep_after: "U+0020"}}
```

A cluster matcher can use any of the following. Empty lists do not constrain.

- `base`: allowed first codepoints.
- `kind`: the cluster kind.
- `marks_all`, `marks_any`, `marks_none`: marks that must all be present, at least one
  present, or none present.
- `bare`: no marks at all.
- `at`: the cluster's position in its word: `first`, `last` or `only`.
- `capture`: a name that actions can target.

Consecutive matchers take consecutive clusters of one item. A boundary moves from an item's
last cluster to the first cluster of the next word:

- `word`: the next word in the same ayah. Hizb signs in between are skipped.
- `ayah_end`: the first word after an ayah end.
- `any`: the next word, whether or not an ayah ends in between.

Matching rules:

- Matches are leftmost and non-overlapping, in spine order.
- Codepoints are compared exactly; nothing is normalised.
- A match that touches a cluster named in `except` is skipped.

In data builds, a rule that matches nowhere, or an exception that is never met, is an error.

Each match's patches get the id `<rule id>@<target>.<op>`. They also get
`source: {kind: rule, ref: <rule id>}`, `rule_id` and `status: rule-derived`, and the rule's
`when` is copied onto them.

## Pronunciation-only rules (annotation channel, D40)

A feature that changes no glyph, such as madd lengths, taqlīl or tarqīq, is written with
`set_attr` to the cluster's annotations:
`{op: "set_attr", target: "x", payload: {annotations: {"pron.madd": "4"}}}`. Keys are
`pron.<feature>`, with the feature taken from the closed list in D40. Values are short ASCII
tokens. Annotations never change the text or the drawing.

## Limits (known gaps)

Rules match the spine's own text. They do not match the output of other rules or of farsh
patches.

//! T2.4: option groups (§3.3, D12). A choice comes from the user's selection, else the ṭarīq's
//! default, else the riwāyah's default; with none of these the resolution fails rather than
//! guess. Only groups that the stack's patches depend on need a choice. `allowed_for` is
//! advisory: it yields lint notes and never blocks (rule 11).

use std::collections::{BTreeMap, BTreeSet};

use mushaf_model::{EditOverlay, OptionGroup, OwnerLevel, Patch, PatchFile, Spine};
use serde::Serialize;

use crate::{Mode, ResolveError, Resolved, resolve_patches};

/// The riwāyah, ṭarīq and option choices to resolve for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub riwayah: String,
    pub tariq: Option<String>,
    /// User choices by group id; they override defaults.
    pub choices: BTreeMap<String, String>,
}

/// Advisory findings that never fail a resolution.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum LintNote {
    /// The group is used for a riwāyah outside its `allowed_for` list.
    OptionNotAllowed { group: String, riwayah: String },
}

fn choice_for(g: &OptionGroup, sel: &Selection) -> Result<String, ResolveError> {
    if let Some(c) = sel.choices.get(&g.id) {
        return Ok(c.clone());
    }
    g.default_for(&sel.riwayah, sel.tariq.as_deref())
        .map(String::from)
        .ok_or_else(|| ResolveError::NoChoice {
            group: g.id.clone(),
            riwayah: sel.riwayah.clone(),
            tariq: sel.tariq.clone(),
        })
}

/// Resolves a stack for a selection: data patch files, then the user's edit overlay (T2.5),
/// which is the only source of the `user` layer. Picks the active option patches, then
/// applies everything with `resolve_patches`.
pub fn resolve(
    spine: &Spine,
    files: &[&PatchFile],
    groups: &BTreeMap<String, OptionGroup>,
    sel: &Selection,
    overlay: Option<&EditOverlay>,
    mode: Mode,
) -> Result<Resolved, ResolveError> {
    let group = |id: &str| {
        groups
            .get(id)
            .ok_or_else(|| ResolveError::UnknownOption { group: id.into() })
    };
    let known_choice = |g: &OptionGroup, c: &str| {
        if g.choices.iter().any(|x| x == c) {
            Ok(())
        } else {
            Err(ResolveError::UnknownChoice {
                group: g.id.clone(),
                choice: c.into(),
            })
        }
    };
    if let Some(f) = files.iter().find(|f| f.spine != spine.id) {
        return Err(ResolveError::SpineMismatch {
            file: f.spine.clone(),
            spine: spine.id.clone(),
        });
    }
    if let Some(o) = overlay.filter(|o| o.spine != spine.id) {
        return Err(ResolveError::SpineMismatch {
            file: o.spine.clone(),
            spine: spine.id.clone(),
        });
    }
    let data = files.iter().flat_map(|f| &f.patches);
    if let Some(p) = data.clone().find(|p| p.owner.level == OwnerLevel::User) {
        return Err(ResolveError::UserPatchInDataFile {
            patch: p.id.clone(),
        });
    }
    for (id, c) in &sel.choices {
        known_choice(group(id)?, c)?;
    }
    let patches: Vec<&Patch> = data
        .chain(overlay.iter().flat_map(|o| &o.patches))
        .collect();
    let mut used: BTreeSet<&str> = sel.choices.keys().map(String::as_str).collect();
    for p in &patches {
        if let Some(w) = &p.when {
            let g = group(&w.option)?;
            known_choice(g, &w.choice)?;
            if !g.affects.contains(&p.id) {
                return Err(ResolveError::NotInAffects {
                    group: g.id.clone(),
                    patch: p.id.clone(),
                });
            }
            used.insert(&g.id);
        }
    }
    let mut choices = BTreeMap::new();
    let mut lint = Vec::new();
    for id in used {
        let g = group(id)?;
        choices.insert(g.id.clone(), choice_for(g, sel)?);
        if !g.allowed_for.is_empty() && !g.allowed_for.contains(&sel.riwayah) {
            lint.push(LintNote::OptionNotAllowed {
                group: g.id.clone(),
                riwayah: sel.riwayah.clone(),
            });
        }
    }
    let active: Vec<&Patch> = patches
        .into_iter()
        .filter(|p| {
            p.when
                .as_ref()
                .is_none_or(|w| choices[&w.option] == w.choice)
        })
        .collect();
    let mut r = resolve_patches(spine, &active, mode)?;
    r.choices = choices;
    r.lint = lint;
    Ok(r)
}

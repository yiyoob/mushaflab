//! The user edit overlay: the highest-priority layer (`user`), stored as an ordinary patch file
//! whose patches all have `owner.level: user` and `status: user-edit` (D12).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{Error, OwnerLevel, Patch, PatchFile};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "PatchFile", into = "PatchFile")]
pub struct EditOverlay {
    pub spine: String,
    pub patches: Vec<Patch>,
}

impl EditOverlay {
    pub fn new(spine: impl Into<String>) -> Self {
        EditOverlay {
            spine: spine.into(),
            patches: Vec::new(),
        }
    }

    /// Adds a user patch, replacing the overlay's patch with the same key (one patch per
    /// target per layer). Returns the replaced patch, if any.
    pub fn upsert(&mut self, patch: Patch) -> Result<Option<Patch>, Error> {
        patch.validate()?;
        if patch.owner.level != OwnerLevel::User {
            return Err(Error::new(format!(
                "patch {:?} is not owner.level user",
                patch.id
            )));
        }
        let key = patch.key();
        if let Some(old) = self.patches.iter_mut().find(|p| p.key() == key) {
            return Ok(Some(std::mem::replace(old, patch)));
        }
        if self.patches.iter().any(|p| p.id == patch.id) {
            return Err(Error::new(format!(
                "overlay already has a patch {:?}",
                patch.id
            )));
        }
        self.patches.push(patch);
        Ok(None)
    }
}

impl TryFrom<PatchFile> for EditOverlay {
    type Error = Error;
    fn try_from(f: PatchFile) -> Result<Self, Error> {
        if let Some(p) = f.patches.iter().find(|p| p.owner.level != OwnerLevel::User) {
            return Err(Error::new(format!(
                "overlay patch {:?} is not owner.level user",
                p.id
            )));
        }
        let mut keys = BTreeSet::new();
        if let Some(p) = f.patches.iter().find(|p| !keys.insert(p.key())) {
            return Err(Error::new(format!(
                "overlay has two patches for {} (one patch per target per layer)",
                p.key()
            )));
        }
        Ok(EditOverlay {
            spine: f.spine,
            patches: f.patches,
        })
    }
}

impl From<EditOverlay> for PatchFile {
    fn from(o: EditOverlay) -> Self {
        PatchFile {
            spine: o.spine,
            patches: o.patches,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{from_json, from_yaml, to_json, to_yaml};

    const Y: &str = "spine: \"t/v1\"\npatches:\n  - id: \"u1\"\n    owner: {level: \"user\", id: \"me\"}\n    op: \"set_marks\"\n    target: \"1:1:1:1\"\n    payload: {add: [\"U+06DC\"]}\n    source: {kind: \"user\", ref: \"session\"}\n    status: \"user-edit\"\n  - id: \"u2\"\n    owner: {level: \"user\", id: \"me\"}\n    op: \"insert_after\"\n    target: \"1:1:1\"\n    payload: {seq: 1, item: {kind: \"word\", sep_after: \"U+0020\", clusters: [{kind: \"letter\", cps: \"U+0078\"}]}}\n    source: {kind: \"user\", ref: \"session\"}\n    status: \"user-edit\"\n";

    #[test]
    fn serializes_as_a_patch_file_byte_stably() {
        let o: EditOverlay = from_yaml(Y).unwrap();
        let yaml = to_yaml(&o).unwrap();
        assert_eq!(
            to_yaml(&from_yaml::<EditOverlay>(&yaml).unwrap()).unwrap(),
            yaml
        );
        let json = to_json(&o).unwrap();
        assert_eq!(
            to_json(&from_json::<EditOverlay>(&json).unwrap()).unwrap(),
            json
        );
        // the overlay file is an ordinary patch file
        let f: PatchFile = from_yaml(&yaml).unwrap();
        assert_eq!(EditOverlay::try_from(f).unwrap(), o);
        // ids are quoted, so YAML 1.1 readers cannot turn them into numbers
        assert!(
            yaml.contains("'1:1:1:1'") || yaml.contains("\"1:1:1:1\""),
            "{yaml}"
        );
    }

    #[test]
    fn only_user_patches_once_per_key() {
        assert!(
            from_yaml::<EditOverlay>(
                &Y.replace(
                    "level: \"user\", id: \"me\"}\n    op: \"set_marks\"",
                    "level: \"riwayah\", id: \"me\"}\n    op: \"set_marks\""
                )
                .replacen("user-edit", "raw-derived", 1)
            )
            .is_err()
        );
        assert!(from_yaml::<EditOverlay>(&Y.replace("insert_after\"\n    target: \"1:1:1\"", "set_marks\"\n    target: \"1:1:1:1\"").replace("{seq: 1, item: {kind: \"word\", sep_after: \"U+0020\", clusters: [{kind: \"letter\", cps: \"U+0078\"}]}}", "{add: [\"U+0301\"]}")).is_err());
    }

    #[test]
    fn upsert_keeps_one_patch_per_key() {
        let mut o: EditOverlay = from_yaml(Y).unwrap();
        let mut p = o.patches[0].clone();
        p.id = "u3".into();
        let old = o.upsert(p.clone()).unwrap().unwrap();
        assert_eq!(
            (old.id.as_str(), o.patches.len(), o.patches[0].id.as_str()),
            ("u1", 2, "u3")
        );
        p.target = "1:1:1:2".parse().unwrap();
        assert!(o.upsert(p.clone()).is_err(), "same id, different key");
        p.id = "u4".into();
        assert!(o.upsert(p.clone()).unwrap().is_none());
        p.owner.level = OwnerLevel::Riwayah;
        p.id = "u5".into();
        assert!(o.upsert(p).is_err());
    }
}

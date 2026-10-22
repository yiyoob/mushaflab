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

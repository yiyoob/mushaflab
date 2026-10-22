//! T2.3: identity on the real tracked Hafs spine. With no patches, every one of its items and
//! clusters comes back unchanged, in order (data/spine/hafs/v1, counts from its manifest).

mod common;

use common::{as_spine_items, repo};
use mushaf_resolve::{Mode, resolve_patches};

#[test]
fn no_patches_gives_the_hafs_spine() {
    let ds = mushaf_db::load_dir(&repo().join("data")).unwrap();
    let spine = &ds.spines["hafs/v1"];
    let r = resolve_patches(spine, &[], Mode::Data).unwrap();
    assert_eq!(r.items.len(), 6236 + 199 + 77430);
    assert!(
        as_spine_items(&r) == spine.items,
        "identity resolution changed the spine"
    );
}

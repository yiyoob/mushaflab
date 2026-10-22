//! T2.3 property tests on the synthetic spine (fixed seed, so CI runs are reproducible):
//! valid patch sets never make free mode fail (rule 11), resolution is deterministic and
//! independent of patch order, and a second patch on a key in the same layer is a conflict.

mod common;

use std::collections::BTreeSet;

use common::spine;
use mushaf_model::{
    AttrChange, ClusterKind, Cp, Cps, ItemKind, Layer, MarkAdd, MarkAt, NewCluster, NewItem,
    NewNode, Op, Owner, OwnerLevel, Patch, Ref, Side, Source, Spine, Status, When,
};
use mushaf_resolve::{Mode, ResolveError, resolve_patches};
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};

fn config() -> Config {
    Config {
        cases: 300,
        failure_persistence: None,
        rng_seed: RngSeed::Fixed(0x7265_736f_6c76),
        ..Config::default()
    }
}

fn cps(s: &str) -> Cps {
    Cps::from_text(s)
}

/// (layer, item index, cluster index, insert seq in the target, op choice, seq, side, letter)
type Spec = (u8, usize, Option<u32>, Option<u32>, u8, u32, bool, char);

fn build(spine: &Spine, n: usize, s: Spec) -> Patch {
    let (layer, item, cluster, ins, op, seq, before, letter) = s;
    let mut it = spine.items[item % spine.items.len()].id.clone();
    let target = match (cluster, ins) {
        (None, None) => Ref::Item(it),
        (None, Some(k)) => Ref::Item(it.inserted(k)),
        (Some(c), ins) => {
            if ins == Some(9) {
                it = it.inserted(1);
            }
            let c = it.cluster(c);
            Ref::Cluster(match ins {
                Some(k) if k < 9 => c.inserted(k),
                _ => c,
            })
        }
    };
    let side = if before { Side::Before } else { Side::After };
    let letter = letter.to_string();
    let op = match (&target, op % 6) {
        (_, 0) => Op::Delete,
        (Ref::Item(_), 1) => Op::ReplaceItem {
            sep_before: None,
            sep_after: Some(cps(" ")),
        },
        (Ref::Item(_), 2 | 3) => Op::Insert {
            side,
            seq,
            node: NewNode::Item(NewItem {
                kind: ItemKind::Word,
                sep_before: Cps::default(),
                sep_after: cps(" "),
                clusters: vec![NewCluster {
                    kind: ClusterKind::Letter,
                    cps: cps(&letter),
                }],
            }),
        },
        (Ref::Item(_), _) => Op::SetAttr(AttrChange {
            width_em_delta: Some(seq as i32),
            ..AttrChange::default()
        }),
        (Ref::Cluster(_), 1) => Op::ReplaceCluster(NewCluster {
            kind: ClusterKind::Letter,
            cps: cps(&letter),
        }),
        (Ref::Cluster(_), 2) => Op::Insert {
            side,
            seq,
            node: NewNode::Cluster(NewCluster {
                kind: ClusterKind::Letter,
                cps: cps(&letter),
            }),
        },
        (Ref::Cluster(_), 3) => Op::SetMarks {
            remove: if before { vec![Cp('\u{301}')] } else { vec![] },
            add: vec![
                MarkAdd::Append(Cp('\u{302}')),
                MarkAdd::At(MarkAt {
                    cp: Cp('\u{303}'),
                    at: seq % 3,
                }),
            ],
        },
        (Ref::Cluster(_), 4) => Op::ClearMarks,
        (Ref::Cluster(_), _) => Op::SetAttr(AttrChange {
            stretch: Some(seq as i32),
            ..AttrChange::default()
        }),
    };
    let level = [
        OwnerLevel::Qiraah,
        OwnerLevel::Riwayah,
        OwnerLevel::Tariq,
        OwnerLevel::Riwayah,
        OwnerLevel::User,
    ][layer as usize % 5];
    Patch {
        id: format!("p{n}"),
        owner: Owner {
            level,
            id: "o".into(),
        },
        target,
        op,
        source: Source {
            kind: "test".into(),
            reference: "property".into(),
        },
        status: if level == OwnerLevel::User {
            Status::UserEdit
        } else {
            Status::RawDerived
        },
        rule_id: None,
        // layer 3 is the option layer
        when: (layer % 5 == 3).then(|| When {
            option: "g".into(),
            choice: "c".into(),
        }),
        single_source: false,
    }
}

/// Keeps the first patch per (layer, key) and drops patches inside items their own layer
/// deletes, so the set is valid.
fn sanitize(ps: Vec<Patch>) -> Vec<Patch> {
    let mut keys = BTreeSet::new();
    let ps: Vec<Patch> = ps
        .into_iter()
        .filter(|p| keys.insert((p.layer(), p.key())))
        .collect();
    let deleted: BTreeSet<(Layer, Ref)> = ps
        .iter()
        .filter(|p| p.op == Op::Delete && matches!(p.target, Ref::Item(_)))
        .map(|p| (p.layer(), p.target.clone()))
        .collect();
    ps.into_iter()
        .filter(|p| {
            let item = Ref::Item(p.key().item().clone());
            !(p.key().is_inside(&item) && deleted.contains(&(p.layer(), item)))
        })
        .collect()
}

fn arb_spec() -> impl Strategy<Value = Spec> {
    (
        0u8..5,
        0usize..10,
        prop::option::weighted(0.6, 1u32..4),
        prop::option::weighted(0.25, prop_oneof![1u32..3, Just(9u32)]),
        0u8..6,
        1u32..4,
        any::<bool>(),
        prop::sample::select(vec!['x', 'y', 'z']),
    )
}

fn arb_set() -> impl Strategy<Value = (Vec<Patch>, Vec<Patch>)> {
    let sp = spine();
    prop::collection::vec(arb_spec(), 0..24).prop_flat_map(move |specs| {
        let ps = sanitize(
            specs
                .into_iter()
                .enumerate()
                .map(|(n, s)| build(&sp, n, s))
                .collect(),
        );
        (Just(ps.clone()), Just(ps).prop_shuffle())
    })
}

fn refs(ps: &[Patch]) -> Vec<&Patch> {
    ps.iter().collect()
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn valid_sets_resolve_deterministically_in_any_order((ps, shuffled) in arb_set()) {
        let sp = spine();
        let a = resolve_patches(&sp, &refs(&ps), Mode::Free);
        prop_assert!(a.is_ok(), "free mode refused a valid set: {:?}", a);
        let a = a.unwrap();
        let b = resolve_patches(&sp, &refs(&ps), Mode::Free).unwrap();
        prop_assert_eq!(serde_json::to_string(&a).unwrap(), serde_json::to_string(&b).unwrap());
        let c = resolve_patches(&sp, &refs(&shuffled), Mode::Free).unwrap();
        prop_assert_eq!(a, c);
    }

    #[test]
    fn a_second_patch_on_a_key_in_one_layer_is_a_conflict((ps, _) in arb_set(), pick in any::<prop::sample::Index>()) {
        prop_assume!(!ps.is_empty());
        let mut dup = ps[pick.index(ps.len())].clone();
        dup.id = "dup".into();
        let mut all = ps.clone();
        all.push(dup);
        let r = resolve_patches(&spine(), &refs(&all), Mode::Free);
        prop_assert!(matches!(r, Err(ResolveError::Conflict { .. })), "{:?}", r);
    }
}

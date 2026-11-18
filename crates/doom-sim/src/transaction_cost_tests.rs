//! Private structural rollback check and opt-in host cost observations.
use super::*;
#[path = "../../../tests/support/m3g1.rs"]
mod fixture;
use std::hint::black_box;
#[test]
fn projection_mutation_is_isolated_and_component_costs_are_observable() {
    for (label, profile) in [("small_control", "20"), ("canonical_small", "original")] {
        let workload = fixture::Workload::setup(label, profile).unwrap();
        let mut sim = workload.sim;
        let before = sim.snapshot();
        let mut plan = sim.demon_plan().unwrap();
        assert_eq!(
            plan.actors.order().collect::<Vec<_>>(),
            sim.actor_order().collect::<Vec<_>>()
        );
        assert_eq!(plan.spatial.order, sim.spatial.order);
        // The shared order is staged with the slots; without sector thinkers it is
        // the actor order and the empty sector class costs no allocation.
        assert_eq!(
            plan.actors.shared_order().collect::<Vec<_>>(),
            sim.actors.shared_order().collect::<Vec<_>>()
        );
        assert!(
            plan.actors
                .shared_order()
                .eq(sim.actor_order().map(ThinkerId::Actor))
        );
        assert_eq!(plan.actors.sector_capacity(), 0);
        assert_eq!(plan.actors.sector_slots.capacity(), 0);
        let first = plan.actors.order().next().unwrap();
        plan.actors.mark_remove(first).unwrap();
        plan.rng.p_random();
        plan.statistics.as_mut().unwrap().total_kills += 1;
        plan.bindings.fill(None);
        drop(plan);
        assert_eq!(sim.snapshot(), before);
    }
    if std::env::var_os("DOOM_STEPPING_COST").is_none() {
        return;
    }
    for (label, profile) in [
        ("small_control", "20"),
        ("four_owners", "20"),
        ("high_cell_demon", "20"),
        ("canonical_small", "original"),
        ("actor_capacity", "original"),
    ] {
        let mut sim = fixture::Workload::setup(label, profile).unwrap().sim;
        let before = sim.snapshot();
        let slot_bytes = std::mem::size_of_val(sim.actors.slots.as_slice());
        let spatial_copy_bytes = std::mem::size_of_val(sim.spatial.order.as_slice());
        let spatial_reserved_bytes = sim.actors.max_slots as usize * std::mem::size_of::<ActorId>();
        for operation in ["preflight", "actor_preflight", "staging"] {
            let (result, elapsed) = fixture::interval(|| -> Result<(), SimError> {
                for _ in 0..fixture::UPDATES {
                    match operation {
                        "preflight" => black_box(&sim).preflight_expanded_demons()?,
                        "actor_preflight" => black_box(&mut sim).preflight_actor_step()?,
                        _ => drop(black_box(sim.demon_plan()?)),
                    }
                }
                Ok(())
            })
            .unwrap();
            result.unwrap();
            assert_eq!(sim.snapshot(), before);
            println!(
                "transaction label={label} profile={profile} operation={operation} calls={} ns={} logical_slots={} slot_bytes={slot_bytes} spatial_copy_bytes={spatial_copy_bytes} spatial_reserved_bytes={spatial_reserved_bytes}",
                fixture::UPDATES,
                elapsed.as_nanos(),
                sim.actors.slots.len()
            );
        }
    }
}

#[test]
fn specials_plans_stage_the_sector_class_in_the_committed_buffer() {
    // pools_lights_soak: eight light thinkers in the sector-thinker class.
    let mut sim = crate::specials_evidence::setup(&crate::specials_evidence::cases()[0]).0;
    assert_eq!(sim.actors.sector_slots.len(), 8);
    for _ in 0..2 {
        sim.advance([TicCommand::default(); 4]).unwrap();
    }
    // After a commit the replaced vector is kept as the next plan's buffer, so
    // staging the sector class reuses it instead of allocating.
    let spare = sim.spare_sector_slots.as_ptr();
    assert!(sim.spare_sector_slots.capacity() >= sim.actors.sector_slots.len());
    let plan = sim.demon_plan().unwrap();
    assert_eq!(plan.actors.sector_slots.as_ptr(), spare);
    assert_eq!(plan.actors.sector_slots.len(), 8);
    assert_eq!(sim.spare_sector_slots.capacity(), 0);
    let staged = sim.snapshot();
    drop(plan);
    assert_eq!(sim.snapshot(), staged);
}

#[test]
fn plans_stage_sector_writes_in_a_lazy_copy() {
    // No policy before the movers policy writes a sector height or a line special:
    // a whole tic copies nothing, and the staging buffers never allocate.
    let mut sim = crate::specials_evidence::setup(&crate::specials_evidence::cases()[0]).0;
    for _ in 0..2 {
        sim.advance([TicCommand::default(); 4]).unwrap();
    }
    assert!(sim.spare_writes.is_empty());
    assert_eq!(sim.spare_writes, staging::StagedWrites::default());
    assert_eq!(sim.spare_writes.buffer_capacities(), (0, 0));
    assert!(sim.line_specials.is_empty());
    let table = sim.sectors.clone();
    let before = sim.snapshot();
    let moved = SectorState {
        floor: table[0].floor + Fixed::from_map_units(8),
        ceiling: table[0].ceiling,
    };
    // A write is visible through the plan's view at once, and only there; a
    // refused (dropped) plan leaves the committed table byte-identical.
    let mut plan = sim.demon_plan().unwrap();
    assert!(plan.writes.is_empty());
    plan.writes.write_sector(&sim.sectors, 0, moved).unwrap();
    let view = plan.writes.sectors(&sim.sectors);
    assert_eq!(view[0], moved);
    assert!((1..table.len()).all(|i| view[i] == table[i]));
    assert_eq!(sim.sector_view()[0], table[0]);
    drop(plan);
    assert_eq!(sim.sectors, table);
    assert_eq!(sim.snapshot(), before);
    // A committed plan swaps the copy in and keeps the replaced table as the next
    // plan's buffer, which then stages without allocating.
    let mut plan = sim.demon_plan().unwrap();
    plan.writes.write_sector(&sim.sectors, 0, moved).unwrap();
    let _ = sim.commit_demon_plan(&mut plan);
    assert_eq!(sim.sectors[0], moved);
    assert!((1..table.len()).all(|i| sim.sectors[i] == table[i]));
    assert!(sim.spare_writes.is_empty());
    let (capacity, lines) = sim.spare_writes.buffer_capacities();
    assert!(capacity >= table.len());
    assert_eq!(lines, 0);
    let mut plan = sim.demon_plan().unwrap();
    assert!(plan.writes.is_empty());
    assert_eq!(plan.writes.sectors(&sim.sectors)[0], moved);
    plan.writes.write_sector(&sim.sectors, 1, moved).unwrap();
    assert_eq!(plan.writes.buffer_capacities().0, capacity);
    let view = plan.writes.sectors(&sim.sectors);
    assert_eq!([view[0], view[1]], [moved, moved]);
    // Refusing this plan leaves the committed table of the earlier commit intact.
    let committed = sim.sectors.clone();
    let before = sim.snapshot();
    drop(plan);
    assert_eq!(sim.sectors, committed);
    assert_eq!(sim.snapshot(), before);
}

/// The M3h8 §9 staging measurement (opt-in, `DOOM_STAGING_COST=1`, release). Sectors:
/// one synthetic tic stages `k` sector writes, reads `reads` sectors through the staged
/// view and commits, against a baseline that reads the committed table directly and
/// stages nothing. Variants: `lazy`, the implementation (a full copy into a reused
/// buffer taken on the first write, swapped in on commit); `sparse`, the step-5
/// overlay (written entries, a linear scan per read), kept here for comparison;
/// `full`, an eager full copy every tic (the M3h7 `SectorSpecial` pattern); and
/// `indexed`, the sparse entries plus a reused per-sector position table (`u16::MAX`
/// when unwritten) reset only at the written positions. Line specials: the same
/// shape for the runtime line-special vector, `lazy` (the implementation) against a
/// sparse overlay of `(line, special)` entries.
#[test]
fn staging_cost_overlays_against_lazy_copies() {
    if std::env::var_os("DOOM_STAGING_COST").is_none() {
        return;
    }
    const TICS: usize = 100_000;
    const REPEATS: usize = 7;
    let median = |mut runs: Vec<u128>| {
        runs.sort_unstable();
        runs[runs.len() / 2]
    };
    let per_tic = |runs: Vec<u128>| median(runs) as f64 / TICS as f64;
    let time = |runs: &mut Vec<u128>, f: &mut dyn FnMut() -> i64| {
        let start = std::time::Instant::now();
        black_box(f());
        runs.push(start.elapsed().as_nanos());
    };
    for sectors in [88usize, 4096] {
        let mut table: Vec<SectorState> = (0..sectors)
            .map(|i| SectorState {
                floor: Fixed::from_map_units(i as i16 % 256),
                ceiling: Fixed::from_map_units(256 + i as i16 % 256),
            })
            .collect();
        for k in [0usize, 1, 2, 4, 9, 18] {
            for reads in [0usize, 256, 2048] {
                // Deterministic spread of written and read indices.
                let written: Vec<u16> = (0..k).map(|j| ((j * 37 + 5) % sectors) as u16).collect();
                let read: Vec<usize> = (0..reads).map(|r| (r * 7919 + 13) % sectors).collect();
                let state = |t: usize| SectorState {
                    floor: Fixed::from_bits(t as i32 & 0xffff),
                    ceiling: Fixed::from_map_units(512),
                };
                let mut lazy = staging::StagedWrites::default();
                let mut spare: Vec<SectorState> = Vec::new();
                let mut lines: Vec<i16> = Vec::new();
                let mut sparse: Vec<(u16, SectorState)> = Vec::new();
                let mut position = vec![u16::MAX; sectors];
                let mut entries: Vec<(u16, SectorState)> = Vec::new();
                let [
                    mut base,
                    mut lazy_ns,
                    mut sparse_ns,
                    mut full_ns,
                    mut indexed_ns,
                ] = [(); 5].map(|()| Vec::new());
                for _ in 0..REPEATS {
                    time(&mut base, &mut || {
                        let mut sum = 0i64;
                        for _ in 0..TICS {
                            let view = staging::Sectors::committed(black_box(&table));
                            for &i in &read {
                                sum += i64::from(view[i].floor.bits());
                            }
                        }
                        sum
                    });
                    time(&mut lazy_ns, &mut || {
                        let mut sum = 0i64;
                        for t in 0..TICS {
                            lazy.clear();
                            for &s in &written {
                                lazy.write_sector(&table, s, state(t)).unwrap();
                            }
                            let view = lazy.sectors(black_box(&table));
                            for &i in &read {
                                sum += i64::from(view[i].floor.bits());
                            }
                            lazy.commit(&mut table, &mut lines);
                        }
                        sum
                    });
                    time(&mut sparse_ns, &mut || {
                        let mut sum = 0i64;
                        for t in 0..TICS {
                            sparse.clear();
                            for &s in &written {
                                match sparse.iter_mut().find(|(i, _)| *i == s) {
                                    Some(entry) => entry.1 = state(t),
                                    None => {
                                        sparse.try_reserve(1).unwrap();
                                        sparse.push((s, state(t)));
                                    }
                                }
                            }
                            let table_ref = black_box(&table);
                            for &i in &read {
                                let sector = match sparse.iter().find(|(s, _)| usize::from(*s) == i)
                                {
                                    Some((_, state)) => state,
                                    None => &table_ref[i],
                                };
                                sum += i64::from(sector.floor.bits());
                            }
                            for &(s, state) in &sparse {
                                table[usize::from(s)] = state;
                            }
                        }
                        sum
                    });
                    time(&mut full_ns, &mut || {
                        let mut sum = 0i64;
                        for t in 0..TICS {
                            spare.clear();
                            spare.try_reserve(table.len()).unwrap();
                            spare.extend_from_slice(&table);
                            for &s in &written {
                                spare[usize::from(s)] = state(t);
                            }
                            let view = black_box(spare.as_slice());
                            for &i in &read {
                                sum += i64::from(view[i].floor.bits());
                            }
                            std::mem::swap(&mut table, &mut spare);
                        }
                        sum
                    });
                    time(&mut indexed_ns, &mut || {
                        let mut sum = 0i64;
                        for t in 0..TICS {
                            for &(s, _) in &entries {
                                position[usize::from(s)] = u16::MAX;
                            }
                            entries.clear();
                            for &s in &written {
                                let at = &mut position[usize::from(s)];
                                if *at == u16::MAX {
                                    *at = entries.len() as u16;
                                    entries.push((s, state(t)));
                                } else {
                                    entries[usize::from(*at)].1 = state(t);
                                }
                            }
                            let (table_ref, position_ref) =
                                (black_box(&table), black_box(&position));
                            for &i in &read {
                                let at = position_ref[i];
                                let sector = if at == u16::MAX {
                                    &table_ref[i]
                                } else {
                                    &entries[usize::from(at)].1
                                };
                                sum += i64::from(sector.floor.bits());
                            }
                            for &(s, state) in &entries {
                                table[usize::from(s)] = state;
                            }
                        }
                        sum
                    });
                }
                println!(
                    "staging sectors={sectors} writes={k} reads={reads} baseline_ns_per_tic={:.1} lazy_ns_per_tic={:.1} sparse_ns_per_tic={:.1} full_copy_ns_per_tic={:.1} indexed_ns_per_tic={:.1}",
                    per_tic(base),
                    per_tic(lazy_ns),
                    per_tic(sparse_ns),
                    per_tic(full_ns),
                    per_tic(indexed_ns)
                );
            }
        }
    }
    for line_count in [512usize, 4096] {
        let mut specials: Vec<i16> = (0..line_count).map(|i| (i % 7 * 11) as i16).collect();
        let mut no_sectors: Vec<SectorState> = Vec::new();
        for k in [0usize, 1, 2, 4] {
            for reads in [0usize, 16, 256] {
                let written: Vec<u16> =
                    (0..k).map(|j| ((j * 37 + 5) % line_count) as u16).collect();
                let read: Vec<u16> = (0..reads)
                    .map(|r| ((r * 7919 + 13) % line_count) as u16)
                    .collect();
                let mut lazy = staging::StagedWrites::default();
                let mut sparse: Vec<(u16, i16)> = Vec::new();
                let [mut base, mut lazy_ns, mut sparse_ns] = [(); 3].map(|()| Vec::new());
                for _ in 0..REPEATS {
                    time(&mut base, &mut || {
                        let mut sum = 0i64;
                        for _ in 0..TICS {
                            let table = black_box(&specials);
                            for &l in &read {
                                sum += i64::from(table[usize::from(l)]);
                            }
                        }
                        sum
                    });
                    time(&mut lazy_ns, &mut || {
                        let mut sum = 0i64;
                        for t in 0..TICS {
                            lazy.clear();
                            for &l in &written {
                                lazy.write_line_special(&specials, l, t as i16).unwrap();
                            }
                            let table = black_box(&specials);
                            for &l in &read {
                                sum += i64::from(lazy.line_special(table, l).unwrap());
                            }
                            lazy.commit(&mut no_sectors, &mut specials);
                        }
                        sum
                    });
                    time(&mut sparse_ns, &mut || {
                        let mut sum = 0i64;
                        for t in 0..TICS {
                            sparse.clear();
                            for &l in &written {
                                match sparse.iter_mut().find(|(i, _)| *i == l) {
                                    Some(entry) => entry.1 = t as i16,
                                    None => {
                                        sparse.try_reserve(1).unwrap();
                                        sparse.push((l, t as i16));
                                    }
                                }
                            }
                            let table = black_box(&specials);
                            for &l in &read {
                                let special = match sparse.iter().find(|(i, _)| *i == l) {
                                    Some((_, special)) => *special,
                                    None => table[usize::from(l)],
                                };
                                sum += i64::from(special);
                            }
                            for &(l, special) in &sparse {
                                specials[usize::from(l)] = special;
                            }
                        }
                        sum
                    });
                }
                println!(
                    "staging lines={line_count} writes={k} reads={reads} baseline_ns_per_tic={:.1} lazy_ns_per_tic={:.1} sparse_ns_per_tic={:.1}",
                    per_tic(base),
                    per_tic(lazy_ns),
                    per_tic(sparse_ns)
                );
            }
        }
    }
}

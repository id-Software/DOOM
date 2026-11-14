//! Shared built-in actor thinker pass; profile selection does not create a scheduler.
use crate::*;
pub(crate) struct Inputs<'a> {
    pub world: &'a StaticWorld,
    /// The committed sector table; every visit reads it through `writes`, so a
    /// mover's height writes are visible to every later thinker of the pass.
    pub committed: &'a [SectorState],
    /// The committed runtime line specials (empty without movers); read and
    /// written through `writes` like the sectors.
    pub committed_lines: &'a [i16],
    /// The plan's staged sector and line-special writes (empty outside movers).
    pub writes: &'a mut crate::staging::StagedWrites,
    /// The source `leveltime` of this tic (the completed-tic count).
    pub leveltime: u64,
    pub profile: Profile,
    pub bindings: [Option<ActorId>; 4],
    pub live_bindings: &'a mut [Option<ActorId>; 4],
    pub registry: &'a Option<Arc<ActorRegistry>>,
    pub options: ActorOptions,
    pub player: Option<&'a mut original_player::PlayerRuntime>,
    pub statistics: &'a mut Option<LevelStatistics>,
    pub rng: &'a mut DoomRng,
    pub spatial: &'a mut movement::SpatialIndex,
    pub events: &'a mut Vec<Event>,
    pub commands: [TicCommand; 4],
    pub perception_inputs: Option<perception::PerceptionInputs<'a>>,
    /// The one-sided boundary of the diagonal policy (the escape witness).
    pub boundary: Option<&'a player_map_scale::Boundary>,
    /// The specials policy's staged runtime sector lights and specials, written by
    /// the light thinkers at their turn; `None` under every other policy.
    pub specials: Option<&'a mut [sector_thinkers::SectorSpecial]>,
    /// The movers policy's staged switch buttons, which a monster's hitscan into a
    /// gun-triggered line (GR 46, Episode 1 policy) starts; `None` otherwise.
    pub buttons: Option<&'a mut use_lines::Buttons>,
}
pub(crate) fn run(actors: &mut ThinkerStore<Actor>, input: Inputs<'_>) {
    let Inputs {
        world,
        committed,
        committed_lines,
        writes,
        leveltime,
        profile,
        bindings,
        live_bindings,
        registry,
        options,
        statistics,
        mut player,
        rng,
        spatial,
        events,
        commands,
        perception_inputs,
        boundary,
        mut specials,
        mut buttons,
    } = input;
    let recoverable_refill = player.as_ref().is_some_and(|p| p.policy.has_imps());
    // A refused sector-thinker visit outside the checked transaction; the checked
    // transaction instead records it and discards the tic.
    let mut refused = None;
    actors
        .run(actors.visit_bound(), |visit, mut context| match visit {
            Visit::SectorThink(_) | Visit::SectorRetired(_) => {
                let checked = perception_inputs.and_then(|p| p.checked);
                if checked.is_some_and(|c| c.failure.get().is_some()) {
                    return;
                }
                let movers = player.as_ref().is_some_and(|p| p.policy.has_movers());
                let mover = match visit {
                    Visit::SectorThink(id) => context
                        .sector_get(id)
                        .and_then(SectorThinker::mover_sector)
                        .map(|_| id),
                    _ => None,
                };
                let result = match (visit, mover) {
                    // T_VerticalDoor, T_PlatRaise, T_MoveFloor at their turn.
                    (_, Some(id)) if movers => {
                        let mut host = planes::PlaneHost {
                            committed,
                            committed_lines,
                            writes: &mut *writes,
                            access: context,
                            player: player.as_deref_mut(),
                            statistics: statistics.as_mut(),
                            perception: perception_inputs,
                            registry: registry.as_deref().expect("original registry"),
                            world,
                            options,
                            rng: &mut *rng,
                            spatial: &mut *spatial,
                            bindings: &mut *live_bindings,
                            events: &mut *events,
                            boundary,
                            specials: specials.as_deref_mut(),
                        };
                        movers::think(&mut host, id, leveltime)
                    }
                    // Only movers are ever removed: the marked entry is unlinked now.
                    (Visit::SectorRetired(id), _) if movers => {
                        events.push(Event::Mover(MoverEvent::Unlinked { mover: id }));
                        Ok(())
                    }
                    _ => sector_thinkers::visit(context, visit, specials.as_deref_mut(), rng),
                };
                if let Err(error) = result {
                    match checked {
                        Some(control) => control.failure.set(Some(error)),
                        None => refused = refused.or(Some(error)),
                    }
                }
            }
            Visit::Retired(id) => events.push(Event::Retired(id)),
            Visit::Think(id) => {
                let sectors = writes.view(committed, committed_lines);
                if perception_inputs
                    .and_then(|p| p.checked)
                    .is_some_and(|c| c.failure.get().is_some())
                {
                    return;
                }
                events.push(Event::Think(id));
                let mut actor = *context.get(id).expect("live thinker handle");
                if profile == Profile::OriginalDemon
                    && actor
                        .life
                        .is_some_and(|l| original_registry::is_missile_type(l.actor_type))
                {
                    // P_MobjThinker of a missile: its movement owns contact and
                    // explosion, then ordinary state countdown.
                    let mut host = lifecycle::LifeHost {
                        specials: specials.as_deref(),
                        buttons: None,
                        player: player.as_deref_mut(),
                        statistics: statistics.as_mut(),
                        perception: perception_inputs,
                        access: context,
                        registry: registry.as_deref().unwrap(),
                        world,
                        sectors,
                        options,
                        rng,
                        spatial,
                        bindings: live_bindings,
                        events,
                    };
                    // p_mobj.c P_MobjThinker: a missile removed by its own
                    // P_XYMovement (the sky hack) returns before P_ZMovement and
                    // the state countdown.
                    if !projectiles::fly(&mut host, id) {
                        host.tick(id);
                    }
                    return;
                }
                let had_xy_momentum = actor.momentum().iter().any(|m| *m != Fixed::ZERO);
                let source_player = player.as_ref().is_some_and(|p| p.mo == Some(id));
                if profile == Profile::Foundation {
                    events.extend(
                        world
                            .move_xy(&mut actor)
                            .into_iter()
                            .map(|attempt| Event::Movement { actor: id, attempt }),
                    );
                } else {
                    let command = if source_player {
                        Some(commands[usize::from(PlayerSlot::One.ordinal())])
                    } else {
                        bindings
                            .iter()
                            .position(|binding| *binding == Some(id))
                            .map(|slot| commands[slot])
                    };
                    let motion = if world.teleport && actor.momentum() != [Fixed::ZERO; 2] {
                        // Wire 24 (M4b step 4): every `P_XYMovement` runs its crossings
                        // inside the move, teleports included (decision I7).
                        let mover = if source_player {
                            teleport::Mover::Player
                        } else if boundary.is_some()
                            && actor.life.is_some_and(|l| {
                                player
                                    .as_ref()
                                    .is_some_and(|p| barrels::is_barrel(p.policy, l.actor_type))
                            })
                        {
                            teleport::Mover::Barrel
                        } else {
                            teleport::Mover::Other
                        };
                        let Some(registry) = registry.as_deref() else {
                            refuse(
                                perception_inputs,
                                &mut refused,
                                ActorError::InvalidType.into(),
                            );
                            return;
                        };
                        match movement::move_xy_contacts(
                            world,
                            id,
                            &mut actor,
                            spatial,
                            teleport::Motion {
                                mover,
                                access: context,
                                player: player.as_deref_mut(),
                                statistics: statistics.as_mut(),
                                perception: perception_inputs,
                                registry,
                                world,
                                sectors,
                                options,
                                rng: &mut *rng,
                                bindings: &mut *live_bindings,
                                specials: specials.as_deref_mut(),
                                boundary,
                            },
                            command,
                            sectors,
                        ) {
                            Ok(events) => events,
                            Err(error) => {
                                refuse(perception_inputs, &mut refused, error);
                                return;
                            }
                        }
                    } else if source_player
                        && player
                            .as_ref()
                            .is_some_and(|p| p.policy.has_world_pickups())
                    {
                        let Some(p) = player.as_deref_mut() else {
                            return;
                        };
                        match movement::move_xy_contacts(
                            world,
                            id,
                            &mut actor,
                            spatial,
                            world_pickups::Contacts {
                                access: &mut context,
                                player: p,
                                sectors,
                                boundary,
                            },
                            command,
                            sectors,
                        ) {
                            Ok(events) => events,
                            Err(error) => {
                                if let Some(control) = perception_inputs.and_then(|p| p.checked) {
                                    control.failure.set(Some(error));
                                }
                                return;
                            }
                        }
                    } else if let Some(boundary) = boundary.filter(|_| {
                        actor.life.is_some_and(|l| {
                            player
                                .as_ref()
                                .is_some_and(|p| barrels::is_barrel(p.policy, l.actor_type))
                        })
                    }) {
                        // The barrel escape witness (barrels policy only): a radius-10
                        // barrel can clear a wall corner in one accepted 30-unit step,
                        // which the player (r 16) and the monsters (r 20) cannot;
                        // see `barrels::BarrelMotion`.
                        match movement::move_xy_contacts(
                            world,
                            id,
                            &mut actor,
                            spatial,
                            barrels::BarrelMotion {
                                get: |id| context.get(id).copied(),
                                boundary,
                            },
                            command,
                            sectors,
                        ) {
                            Ok(events) => events,
                            Err(error) => {
                                refuse(perception_inputs, &mut refused, error);
                                return;
                            }
                        }
                    } else if profile.uses_sectors() {
                        movement::move_xy_multisector(
                            world,
                            id,
                            &mut actor,
                            spatial,
                            |id| context.get(id).copied(),
                            command,
                            sectors,
                        )
                    } else {
                        movement::move_xy(
                            world,
                            id,
                            &mut actor,
                            spatial,
                            |id| context.get(id).copied(),
                            command,
                        )
                    };
                    let crosser = if source_player {
                        line_triggers::Crosser::Player
                    } else {
                        line_triggers::Crosser::Monster
                    };
                    for event in motion {
                        events.push(match event {
                            movement::MotionEvent::Contact(event) => event.event(),
                            movement::MotionEvent::Attempt(attempt) => {
                                Event::ActorMovement { actor: id, attempt }
                            }
                            movement::MotionEvent::Slide(line) => Event::Slide { actor: id, line },
                            movement::MotionEvent::Sector(attempt) => {
                                Event::SectorMovement { actor: id, attempt }
                            }
                            // P_CrossSpecialLine after the accepted attempt, with the
                            // staged tables; a W1 clear is staged after the call.
                            movement::MotionEvent::Cross(crossing) => {
                                let result = line_triggers::cross(
                                    &mut movers::Starter {
                                        world,
                                        sectors: writes.view(committed, committed_lines),
                                        store: &mut *context,
                                        events: &mut *events,
                                        specials: specials.as_deref_mut(),
                                        player: None,
                                    },
                                    id,
                                    crosser,
                                    crossing,
                                )
                                .and_then(|after| {
                                    if after == crossing.special {
                                        Ok(())
                                    } else {
                                        writes.write_line_special(
                                            committed_lines,
                                            crossing.line,
                                            after,
                                        )
                                    }
                                });
                                if let Err(error) = result {
                                    refuse(perception_inputs, &mut refused, error);
                                    return;
                                }
                                continue;
                            }
                            movement::MotionEvent::Outer(event) => event,
                            movement::MotionEvent::Stage { line, after } => {
                                if let Err(error) =
                                    writes.write_line_special(committed_lines, line, after)
                                {
                                    refuse(perception_inputs, &mut refused, error);
                                    return;
                                }
                                continue;
                            }
                            movement::MotionEvent::SpecialCross => {
                                refuse(
                                    perception_inputs,
                                    &mut refused,
                                    MapScaleError::SpecialCross.into(),
                                );
                                return;
                            }
                        });
                    }
                }
                let sectors = writes.view(committed, committed_lines);
                if (profile == Profile::PlayerWeapons || source_player)
                    && had_xy_momentum
                    && actor.z <= actor.floor
                    && actor.momentum() == [Fixed::ZERO; 2]
                    && (if source_player {
                        commands[usize::from(PlayerSlot::One.ordinal())].forward == 0
                            && commands[usize::from(PlayerSlot::One.ordinal())].side == 0
                    } else {
                        bindings
                            .iter()
                            .position(|b| *b == Some(id))
                            .is_some_and(|slot| {
                                commands[slot].forward == 0 && commands[slot].side == 0
                            })
                    })
                {
                    let run = identity::OriginalState::S_PLAY_RUN1.ordinal();
                    if actor
                        .life
                        .unwrap()
                        .state
                        .is_some_and(|s| s.0.wrapping_sub(run) < 4)
                    {
                        let play = StateId(identity::OriginalState::S_PLAY.ordinal());
                        *context.get_mut(id).unwrap() = actor;
                        lifecycle::LifeHost {
                            specials: None,
                            buttons: None,
                            player: player.as_deref_mut(),
                            statistics: statistics.as_mut(),
                            perception: perception_inputs,
                            access: &mut *context,
                            registry: registry.as_deref().unwrap(),
                            world,
                            sectors,
                            options,
                            rng,
                            spatial,
                            bindings: live_bindings,
                            events,
                        }
                        .enter(id, play);
                        actor = *context.get(id).unwrap();
                    }
                }
                if profile.uses_sectors() {
                    if let Some(update) = multisector::move_z(&mut actor) {
                        // p_mobj.c P_ZMovement: on floor contact a player whose
                        // momz was below -GRAVITY*8 requests sfx_oof before momz is
                        // zeroed. Reachable only off the flat floor (heights policy).
                        if update.floor_contact
                            && update.before[1]
                                < Fixed::from_map_units(player_map_scale::LANDING_SOUND_MOMZ)
                            && player
                                .as_ref()
                                .is_some_and(|p| p.policy.has_heights() && p.mo == Some(id))
                        {
                            events.push(Event::ActorState(lifecycle::ActorStateEvent::Sound {
                                actor: Some(id),
                                sound: player_map_scale::SFX_OOF,
                            }));
                        }
                        events.push(Event::Vertical { actor: id, update });
                    }
                }
                *context.get_mut(id).expect("same live thinker handle") = actor;
                if let Some(registry) = registry {
                    lifecycle::LifeHost {
                        specials: specials.as_deref(),
                        buttons: buttons.as_deref_mut(),
                        player: player.as_deref_mut(),
                        statistics: statistics.as_mut(),
                        perception: perception_inputs,
                        access: context,
                        registry,
                        world,
                        sectors,
                        options,
                        rng,
                        spatial,
                        bindings: live_bindings,
                        events,
                    }
                    .tick(id);
                }
            }
        })
        .or_else(|error| match perception_inputs.and_then(|p| p.checked) {
            // A fireball can spawn into a slot retired earlier in this pass. The
            // checked transaction discards that tic; older policies are unchanged.
            Some(control) if recoverable_refill && error == StoreError::VisitLimit => {
                control
                    .failure
                    .set(control.failure.get().or(Some(error.into())));
                Ok(())
            }
            _ => Err(error),
        })
        .expect("built-in traversal preserves links and is bounded by max slots");
    if let Some(error) = refused {
        panic!(
            "no admitted policy runs movers or sector thinkers outside the checked plan: {error}"
        );
    }
}
/// Records a refused visit: in the checked transaction's failure, which discards the
/// tic, or (no admitted policy) in `refused`.
fn refuse(
    perception: Option<perception::PerceptionInputs<'_>>,
    refused: &mut Option<SimError>,
    error: SimError,
) {
    match perception.and_then(|p| p.checked) {
        Some(control) => control.failure.set(Some(error)),
        None => *refused = refused.or(Some(error)),
    }
}

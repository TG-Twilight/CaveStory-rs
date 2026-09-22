use super::*;
use crate::framework::context::Context;
use crate::game::npc::list::NPCList;
use crate::game::weapon::Weapon;

fn engine() -> (Context, SharedGameState, Player, Inventory) {
    let mut ctx = Context::new();
    ctx.headless = true;
    ctx.filesystem
        .mount_vfs(Box::new(crate::data::builtin_fs::BuiltinFS::new()));
    let mut state = SharedGameState::new(&mut ctx).unwrap();
    let mut player = Player::new(&mut state, &mut ctx);
    player.cond.set_alive(true);
    player.life = 3;
    player.max_life = 12;
    let mut inventory = Inventory::new();
    inventory.add_weapon_data(WeaponType::PolarStar, 0, 0, 0, WeaponLevel::Level1);
    inventory.add_weapon_data(WeaponType::MachineGun, 10, 100, 0, WeaponLevel::Level1);
    (ctx, state, player, inventory)
}

#[test]
fn scripted_transition_keeps_rules_but_rejects_writes_and_death_clears() {
    let (_ctx, state, mut p1, mut i1) = engine();
    let mut p2 = p1.clone();
    let mut i2 = i1.clone();
    let mut scene = ScenePort { ports: [
        Port { id: 1, player: &mut p1, inventory: &mut i1, state: &state,
            epoch: 1, stage_id: 13, status: GameStatus::Playing },
        Port { id: 2, player: &mut p2, inventory: &mut i2, state: &state,
            epoch: 2, stage_id: 13, status: GameStatus::Absent },
    ] };
    scene.ports[0].set_effects(Effects { jump_percent: 175, movement_percent: 150,
        weapons: vec![WeaponEffect { weapon: 4, lock_ammo: true,
            lock_experience: true, damage_percent: 200 }], ..Effects::default() });
    scene.ports[0].status = GameStatus::Transition;
    assert!(!scene.view().can_heal());
    scene.maintain_effects();
    assert_eq!(scene.view().details.effects.jump_percent, 175);
    assert!(scene.ports[0].inventory.get_weapon(1).unwrap().trainer_lock_ammo);
    scene.ports[0].status = GameStatus::Dead;
    scene.maintain_effects();
    assert_eq!(scene.view().details.effects, Effects::default());
    assert!(!scene.ports[0].inventory.get_weapon(1).unwrap().trainer_lock_ammo);
}

#[test]
fn transfer_advances_both_targets_without_changing_session_instance() {
    let bridge = Bridge::new(Idle).unwrap();
    let instance = bridge.instance();
    let mut adapter = Adapter { bridge: Some(bridge), server: None, epoch: 0,
        player2_epoch: 0, player2_state: Some((true, true, true)) };
    adapter.transfer_context();
    let first = (adapter.epoch, adapter.player2_epoch);
    adapter.transfer_context();
    assert_ne!(first.0, adapter.epoch);
    assert_ne!(first.1, adapter.player2_epoch);
    assert_ne!(adapter.epoch, adapter.player2_epoch);
    assert_eq!(adapter.bridge.as_ref().unwrap().instance(), instance);
    assert!(!adapter.observe_player2((true, true, true)));
}

#[test]
fn damage_lock_prevents_lethal_damage_and_does_not_change_debug_settings() {
    let (_ctx, mut state, mut player, _) = engine();
    let (npcs, _token) = NPCList::new();
    player.trainer_effects.lock_health = true;
    player.damage(100, &mut state, &npcs);
    assert_eq!(player.life, 3);
    assert!(player.cond.alive());
    assert!(!state.settings.god_mode);
    player.trainer_effects.lock_health = false;
    player.damage(2, &mut state, &npcs);
    assert_eq!(player.life, 1);
}

#[test]
fn damage_reduction_is_applied_after_difficulty_scaling() {
    let (_ctx, mut state, mut player, _) = engine();
    let (npcs, _token) = NPCList::new();
    player.trainer_effects.incoming_damage_percent = 50;
    player.damage(2, &mut state, &npcs);
    assert_eq!(player.life, 2);
}

#[test]
fn weapon_locks_prevent_consumption_recharge_and_xp_changes() {
    let (_ctx, mut state, mut player, mut inventory) = engine();
    inventory.current_weapon = 1;
    let weapon = inventory.get_current_weapon_mut().unwrap();
    weapon.trainer_lock_ammo = true;
    weapon.trainer_lock_experience = true;
    weapon.experience = 2;
    assert!(weapon.consume_ammo(1));
    weapon.refill_ammo(99);
    weapon.add_xp(20, &mut player, &mut state);
    assert_eq!((weapon.ammo, weapon.experience), (10, 2));
    inventory.take_xp(10, &mut state);
    let weapon = inventory.get_current_weapon_mut().unwrap();
    assert_eq!((weapon.level as u8, weapon.experience), (1, 2));
    weapon.trainer_lock_ammo = false;
    weapon.trainer_lock_experience = false;
    assert!(weapon.consume_ammo(1));
    assert_eq!(weapon.ammo, 9);
    inventory.take_xp(1, &mut state);
    assert_eq!(inventory.get_current_weapon().unwrap().experience, 1);
    let mut empty = Weapon::new(WeaponType::MachineGun, WeaponLevel::Level1, 0, 0, 100);
    empty.trainer_lock_ammo = true;
    assert!(!empty.consume_ammo(1));
}

#[test]
fn paused_port_edits_real_player_and_selected_weapon_only() {
    let (_ctx, state, mut player, mut inventory) = engine();
    let mut port = Port {
        id: 1,
        player: &mut player,
        inventory: &mut inventory,
        state: &state,
        epoch: 7,
        stage_id: 13,
        status: GameStatus::Paused,
    };
    port.edit(&Edit::MaxHealth { maximum: 24 }).unwrap();
    port.heal().unwrap();
    assert_eq!(
        port.view().health,
        Some(Health {
            current: 24,
            maximum: 24
        })
    );
    port.edit(&Edit::WeaponExperience {
        weapon: 2,
        delta: 15,
    })
    .unwrap();
    let w = port.inventory.get_weapon(0).unwrap();
    assert_eq!((w.level as u8, w.experience), (2, 5));
    port.edit(&Edit::WeaponExperience {
        weapon: 2,
        delta: -8,
    })
    .unwrap();
    let w = port.inventory.get_weapon(0).unwrap();
    assert_eq!((w.level as u8, w.experience), (1, 7));
    port.edit(&Edit::WeaponAmmo {
        weapon: 4,
        current: 40,
        maximum: 200,
    })
    .unwrap();
    assert_eq!(port.inventory.get_weapon(1).unwrap().ammo, 40);
    assert_eq!(port.inventory.current_weapon, 0);
    assert_eq!(port.inventory.get_weapon(0).unwrap().max_ammo, 0);
}

#[test]
fn disconnect_expiry_and_new_context_clear_real_rules() {
    use std::time::Instant;
    use trainer_bridge::LEASE;
    for reason in 0..4 {
        let (_ctx, state, mut player, mut inventory) = engine();
        let port = Port {
            id: 1,
            player: &mut player,
            inventory: &mut inventory,
            state: &state,
            epoch: 7,
            stage_id: 13,
            status: GameStatus::Playing,
        };
        let mut bridge = Bridge::new(port).unwrap();
        let now = Instant::now();
        let hello = Request {
            version: VERSION,
            request_id: 1,
            instance: bridge.instance(),
            session: None,
            command: Command::Hello {},
        };
        bridge.handle(hello, now);
        bridge.game_mut().set_effects(Effects {
            lock_health: true,
            weapons: vec![WeaponEffect {
                weapon: 4,
                lock_experience: true,
                lock_ammo: true,
                damage_percent: 200,
            }],
            ..Effects::default()
        });
        match reason {
            0 => bridge.disconnect(),
            1 => bridge.expire(now + LEASE),
            2 => {
                bridge.game_mut().epoch = 8;
                bridge.maintain_context();
            }
            _ => {
                bridge.game_mut().status = GameStatus::Dead;
                bridge.maintain_context();
            }
        }
        assert_eq!(bridge.game().view().details.effects, Effects::default());
        assert!(
            !bridge
                .game()
                .inventory
                .get_weapon(1)
                .unwrap()
                .trainer_lock_ammo
        );
        assert!(
            !bridge
                .game()
                .inventory
                .get_weapon(1)
                .unwrap()
                .trainer_lock_experience
        );
    }
}

#[test]
fn new_scene_reset_clears_cloned_rules_even_when_interface_is_disabled() {
    let (_ctx, _state, mut player, mut inventory) = engine();
    player.trainer_effects.lock_health = true;
    player.trainer_effects.movement_percent = 200;
    player.trainer_jump_active = true;
    let weapon = inventory.get_weapon_by_type_mut(WeaponType::MachineGun).unwrap();
    weapon.trainer_lock_ammo = true;
    weapon.trainer_lock_experience = true;
    let life = player.life;
    clear_player_effects(&mut player, &mut inventory);
    assert_eq!(player.trainer_effects, Effects::default());
    assert!(!player.trainer_jump_active);
    assert_eq!(player.life, life);
    let weapon = inventory.get_weapon(1).unwrap();
    assert!(!weapon.trainer_lock_ammo && !weapon.trainer_lock_experience);
}

#[test]
fn physics_and_projectile_damage_do_not_modify_base_constants() {
    let (_ctx, state, _, _) = engine();
    let effects = Effects {
        movement_percent: 200,
        jump_percent: 150,
        weapons: vec![WeaponEffect {
            weapon: 2,
            lock_experience: false,
            lock_ammo: false,
            damage_percent: 300,
        }],
        ..Effects::default()
    };
    let original = state.constants.player.air_physics;
    let boosted = physics(original, &effects);
    assert_eq!(boosted.max_dash, original.max_dash * 2);
    assert_eq!(boosted.jump, original.jump * 3 / 2);
    assert_eq!(boosted.gravity_air, original.gravity_air);
    assert_eq!(state.constants.player.air_physics.jump, original.jump);
    assert_eq!(
        bullet_damage(4, 2, TargetPlayer::Player1, &effects, &effects),
        12
    );
    assert_eq!(
        bullet_damage(4, 2, TargetPlayer::Player2, &effects, &effects),
        12
    );
    assert_eq!(
        bullet_damage(4, 13, TargetPlayer::Player1, &effects, &effects),
        4
    );
}

#[test]
fn movement_and_jump_hooks_change_actual_player_ticks() {
    use crate::entity::GameEntity;
    use crate::input::{
        player_controller::PlayerController, replay_player_controller::ReplayController,
    };
    let (_ctx, mut state, mut normal, _) = engine();
    state.control_flags.set_control_enabled(true);
    let (npcs, _token) = NPCList::new();
    let mut controller = ReplayController::new();
    controller.state.set_right(true);
    controller.update_trigger();
    normal.controller = Box::new(controller);
    let mut fast = normal.clone();
    fast.trainer_effects.movement_percent = 200;
    for _ in 0..30 {
        normal.flags.set_hit_bottom_wall(true);
        fast.flags.set_hit_bottom_wall(true);
        normal.tick(&mut state, &npcs).unwrap();
        fast.tick(&mut state, &npcs).unwrap();
    }
    assert!(fast.x > normal.x * 18 / 10);
    let mut controller = ReplayController::new();
    controller.state.set_jump(true);
    controller.update_trigger();
    normal.controller = Box::new(controller);
    normal.vel_y = 0;
    normal.flags.set_hit_bottom_wall(true);
    let mut high = normal.clone();
    high.trainer_effects.jump_percent = 150;
    normal.tick(&mut state, &npcs).unwrap();
    high.tick(&mut state, &npcs).unwrap();
    assert!(normal.vel_y < 0);
    assert!(high.vel_y < normal.vel_y * 14 / 10);
    assert_eq!(state.constants.player.air_physics.jump, 0x500);
}

#[test]
fn item_catalog_add_change_remove_and_capacity_use_real_inventory() {
    use crate::game::scripting::tsc::text_script::TextScript;
    let (_ctx, mut state, mut player, mut inventory) = engine();
    let source = (1..=33)
        .map(|id| format!("#{:04}\n<END\n", 6000 + id))
        .collect::<String>();
    let script = TextScript::compile(
        source.as_bytes(),
        false,
        state.constants.textscript.encoding,
    )
    .unwrap();
    state.textscript_vm.set_inventory_script(script);
    let mut port = Port {
        id: 1,
        player: &mut player,
        inventory: &mut inventory,
        state: &state,
        epoch: 7,
        stage_id: 13,
        status: GameStatus::Paused,
    };
    assert_eq!(
        port.edit(&Edit::Item {
            item: 999,
            amount: 1
        }),
        Err(ErrorCode::Unsupported)
    );
    port.edit(&Edit::Item { item: 1, amount: 5 }).unwrap();
    port.edit(&Edit::Item { item: 1, amount: 2 }).unwrap();
    assert_eq!(port.inventory.get_item(1).unwrap().1, 2);
    port.edit(&Edit::Item { item: 1, amount: 0 }).unwrap();
    assert!(!port.inventory.has_item(1));
    for id in 1..=32 {
        port.edit(&Edit::Item {
            item: id,
            amount: 1,
        })
        .unwrap();
    }
    assert_eq!(
        port.edit(&Edit::Item {
            item: 33,
            amount: 1
        }),
        Err(ErrorCode::Capacity)
    );
    port.inventory.current_item = 31;
    port.edit(&Edit::Item {
        item: 32,
        amount: 0,
    })
    .unwrap();
    assert_eq!(port.inventory.current_item, 30);
}

#[test]
fn projectile_keeps_weapon_identity_even_for_spur_polar_star_shared_bullet() {
    use crate::game::weapon::bullet::BulletManager;
    use crate::input::{
        player_controller::PlayerController, replay_player_controller::ReplayController,
    };
    let (_ctx, mut state, mut player, _) = engine();
    let mut controller = ReplayController::new();
    controller.state.set_shoot(true);
    controller.update_trigger();
    player.controller = Box::new(controller);
    let mut weapon = Weapon::new(WeaponType::Spur, WeaponLevel::Level1, 0, 0, 0);
    let mut bullets = BulletManager::new();
    weapon.tick(&mut state, &mut player, TargetPlayer::Player1, &mut bullets);
    assert_eq!(bullets.bullets.len(), 1);
    assert_eq!(bullets.bullets[0].btype, 6);
    assert_eq!(bullets.bullets[0].trainer_weapon, 13);
}

#[test]
fn jump_multiplier_does_not_increase_upward_wind_speed() {
    use crate::entity::GameEntity;
    let (_ctx, mut state, mut normal, _) = engine();
    state.control_flags.set_control_enabled(true);
    let (npcs, _token) = NPCList::new();
    normal.flags.set_force_up(true);
    normal.vel_y = -10000;
    let mut boosted = normal.clone();
    boosted.trainer_effects.jump_percent = 200;
    normal.tick(&mut state, &npcs).unwrap();
    boosted.tick(&mut state, &npcs).unwrap();
    assert_eq!(boosted.vel_y, normal.vel_y);
}

#[test]
fn bulk_refill_and_reset_respect_weapon_locks() {
    let (_ctx, _state, _player, mut inventory) = engine();
    let weapon = inventory
        .get_weapon_by_type_mut(WeaponType::MachineGun)
        .unwrap();
    weapon.trainer_lock_ammo = true;
    weapon.trainer_lock_experience = true;
    weapon.experience = 2;
    inventory.refill_all_ammo();
    inventory.reset_all_weapon_xp();
    let weapon = inventory.get_weapon(1).unwrap();
    assert_eq!((weapon.ammo, weapon.experience), (10, 2));
}

#[test]
fn weapon_replacement_or_removal_clears_stale_rules_without_reenabling() {
    for replacement in 0..3 {
        let (_ctx, state, mut player, mut inventory) = engine();
        let mut port = Port {
            id: 1,
            player: &mut player,
            inventory: &mut inventory,
            state: &state,
            epoch: 7,
            stage_id: 13,
            status: GameStatus::Playing,
        };
        port.set_effects(Effects {
            movement_percent: 150,
            weapons: vec![WeaponEffect {
                weapon: 4,
                lock_experience: true,
                lock_ammo: true,
                damage_percent: 200,
            }],
            ..Effects::default()
        });
        match replacement {
            0 => port.inventory.trade_weapon(
                Some(WeaponType::MachineGun),
                WeaponType::MachineGun,
                100,
            ),
            1 => port.inventory.trade_weapon(
                Some(WeaponType::MachineGun),
                WeaponType::MissileLauncher,
                100,
            ),
            _ => port.inventory.remove_weapon(WeaponType::MachineGun),
        }
        port.reconcile_effects();
        assert!(port.view().details.effects.weapons.is_empty());
        assert_eq!(port.view().details.effects.movement_percent, 150);
        if let Some(weapon) = port.inventory.get_weapon(1) {
            assert!(!weapon.trainer_lock_ammo);
        }
    }
}
#[test]
fn coop_routes_edits_and_cleanup_to_real_independent_players() {
    let (_, state, mut p1, mut inv1) = engine();
    let (_, _, mut p2, mut inv2) = engine();
    p2.life = 5;
    let scene = ScenePort {
        ports: [
            Port {
                id: 1,
                player: &mut p1,
                inventory: &mut inv1,
                state: &state,
                epoch: 100,
                stage_id: 13,
                status: GameStatus::Paused,
            },
            Port {
                id: 2,
                player: &mut p2,
                inventory: &mut inv2,
                state: &state,
                epoch: 200,
                stage_id: 13,
                status: GameStatus::Paused,
            },
        ],
    };
    let mut bridge = Bridge::new(scene).unwrap();
    let now = std::time::Instant::now();
    let mut seq = 0;
    let mut session = None;
    let mut send = |bridge: &mut Bridge<ScenePort<'_>>, command| {
        seq += 1;
        let response = bridge.handle(
            Request {
                version: VERSION,
                request_id: seq,
                instance: bridge.instance(),
                session,
                command,
            },
            now,
        );
        if let Outcome::Welcome { session: id, .. } = response.outcome {
            session = Some(id);
        }
        response.outcome
    };
    send(&mut bridge, Command::Hello {});
    assert!(
        matches!(send(&mut bridge, Command::Heal { player: 2, context_epoch: 200 }), Outcome::Healed { snapshot } if snapshot.view.player == 2 && snapshot.view.health.unwrap().current == 12)
    );
    assert_eq!(bridge.game().view().health.unwrap().current, 3);
    for (player, epoch, current) in [(1, 100, 7), (2, 200, 9)] {
        assert!(matches!(
            send(
                &mut bridge,
                Command::Edit {
                    player,
                    context_epoch: epoch,
                    edit: Edit::Health { current }
                }
            ),
            Outcome::Edited { .. }
        ));
    }
    assert_eq!(
        bridge
            .game()
            .target_view(1)
            .unwrap()
            .health
            .unwrap()
            .current,
        7
    );
    assert_eq!(
        bridge
            .game()
            .target_view(2)
            .unwrap()
            .health
            .unwrap()
            .current,
        9
    );
    assert!(matches!(
        send(
            &mut bridge,
            Command::Edit {
                player: 2,
                context_epoch: 200,
                edit: Edit::WeaponAmmo {
                    weapon: 4,
                    current: 33,
                    maximum: 80
                }
            }
        ),
        Outcome::Edited { .. }
    ));
    assert_eq!(
        bridge.game_mut().ports[0]
            .inventory
            .get_weapon_by_type_mut(WeaponType::MachineGun)
            .unwrap()
            .ammo,
        10
    );
    assert_eq!(
        bridge.game_mut().ports[1]
            .inventory
            .get_weapon_by_type_mut(WeaponType::MachineGun)
            .unwrap()
            .ammo,
        33
    );
    for (player, epoch) in [(1, 100), (2, 200)] {
        let effects = Effects {
            lock_health: true,
            movement_percent: 150,
            ..Effects::default()
        };
        assert!(matches!(
            send(
                &mut bridge,
                Command::Edit {
                    player,
                    context_epoch: epoch,
                    edit: Edit::Effects { effects }
                }
            ),
            Outcome::Edited { .. }
        ));
    }
    // Leaving P2 clears only that player's rules; P1's session remains usable.
    bridge.game_mut().ports[1].status = GameStatus::Absent;
    bridge.game_mut().ports[1].epoch = 201;
    bridge.maintain_context();
    assert!(bridge.game().ports[0].player.trainer_effects.lock_health);
    assert_eq!(
        bridge.game().ports[1].player.trainer_effects,
        Effects::default()
    );
    assert_eq!(
        send(
            &mut bridge,
            Command::Heal {
                player: 2,
                context_epoch: 200
            }
        ),
        Outcome::Rejected {
            code: ErrorCode::ContextChanged
        }
    );
    assert_eq!(
        send(
            &mut bridge,
            Command::Heal {
                player: 2,
                context_epoch: 201
            }
        ),
        Outcome::Rejected {
            code: ErrorCode::NotPlayable
        }
    );
    assert_eq!(
        send(&mut bridge, Command::ReadState { player: 3 }),
        Outcome::Rejected {
            code: ErrorCode::WrongPlayer
        }
    );
    let absent = bridge.game().target_view(2).unwrap();
    assert!(absent.health.is_none());
    assert!(absent.details.weapons.is_empty());
    bridge.game_mut().ports[1].status = GameStatus::Playing;
    bridge.game_mut().ports[1].epoch = 202;
    assert_eq!(
        send(
            &mut bridge,
            Command::Heal {
                player: 2,
                context_epoch: 200
            }
        ),
        Outcome::Rejected {
            code: ErrorCode::ContextChanged
        }
    );
    send(&mut bridge, Command::Disconnect {});
    for port in &bridge.game().ports {
        assert_eq!(port.player.trainer_effects, Effects::default());
    }
}

#[test]
fn coop_same_weapon_damage_uses_owner_and_does_not_mutate_other_rules() {
    let p1 = Effects {
        weapons: vec![WeaponEffect {
            weapon: 2,
            damage_percent: 200,
            lock_experience: false,
            lock_ammo: false,
        }],
        ..Effects::default()
    };
    let p2 = Effects {
        weapons: vec![WeaponEffect {
            weapon: 2,
            damage_percent: 300,
            lock_experience: false,
            lock_ammo: false,
        }],
        ..Effects::default()
    };
    assert_eq!(bullet_damage(4, 2, TargetPlayer::Player1, &p1, &p2), 8);
    assert_eq!(bullet_damage(4, 2, TargetPlayer::Player2, &p1, &p2), 12);
    assert_eq!(bullet_damage(4, 13, TargetPlayer::Player2, &p1, &p2), 4);
}
#[test]
fn coop_join_leave_death_and_rejoin_advance_only_second_player_generation() {
    let mut adapter = Adapter {
        bridge: None,
        server: None,
        epoch: 500,
        player2_epoch: 0,
        player2_state: None,
    };
    let mut last = 0;
    for state in [
        (false, false, true),
        (true, false, true),
        (true, true, true),
        (true, true, false),
        (true, false, false),
        (false, false, false),
        (true, true, true),
    ] {
        assert!(adapter.observe_player2(state));
        assert_ne!(adapter.player2_epoch, last);
        last = adapter.player2_epoch;
        assert!(!adapter.observe_player2(state));
        assert_eq!(adapter.player2_epoch, last);
        assert_eq!(adapter.epoch, 500);
    }
}

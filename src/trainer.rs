//! Shared game-thread adapter. Platform activation and I/O live in trainer_platform.
use crate::engine_constants::PhysicsConsts;
use crate::game::inventory::Inventory;
use crate::game::player::{Player, TargetPlayer};
use crate::game::shared_game_state::{PlayerCount, ReplayState, SharedGameState};
use crate::game::weapon::{WeaponLevel, WeaponType};
use crate::scene::game_scene::GameScene;
use std::sync::atomic::{AtomicU64, Ordering};
use trainer_bridge::{Bridge, GamePort, PortError};
use trainer_protocol::*;
use crate::trainer_platform::{self, Endpoint};

static EPOCH: AtomicU64 = AtomicU64::new(1);

/// Called before a new scene can advance, including when access was revoked
/// while the transfer cloned players from the old scene.
pub fn clear_player_effects(player: &mut Player, inventory: &mut Inventory) {
    player.trainer_effects = Effects::default();
    player.trainer_jump_active = false;
    for idx in 0..inventory.get_weapon_count() {
        let kind = inventory.get_weapon(idx).unwrap().wtype;
        let weapon = inventory.get_weapon_by_type_mut(kind).unwrap();
        weapon.trainer_lock_experience = false;
        weapon.trainer_lock_ammo = false;
    }
}
pub struct Idle;
impl GamePort for Idle {
    fn view(&self) -> GameView {
        GameView {
            player: 1,
            details: Details::default(),
            context_epoch: 0,
            stage_id: 0,
            status: GameStatus::Title,
            health: None,
        }
    }
    fn heal(&mut self) -> Result<(), PortError> {
        Err(PortError)
    }
}
pub struct Adapter {
    bridge: Option<Bridge<Idle>>,
    server: Option<Endpoint>,
    epoch: u64,
    player2_epoch: u64,
    player2_state: Option<(bool, bool, bool)>,
}
impl Adapter {
    /// Move this adapter (including its endpoint) only along the engine's TRA path.
    /// Advance both edit generations without renewing the session's lease.
    pub fn transfer_context(&mut self) {
        self.epoch = EPOCH.fetch_add(1, Ordering::Relaxed);
        self.player2_epoch = EPOCH.fetch_add(1, Ordering::Relaxed);
        if let Some(bridge) = self.bridge.as_mut() {
            bridge.continue_context(self.epoch);
        }
    }
    pub fn new() -> Option<Self> {
        if !trainer_platform::enabled() {
            return None;
        }
        match Bridge::new(Idle) {
            Ok(bridge) => Some(Self {
                bridge: Some(bridge),
                server: None,
                epoch: EPOCH.fetch_add(1, Ordering::Relaxed),
                player2_epoch: EPOCH.fetch_add(1, Ordering::Relaxed),
                player2_state: None,
            }),
            Err(error) => {
                log::warn!("Trainer unavailable: {error}");
                None
            }
        }
    }
    pub fn tick(&mut self, scene: &mut GameScene, state: &SharedGameState, suspended: bool) -> bool {
        // Decide retention from the same observation that triggers real-player cleanup.
        // A later Binder revoke is handled on the next tick, never by dropping Idle.
        let active = trainer_platform::enabled();
        let p2_state = (
            state.player_count == PlayerCount::Two,
            scene.player2.cond.alive(),
            scene.player2.life > 0,
        );
        let changed = self.observe_player2(p2_state);
        let statuses = [
            status(scene, state, suspended, 1),
            status(scene, state, suspended, 2),
        ];
        let mut port = ScenePort {
            ports: [
                Port {
                    id: 1,
                    player: &mut scene.player1,
                    inventory: &mut scene.inventory_player1,
                    state,
                    epoch: self.epoch,
                    stage_id: scene.stage_id as u16,
                    status: statuses[0],
                },
                Port {
                    id: 2,
                    player: &mut scene.player2,
                    inventory: &mut scene.inventory_player2,
                    state,
                    epoch: self.player2_epoch,
                    stage_id: scene.stage_id as u16,
                    status: statuses[1],
                },
            ],
        };
        if !active {
            port.clear_effects();
            if let Some(bridge) = self.bridge.as_mut() { bridge.disconnect(); }
            self.server = None;
            return false;
        }
        if changed {
            port.ports[1].clear_effects();
        }
        if self.server.is_none() {
            // Transfers clone both players. Neither may inherit another scene's rules.
            port.clear_effects();
            match Endpoint::open() {
                Ok(server) => self.server = Some(server),
                Err(_) => return true,
            }
        }
        port.maintain_effects();
        let mut bridge = self.bridge.take().unwrap().map_game(port);
        bridge.maintain_context();
        if let Err(error) = self.server.as_mut().unwrap().poll(&mut bridge) {
            log::debug!("Trainer connection closed: {error}");
        }
        self.bridge = Some(bridge.map_game(Idle));
        true
    }
    fn observe_player2(&mut self, state: (bool, bool, bool)) -> bool {
        if self.player2_state == Some(state) {
            return false;
        }
        self.player2_state = Some(state);
        self.player2_epoch = EPOCH.fetch_add(1, Ordering::Relaxed);
        true
    }
}
fn status(scene: &mut GameScene, state: &SharedGameState, suspended: bool, id: u8) -> GameStatus {
    let player = if id == 1 {
        &scene.player1
    } else {
        &scene.player2
    };
    if scene.intro_mode {
        GameStatus::Title
    } else if id == 2 && state.player_count != PlayerCount::Two {
        GameStatus::Absent
    } else if id == 2 && state.player_count_modified_in_game {
        GameStatus::Transition
    } else if !player.cond.alive() || player.life == 0 {
        GameStatus::Dead
    } else if state.next_scene.is_some()
        || !state.control_flags.control_enabled()
        || state.replay_state != ReplayState::None
    {
        GameStatus::Transition
    } else if suspended || scene.pause_menu.is_paused() {
        GameStatus::Paused
    } else {
        GameStatus::Playing
    }
}
/// One transport session, two independently borrowed single-player editors.
struct ScenePort<'a> {
    ports: [Port<'a>; 2],
}
impl ScenePort<'_> {
    fn index(player: u8) -> Option<usize> {
        match player {
            1 => Some(0),
            2 => Some(1),
            _ => None,
        }
    }
}
impl GamePort for ScenePort<'_> {
    fn view(&self) -> GameView {
        self.ports[0].view()
    }
    fn heal(&mut self) -> Result<(), PortError> {
        self.ports[0].heal()
    }
    fn target_view(&self, player: u8) -> Option<GameView> {
        Self::index(player).map(|i| self.ports[i].view())
    }
    fn target_heal(&mut self, player: u8) -> Result<(), PortError> {
        self.ports[Self::index(player).ok_or(PortError)?].heal()
    }
    fn target_edit(&mut self, player: u8, edit: &Edit) -> Result<(), ErrorCode> {
        self.ports[Self::index(player).ok_or(ErrorCode::WrongPlayer)?].edit(edit)
    }
    fn clear_effects(&mut self) {
        for port in &mut self.ports {
            port.clear_effects();
        }
    }
    fn maintain_effects(&mut self) {
        for port in &mut self.ports {
            // Script control locks (door fades/dialogue) reject edits but do not
            // cancel an ongoing session. Death/absence/replay still clear rules.
            let scripted_transition = port.status == GameStatus::Transition
                && port.state.replay_state == ReplayState::None;
            if !port.view().can_heal() && !scripted_transition {
                port.clear_effects();
            } else {
                port.reconcile_effects();
            }
        }
    }
    fn capabilities(&self) -> Vec<Capability> {
        self.ports[0].capabilities()
    }
}

struct Port<'a> {
    id: u8,
    player: &'a mut Player,
    inventory: &'a mut Inventory,
    state: &'a SharedGameState,
    epoch: u64,
    stage_id: u16,
    status: GameStatus,
}
impl GamePort for Port<'_> {
    fn view(&self) -> GameView {
        if matches!(self.status, GameStatus::Title | GameStatus::Absent) {
            return GameView {
                player: self.id,
                context_epoch: self.epoch,
                stage_id: self.stage_id,
                status: self.status,
                health: None,
                details: Details::default(),
            };
        }
        let weapons = (0..self.inventory.get_weapon_count())
            .filter_map(|idx| self.inventory.get_weapon(idx))
            .map(|w| WeaponView {
                id: w.wtype as u16,
                level: w.level as u8,
                experience: w.experience,
                thresholds: self.state.constants.weapon.level_table[w.wtype as usize],
                ammo: w.ammo,
                max_ammo: w.max_ammo,
            })
            .collect();
        let items = (0..)
            .map_while(|i| self.inventory.get_item_idx(i))
            .map(|item| ItemView {
                id: item.0,
                amount: item.1,
            })
            .collect();
        let item_catalog = self
            .state
            .textscript_vm
            .scripts
            .borrow()
            .inventory_script
            .get_event_ids()
            .into_iter()
            .filter(|id| (6001..=6255).contains(id))
            .map(|id| id - 6000)
            .collect();
        GameView {
            player: self.id,
            context_epoch: self.epoch,
            stage_id: self.stage_id,
            status: self.status,
            health: if matches!(self.status, GameStatus::Title | GameStatus::Absent) {
                None
            } else {
                Some(Health {
                    current: self.player.life,
                    maximum: self.player.max_life,
                })
            },
            details: Details {
                weapons,
                items,
                item_catalog,
                effects: self.player.trainer_effects.clone(),
            },
        }
    }
    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::ReadHealth,
            Capability::HealOnce,
            Capability::EditState,
        ]
    }
    fn heal(&mut self) -> Result<(), PortError> {
        if !self.view().can_heal() {
            return Err(PortError);
        }
        self.player.life = self.player.max_life;
        Ok(())
    }
    fn edit(&mut self, edit: &Edit) -> Result<(), ErrorCode> {
        let before = self.view();
        if !before.can_heal() {
            return Err(ErrorCode::NotPlayable);
        }
        edit.validate(&before)?;
        match edit {
            Edit::Health { current } => self.player.life = *current,
            Edit::MaxHealth { maximum } => {
                self.player.max_life = *maximum;
                self.player.life = self.player.life.min(*maximum);
            }
            Edit::WeaponExperience { weapon, delta } => {
                let w = before
                    .details
                    .weapons
                    .iter()
                    .find(|w| w.id == *weapon)
                    .ok_or(ErrorCode::Unsupported)?;
                let (level, experience) = w.shifted_xp(*delta)?;
                let w = self
                    .inventory
                    .get_weapon_by_type_mut(weapon_type(*weapon)?)
                    .ok_or(ErrorCode::Unsupported)?;
                w.level = match level {
                    1 => WeaponLevel::Level1,
                    2 => WeaponLevel::Level2,
                    _ => WeaponLevel::Level3,
                };
                w.experience = experience;
            }
            Edit::WeaponAmmo {
                weapon,
                current,
                maximum,
            } => {
                let w = self
                    .inventory
                    .get_weapon_by_type_mut(weapon_type(*weapon)?)
                    .ok_or(ErrorCode::Unsupported)?;
                w.ammo = *current;
                w.max_ammo = *maximum;
            }
            Edit::Item { item, amount } => {
                if *amount == 0 {
                    self.inventory.remove_item(*item);
                } else if let Some(entry) = self.inventory.get_item(*item) {
                    entry.1 = *amount;
                } else {
                    self.inventory.add_item_amount(*item, *amount);
                }
                let count = (0..).map_while(|i| self.inventory.get_item_idx(i)).count();
                self.inventory.current_item = self
                    .inventory
                    .current_item
                    .min(count.saturating_sub(1) as u16);
            }
            Edit::Effects { effects } => self.set_effects(effects.clone()),
            Edit::ClearEffects {} => self.clear_effects(),
        }
        Ok(())
    }
    fn clear_effects(&mut self) {
        self.set_effects(Effects::default());
    }
}
impl Port<'_> {
    fn reconcile_effects(&mut self) {
        let mut effects = self.player.trainer_effects.clone();
        effects.weapons.retain(|rule| {
            let Some(weapon) = weapon_type(rule.weapon)
                .ok()
                .and_then(|kind| self.inventory.get_weapon_by_type_mut(kind))
            else {
                return false;
            };
            weapon.trainer_lock_experience == rule.lock_experience
                && weapon.trainer_lock_ammo == rule.lock_ammo
        });
        if effects != self.player.trainer_effects {
            self.set_effects(effects);
        }
    }
    fn set_effects(&mut self, effects: Effects) {
        if effects.jump_percent == 100 {
            self.player.trainer_jump_active = false;
        }
        for idx in 0..self.inventory.get_weapon_count() {
            let kind = self.inventory.get_weapon(idx).unwrap().wtype;
            let weapon = self.inventory.get_weapon_by_type_mut(kind).unwrap();
            let rule = effects.weapons.iter().find(|e| e.weapon == kind as u16);
            weapon.trainer_lock_experience = rule.is_some_and(|e| e.lock_experience);
            weapon.trainer_lock_ammo = rule.is_some_and(|e| e.lock_ammo);
        }
        self.player.trainer_effects = effects;
    }
}
fn weapon_type(id: u16) -> Result<WeaponType, ErrorCode> {
    match id {
        1 => Ok(WeaponType::Snake),
        2 => Ok(WeaponType::PolarStar),
        3 => Ok(WeaponType::Fireball),
        4 => Ok(WeaponType::MachineGun),
        5 => Ok(WeaponType::MissileLauncher),
        7 => Ok(WeaponType::Bubbler),
        9 => Ok(WeaponType::Blade),
        10 => Ok(WeaponType::SuperMissileLauncher),
        12 => Ok(WeaponType::Nemesis),
        13 => Ok(WeaponType::Spur),
        _ => Err(ErrorCode::Unsupported),
    }
}
pub fn incoming_damage(damage: i32, effects: &Effects) -> i32 {
    if damage <= 0 || effects.lock_health {
        return 0;
    }
    ((i64::from(damage) * i64::from(effects.incoming_damage_percent)) / 100)
        .min(i64::from(i32::MAX)) as i32
}
pub fn bullet_damage(
    damage: i16,
    weapon: u16,
    owner: TargetPlayer,
    player1: &Effects,
    player2: &Effects,
) -> i16 {
    if damage <= 0 {
        return damage;
    }
    let effects = match owner {
        TargetPlayer::Player1 => player1,
        TargetPlayer::Player2 => player2,
    };
    let percent = effects
        .weapons
        .iter()
        .find(|e| e.weapon == weapon)
        .map_or(100, |e| e.damage_percent);
    (i32::from(damage) * i32::from(percent) / 100).min(i32::from(i16::MAX)) as i16
}
pub fn physics(mut physics: PhysicsConsts, effects: &Effects) -> PhysicsConsts {
    physics.max_dash = scaled(physics.max_dash, effects.movement_percent);
    physics.dash_ground = scaled(physics.dash_ground, effects.movement_percent);
    physics.dash_air = scaled(physics.dash_air, effects.movement_percent);
    physics.resist = scaled(physics.resist, effects.movement_percent);
    physics.jump = scaled(physics.jump, effects.jump_percent);
    physics
}
pub fn scaled(value: i32, percent: u16) -> i32 {
    (i64::from(value) * i64::from(percent) / 100).clamp(i64::from(i32::MIN), i64::from(i32::MAX))
        as i32
}

#[cfg(test)]
#[path = "trainer_tests.rs"]
mod tests;

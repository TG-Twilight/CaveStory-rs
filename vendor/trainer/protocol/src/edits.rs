use crate::{ErrorCode, GameView};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Details {
    pub weapons: Vec<WeaponView>,
    pub items: Vec<ItemView>,
    pub item_catalog: Vec<u16>,
    pub effects: Effects,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponView {
    pub id: u16,
    pub level: u8,
    pub experience: u16,
    pub thresholds: [u16; 3],
    pub ammo: u16,
    pub max_ammo: u16,
}

impl WeaponView {
    pub fn shifted_xp(&self, delta: i32) -> Result<(u8, u16), ErrorCode> {
        if self.id == 13 || !(1..=3).contains(&self.level) || self.thresholds.contains(&0) {
            return Err(ErrorCode::Unsupported);
        }
        let total: i64 = self.thresholds[..usize::from(self.level - 1)]
            .iter()
            .map(|&x| i64::from(x))
            .sum::<i64>()
            + i64::from(self.experience);
        let cap: i64 = self.thresholds.iter().map(|&x| i64::from(x)).sum();
        let mut target = (total + i64::from(delta)).clamp(0, cap);
        let mut level = 1;
        for threshold in &self.thresholds[..2] {
            if target < i64::from(*threshold) {
                break;
            }
            target -= i64::from(*threshold);
            level += 1;
        }
        Ok((level, target as u16))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemView {
    pub id: u16,
    pub amount: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effects {
    pub lock_health: bool,
    pub incoming_damage_percent: u16,
    pub movement_percent: u16,
    pub jump_percent: u16,
    pub weapons: Vec<WeaponEffect>,
}
impl Default for Effects {
    fn default() -> Self {
        Self {
            lock_health: false,
            incoming_damage_percent: 100,
            movement_percent: 100,
            jump_percent: 100,
            weapons: vec![],
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponEffect {
    pub weapon: u16,
    pub lock_experience: bool,
    pub lock_ammo: bool,
    pub damage_percent: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Edit {
    Health {
        current: u16,
    },
    MaxHealth {
        maximum: u16,
    },
    WeaponExperience {
        weapon: u16,
        delta: i32,
    },
    WeaponAmmo {
        weapon: u16,
        current: u16,
        maximum: u16,
    },
    Item {
        item: u16,
        amount: u16,
    },
    Effects {
        effects: Effects,
    },
    ClearEffects {},
}

impl Edit {
    pub fn validate(&self, view: &GameView) -> Result<(), ErrorCode> {
        let valid = match self {
            Self::Health { current } => view
                .health
                .is_some_and(|h| *current > 0 && *current <= h.maximum),
            Self::MaxHealth { maximum } => (1..=999).contains(maximum),
            Self::WeaponExperience { weapon, delta } => {
                let w = view
                    .details
                    .weapons
                    .iter()
                    .find(|w| w.id == *weapon)
                    .ok_or(ErrorCode::Unsupported)?;
                if view
                    .details
                    .effects
                    .weapons
                    .iter()
                    .any(|w| w.weapon == *weapon && w.lock_experience)
                {
                    return Err(ErrorCode::Locked);
                }
                w.shifted_xp(*delta)?;
                true
            }
            Self::WeaponAmmo {
                weapon,
                current,
                maximum,
            } => {
                let w = view
                    .details
                    .weapons
                    .iter()
                    .find(|w| w.id == *weapon)
                    .ok_or(ErrorCode::Unsupported)?;
                if w.max_ammo == 0 {
                    return Err(ErrorCode::Unsupported);
                }
                if view
                    .details
                    .effects
                    .weapons
                    .iter()
                    .any(|w| w.weapon == *weapon && w.lock_ammo)
                {
                    return Err(ErrorCode::Locked);
                }
                *maximum > 0 && *maximum <= 9999 && *current <= *maximum
            }
            Self::Item { item, amount } => {
                if *item == 0 || !view.details.item_catalog.contains(item) {
                    return Err(ErrorCode::Unsupported);
                }
                if *amount > 0
                    && view.details.items.len() >= 32
                    && !view.details.items.iter().any(|i| i.id == *item)
                {
                    return Err(ErrorCode::Capacity);
                }
                *amount <= 999
            }
            Self::Effects { effects } => {
                if effects.weapons.len() > 8 {
                    return Err(ErrorCode::Capacity);
                }
                for (idx, effect) in effects.weapons.iter().enumerate() {
                    let w = view
                        .details
                        .weapons
                        .iter()
                        .find(|w| w.id == effect.weapon)
                        .ok_or(ErrorCode::Unsupported)?;
                    if (effect.lock_experience && w.id == 13)
                        || (effect.lock_ammo && w.max_ammo == 0)
                    {
                        return Err(ErrorCode::Unsupported);
                    }
                    if effect.damage_percent > 1000
                        || effects.weapons[..idx]
                            .iter()
                            .any(|e| e.weapon == effect.weapon)
                    {
                        return Err(ErrorCode::InvalidValue);
                    }
                }
                effects.incoming_damage_percent <= 100
                    && (100..=300).contains(&effects.movement_percent)
                    && (100..=200).contains(&effects.jump_percent)
            }
            Self::ClearEffects {} => true,
        };
        if valid {
            Ok(())
        } else {
            Err(ErrorCode::InvalidValue)
        }
    }

    /// Compare requested fields with the game-thread readback, independently of transport success.
    pub fn confirmed(&self, before: &GameView, after: &GameView) -> bool {
        if before.player != after.player
            || before.context_epoch != after.context_epoch
            || !after.can_heal()
        {
            return false;
        }
        match self {
            Self::Health { current } => after.health.is_some_and(|h| {
                h.current == *current && Some(h.maximum) == before.health.map(|v| v.maximum)
            }),
            Self::MaxHealth { maximum } => after.health.is_some_and(|h| {
                h.maximum == *maximum
                    && Some(h.current) == before.health.map(|v| v.current.min(*maximum))
            }),
            Self::WeaponExperience { weapon, delta } => before
                .details
                .weapons
                .iter()
                .find(|w| w.id == *weapon)
                .and_then(|w| w.shifted_xp(*delta).ok())
                .is_some_and(|xp| {
                    after
                        .details
                        .weapons
                        .iter()
                        .any(|w| w.id == *weapon && (w.level, w.experience) == xp)
                }),
            Self::WeaponAmmo {
                weapon,
                current,
                maximum,
            } => after
                .details
                .weapons
                .iter()
                .any(|w| w.id == *weapon && w.ammo == *current && w.max_ammo == *maximum),
            Self::Item { item, amount } => {
                after
                    .details
                    .items
                    .iter()
                    .find(|i| i.id == *item)
                    .map_or(0, |i| i.amount)
                    == *amount
            }
            Self::Effects { effects } => after.details.effects == *effects,
            Self::ClearEffects {} => after.details.effects == Effects::default(),
        }
    }
}

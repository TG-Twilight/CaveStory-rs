//! Gameplay feedback, independent of physics and backend motor APIs.
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct RumblePlayback {
    active: Option<(RumbleEffect, Instant)>,
}

impl RumblePlayback {
    pub fn accept(&mut self, effect: RumbleEffect, now: Instant) -> bool {
        if let Some((active, until)) = self.active {
            if until > now && (effect.priority < active.priority ||
                (effect.priority == active.priority &&
                 u32::from(effect.low) + u32::from(effect.high) < u32::from(active.low) + u32::from(active.high))) {
                return false;
            }
        }
        self.active = Some((effect, now + Duration::from_millis(u64::from(effect.duration_ms))));
        true
    }
    pub fn stop(&mut self) -> bool { self.active.take().is_some() }
    pub fn enhanced_active(&self) -> bool { self.active.is_some_and(|(e, _)| e.priority < 3) }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RumbleEffect {
    pub low: u16,
    pub high: u16,
    pub duration_ms: u32,
    pub priority: u8,
}

impl RumbleEffect {
    pub fn original(low: u16, high: u16, ticks: u32, tps: u32) -> Self {
        Self { low, high, duration_ms: ticks.saturating_mul(1000) / tps.max(1), priority: 3 }
    }
}

#[derive(Clone, Default)]
pub struct PlayerFeedback {
    enabled: bool,
    peak_y: Option<i32>,
    contact_cooldown: f64,
    explosion_cooldown: f64,
    pending: Option<RumbleEffect>,
}

impl PlayerFeedback {
    pub fn begin_tick(&mut self, enabled: bool, y: i32, grounded: bool, tps: u32) {
        self.enabled = enabled;
        let elapsed = 1000.0 / tps.max(1) as f64;
        self.contact_cooldown = (self.contact_cooldown - elapsed).max(0.0);
        self.explosion_cooldown = (self.explosion_cooldown - elapsed).max(0.0);
        self.peak_y = Some(if grounded || !enabled { y } else { self.peak_y.unwrap_or(y).min(y) });
    }

    pub fn head_bump(&mut self) {
        if self.enabled && self.contact_cooldown <= 0.0 {
            self.request(RumbleEffect { low: 6553, high: 14417, duration_ms: 50, priority: 1 });
            self.contact_cooldown = 120.0;
        }
    }

    pub fn land(&mut self, y: i32, relative_speed: i32) {
        // Ordinary jumps reach less than 64 pixels; speed alone also fires on them.
        let drop = i64::from(y) - i64::from(self.peak_y.unwrap_or(y));
        if self.enabled && drop >= 64 * 512 && relative_speed > 0x400 && self.contact_cooldown <= 0.0 {
            self.request(RumbleEffect { low: 11796, high: 5242, duration_ms: 70, priority: 1 });
            self.contact_cooldown = 120.0;
        }
        self.peak_y = Some(y);
    }

    pub fn explosion(&mut self, dx: i32, dy: i32, super_missile: bool) {
        if !self.enabled || self.explosion_cooldown > 0.0 { return; }
        let radius = if super_missile { 128.0 } else { 96.0 };
        let distance = (dx as f64).hypot(dy as f64) / 512.0;
        if distance >= radius { return; }
        let scale = 1.0 - distance / radius;
        self.request(RumbleEffect {
            low: (if super_missile { 24903.0 } else { 16383.0 } * scale) as u16,
            high: (if super_missile { 13107.0 } else { 9830.0 } * scale) as u16,
            duration_ms: if super_missile { 130 } else { 90 },
            priority: 2,
        });
    }

    pub fn request(&mut self, effect: RumbleEffect) {
        let rank = |e: RumbleEffect| (e.priority, u32::from(e.low) + u32::from(e.high), e.duration_ms);
        if self.pending.is_none_or(|old| rank(effect) > rank(old)) { self.pending = Some(effect); }
    }

    pub fn take(&mut self) -> Option<RumbleEffect> {
        let effect = self.pending.take();
        if effect.is_some_and(|e| e.priority == 2) { self.explosion_cooldown = 100.0; }
        effect
    }
    pub fn clear(&mut self) { *self = Self::default(); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weak_feedback_cannot_interrupt_damage_but_can_play_after_it_expires() {
        let mut playback = RumblePlayback::default();
        let now = Instant::now();
        let damage = RumbleEffect::original(20000, 20000, 20, 50);
        let tap = RumbleEffect { low: 6000, high: 14000, duration_ms: 50, priority: 1 };
        assert!(playback.accept(damage, now));
        assert!(!playback.accept(tap, now + Duration::from_millis(20)));
        assert!(playback.accept(tap, now + Duration::from_millis(401)));
        assert!(playback.stop());
        assert!(!playback.stop());
        assert!(playback.accept(tap, now));
        assert!(playback.accept(damage, now));
    }

    #[test]
    fn enhanced_contacts_are_short_and_rate_limited() {
        let mut feedback = PlayerFeedback::default();
        feedback.begin_tick(false, 0, true, 50);
        feedback.head_bump();
        assert_eq!(feedback.take(), None);
        feedback.begin_tick(true, 0, false, 50);
        feedback.head_bump();
        assert_eq!(feedback.take().unwrap().duration_ms, 50);
        feedback.head_bump();
        assert_eq!(feedback.take(), None);
        for _ in 0..6 { feedback.begin_tick(true, 0, false, 50); }
        feedback.head_bump();
        assert!(feedback.take().is_some());
    }

    #[test]
    fn normal_jump_and_slow_platform_contact_do_not_rumble() {
        let mut feedback = PlayerFeedback::default();
        feedback.begin_tick(true, 0, true, 60);
        feedback.begin_tick(true, -32 * 512, false, 60);
        feedback.land(0, 0x5ff);
        assert_eq!(feedback.take(), None);
        feedback.begin_tick(true, -80 * 512, false, 60);
        feedback.land(0, 0x100);
        assert_eq!(feedback.take(), None);
        feedback.begin_tick(true, -80 * 512, false, 60);
        feedback.land(0, 0x5ff);
        assert_eq!(feedback.take().unwrap().duration_ms, 70);
    }

    #[test]
    fn explosions_attenuate_and_merge_without_a_backlog() {
        let mut feedback = PlayerFeedback::default();
        feedback.begin_tick(true, 0, true, 50);
        feedback.explosion(96 * 512, 0, false);
        assert_eq!(feedback.take(), None);
        feedback.explosion(48 * 512, 0, false);
        let distant = feedback.take().unwrap();
        feedback.explosion(0, 0, false);
        assert_eq!(feedback.take(), None);
        for _ in 0..5 { feedback.begin_tick(true, 0, true, 50); }
        feedback.explosion(48 * 512, 0, false);
        feedback.explosion(0, 0, true);
        let close = feedback.take().unwrap();
        assert!(close.low > distant.low);
        assert_eq!(close.duration_ms, 130);
        assert_eq!(feedback.take(), None);
    }

    #[test]
    fn damage_wins_same_tick_and_clear_discards_pending_feedback() {
        let mut feedback = PlayerFeedback::default();
        feedback.begin_tick(true, 0, true, 50);
        feedback.head_bump();
        let damage = RumbleEffect::original(0x5000, 0x5000, 20, 50);
        feedback.request(damage);
        feedback.explosion(0, 0, true);
        assert_eq!(feedback.take(), Some(damage));
        feedback.request(damage);
        feedback.clear();
        assert_eq!(feedback.take(), None);
        assert_eq!(RumbleEffect::original(1, 1, 20, 60).duration_ms, 333);
    }
}

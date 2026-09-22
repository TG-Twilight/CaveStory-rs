//! Android overlay policy. Hiding the overlay never disables touch input.
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct TouchVisibility {
    devices: Option<(bool, bool)>,
    last_activity: Option<Instant>,
    gamepad_connection: u32,
}

impl TouchVisibility {
    pub fn sync_devices(&mut self, gamepad: bool, other: bool, display: &mut bool, now: Instant) {
        if !gamepad && !other {
            if self.devices != Some((false, false)) || !*display {
                self.activity(now);
            }
            *display = true;
        } else if gamepad && !self.devices.map_or(false, |devices| devices.0) {
            *display = false;
        }
        self.devices = Some((gamepad, other));
    }

    pub fn has_external_devices(&self) -> bool {
        self.devices.map_or(false, |(gamepad, other)| gamepad || other)
    }

    pub fn gamepad_connection(&mut self, generation: u32) {
        if self.gamepad_connection != generation {
            self.devices = None;
            self.gamepad_connection = generation;
        }
    }

    pub fn sdl_gamepad_added(&mut self) {
        self.devices = None;
    }

    pub fn activity(&mut self, now: Instant) {
        self.last_activity = Some(now);
    }

    pub fn resume(&mut self, now: Instant) {
        self.activity(now);
    }

    pub fn update_idle(&mut self, auto_hide: bool, held: bool, now: Instant) {
        if !auto_hide || held || self.last_activity.is_none() {
            self.activity(now);
        }
    }

    pub fn visible(&self, auto_hide: bool, now: Instant) -> bool {
        !auto_hide || self.last_activity.map_or(true, |last| now.saturating_duration_since(last) < Duration::from_secs(10))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controller_connection_hides_and_last_device_removal_restores() {
        let now = Instant::now();
        let mut policy = TouchVisibility::default();
        let mut display = true;
        policy.sync_devices(true, false, &mut display, now);
        assert!(!display);
        policy.sync_devices(true, true, &mut display, now);
        policy.sync_devices(false, true, &mut display, now);
        assert!(!display, "keyboard/mouse is still connected");
        policy.sync_devices(false, false, &mut display, now);
        assert!(display);
    }

    #[test]
    fn bare_phone_recovers_saved_or_manual_hidden_setting() {
        let mut policy = TouchVisibility::default();
        let mut display = false;
        policy.sync_devices(false, false, &mut display, Instant::now());
        assert!(display);
        display = false;
        policy.sync_devices(false, false, &mut display, Instant::now());
        assert!(display);
    }

    #[test]
    fn connected_user_can_reenable_overlay_until_next_connection() {
        let now = Instant::now();
        let mut policy = TouchVisibility::default();
        let mut display = true;
        policy.sync_devices(true, false, &mut display, now);
        display = true;
        policy.sync_devices(true, false, &mut display, now);
        assert!(display);
        policy.sync_devices(false, false, &mut display, now);
        policy.sync_devices(true, false, &mut display, now);
        assert!(!display);
    }

    #[test]
    fn idle_boundary_and_touch_wake_use_real_time() {
        let now = Instant::now();
        let mut policy = TouchVisibility::default();
        policy.update_idle(true, false, now);
        assert!(policy.visible(true, now + Duration::from_millis(9999)));
        assert!(!policy.visible(true, now + Duration::from_secs(10)));
        policy.activity(now + Duration::from_secs(11));
        assert!(policy.visible(true, now + Duration::from_secs(11)));
        assert!(!policy.visible(true, now + Duration::from_secs(21)));
    }

    #[test]
    fn held_touch_and_disabled_option_keep_overlay_visible() {
        let now = Instant::now();
        let mut policy = TouchVisibility::default();
        policy.activity(now);
        policy.update_idle(true, true, now + Duration::from_secs(30));
        assert!(policy.visible(true, now + Duration::from_secs(30)));
        assert!(policy.visible(false, now + Duration::from_secs(60)));
        policy.update_idle(false, false, now + Duration::from_secs(60));
        assert!(policy.visible(true, now + Duration::from_secs(60)));
    }

    #[test]
    fn resume_and_disconnect_restart_idle_grace_period() {
        let now = Instant::now();
        let mut policy = TouchVisibility::default();
        let mut display = true;
        policy.sync_devices(true, false, &mut display, now);
        policy.sync_devices(false, false, &mut display, now + Duration::from_secs(50));
        assert!(policy.visible(true, now + Duration::from_secs(50)));
        policy.resume(now + Duration::from_secs(100));
        assert!(policy.visible(true, now + Duration::from_secs(100)));
        policy.sync_devices(true, false, &mut display, now + Duration::from_secs(100));
        assert!(!display);
    }

    #[test]
    fn resume_preserves_manual_overlay_choice_for_existing_controller() {
        let now = Instant::now();
        let mut policy = TouchVisibility::default();
        let mut display = true;
        policy.sync_devices(true, false, &mut display, now);
        display = true;
        policy.resume(now + Duration::from_secs(30));
        policy.sync_devices(true, false, &mut display, now + Duration::from_secs(30));
        assert!(display);
        assert!(policy.visible(true, now + Duration::from_secs(30)));
    }

    #[test]
    fn reconnect_between_frames_hides_even_with_unchanged_capabilities() {
        let now = Instant::now();
        let mut policy = TouchVisibility::default();
        let mut display = true;
        policy.gamepad_connection(1);
        policy.sync_devices(true, false, &mut display, now);
        display = true;
        policy.gamepad_connection(1);
        policy.sync_devices(true, false, &mut display, now);
        assert!(display);
        policy.gamepad_connection(2);
        policy.sync_devices(true, false, &mut display, now);
        assert!(!display);
        display = true;
        policy.sdl_gamepad_added();
        policy.sync_devices(true, false, &mut display, now);
        assert!(!display);
    }
}

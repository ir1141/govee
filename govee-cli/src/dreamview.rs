use std::borrow::Cow;
use std::time::Duration;
use govee_lan::{send_brightness, razer_activate, razer_deactivate};

pub fn segment_count(use_razer: bool, segments: usize) -> usize {
    if use_razer { segments.max(1) } else { 1 }
}

pub fn activate(ip: &str, brightness: u8, use_razer: bool) {
    if let Err(e) = send_brightness(ip, brightness) {
        crate::ui::error(&format!("Failed to set brightness: {e}"));
    }
    if use_razer {
        if let Err(e) = razer_activate(ip) {
            crate::ui::error(&format!("Failed to activate DreamView: {e}"));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

pub fn shutdown(ip: &str, use_razer: bool) {
    crate::ui::status_line_finish();
    if use_razer {
        crate::ui::deactivating();
        let _ = razer_deactivate(ip);
    }
    crate::ui::stopped();
}

pub fn apply_mirror(colors: &[(u8, u8, u8)], mirror: bool) -> Cow<'_, [(u8, u8, u8)]> {
    if mirror {
        let mut mirrored = colors.to_vec();
        mirrored.extend(colors.iter().rev());
        Cow::Owned(mirrored)
    } else {
        Cow::Borrowed(colors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_count_razer_on() {
        assert_eq!(segment_count(true, 5), 5);
        assert_eq!(segment_count(true, 0), 1);
    }

    #[test]
    fn segment_count_razer_off() {
        assert_eq!(segment_count(false, 5), 1);
        assert_eq!(segment_count(false, 0), 1);
    }

    #[test]
    fn mirror_enabled() {
        let colors = vec![(255, 0, 0), (0, 255, 0), (0, 0, 255)];
        let result = apply_mirror(&colors, true);
        assert_eq!(result, vec![
            (255, 0, 0), (0, 255, 0), (0, 0, 255),
            (0, 0, 255), (0, 255, 0), (255, 0, 0),
        ]);
    }

    #[test]
    fn mirror_disabled() {
        let colors = vec![(255, 0, 0), (0, 255, 0)];
        let result = apply_mirror(&colors, false);
        assert_eq!(result, vec![(255, 0, 0), (0, 255, 0)]);
    }
}

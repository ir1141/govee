pub fn hex_to_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

pub fn color_distance(c1: (u8, u8, u8), c2: (u8, u8, u8)) -> f64 {
    let dr = c1.0 as f64 - c2.0 as f64;
    let dg = c1.1 as f64 - c2.1 as f64;
    let db = c1.2 as f64 - c2.2 as f64;
    (dr * dr + dg * dg + db * db).sqrt()
}

pub fn smooth(current: (f64, f64, f64), target: (u8, u8, u8), factor: f64) -> (f64, f64, f64) {
    (
        current.0 + (target.0 as f64 - current.0) * factor,
        current.1 + (target.1 as f64 - current.1) * factor,
        current.2 + (target.2 as f64 - current.2) * factor,
    )
}

pub fn saturate_color(rgb: (u8, u8, u8), amount: f64) -> (u8, u8, u8) {
    let avg = (rgb.0 as f64 + rgb.1 as f64 + rgb.2 as f64) / 3.0;
    let r = (avg + (rgb.0 as f64 - avg) * amount).clamp(0.0, 255.0) as u8;
    let g = (avg + (rgb.1 as f64 - avg) * amount).clamp(0.0, 255.0) as u8;
    let b = (avg + (rgb.2 as f64 - avg) * amount).clamp(0.0, 255.0) as u8;
    (r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_to_rgb_valid() {
        assert_eq!(hex_to_rgb("#ff0000"), Some((255, 0, 0)));
        assert_eq!(hex_to_rgb("00ff00"), Some((0, 255, 0)));
        assert_eq!(hex_to_rgb("#000000"), Some((0, 0, 0)));
        assert_eq!(hex_to_rgb("#ffffff"), Some((255, 255, 255)));
    }

    #[test]
    fn hex_to_rgb_invalid() {
        assert_eq!(hex_to_rgb(""), None);
        assert_eq!(hex_to_rgb("#fff"), None);
        assert_eq!(hex_to_rgb("zzzzzz"), None);
    }

    #[test]
    fn distance_same_color_is_zero() {
        assert_eq!(color_distance((0, 0, 0), (0, 0, 0)), 0.0);
        assert_eq!(color_distance((128, 64, 32), (128, 64, 32)), 0.0);
    }

    #[test]
    fn distance_known_value() {
        let d = color_distance((255, 0, 0), (0, 0, 0));
        assert!((d - 255.0).abs() < 0.01);
    }

    #[test]
    fn smooth_approaches_target() {
        let result = smooth((0.0, 0.0, 0.0), (255, 128, 64), 0.5);
        assert!((result.0 - 127.5).abs() < 0.01);
        assert!((result.1 - 64.0).abs() < 0.01);
        assert!((result.2 - 32.0).abs() < 0.01);
    }

    #[test]
    fn saturate_identity() {
        assert_eq!(saturate_color((100, 150, 200), 1.0), (100, 150, 200));
    }
}

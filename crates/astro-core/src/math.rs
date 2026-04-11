pub fn normalize_degrees(angle_deg: f64) -> f64 {
    angle_deg.rem_euclid(360.0)
}

pub fn degrees_to_radians(angle_deg: f64) -> f64 {
    angle_deg.to_radians()
}

pub fn radians_to_degrees(angle_rad: f64) -> f64 {
    angle_rad.to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_degrees_into_canonical_range() {
        assert!((normalize_degrees(-15.0) - 345.0).abs() < 1e-12);
        assert!((normalize_degrees(725.0) - 5.0).abs() < 1e-12);
    }

    #[test]
    fn degree_radian_helpers_round_trip() {
        let angle = 123.456_f64;
        let round_trip = radians_to_degrees(degrees_to_radians(angle));
        assert!((round_trip - angle).abs() < 1e-12);
    }
}

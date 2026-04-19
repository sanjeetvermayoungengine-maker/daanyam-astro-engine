use astro_vedic::{navamsa_sign, Rashi};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GoldenFixture {
    vectors: Vec<GoldenVector>,
}

#[derive(Debug, Deserialize)]
struct GoldenVector {
    sidereal_longitude_deg: f64,
    expected_d9_rashi: String,
}

#[test]
fn navamsa_reference_vectors_match_expected_rashis() {
    let raw = std::fs::read_to_string("../../tests/golden/navamsa_reference_points.json")
        .expect("golden fixture must exist");
    let fixture: GoldenFixture = serde_json::from_str(&raw).expect("golden fixture must parse");

    for vector in fixture.vectors {
        assert_eq!(
            navamsa_sign(vector.sidereal_longitude_deg),
            parse_rashi(&vector.expected_d9_rashi),
            "longitude {}",
            vector.sidereal_longitude_deg
        );
    }
}

fn parse_rashi(value: &str) -> Rashi {
    match value {
        "mesha" => Rashi::Mesha,
        "vrishabha" => Rashi::Vrishabha,
        "mithuna" => Rashi::Mithuna,
        "karka" => Rashi::Karka,
        "simha" => Rashi::Simha,
        "kanya" => Rashi::Kanya,
        "tula" => Rashi::Tula,
        "vrischika" => Rashi::Vrischika,
        "dhanu" => Rashi::Dhanu,
        "makara" => Rashi::Makara,
        "kumbha" => Rashi::Kumbha,
        "meena" => Rashi::Meena,
        other => panic!("unknown rashi: {other}"),
    }
}

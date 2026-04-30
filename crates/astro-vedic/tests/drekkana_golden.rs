use astro_vedic::{drekkana_sign, Rashi};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GoldenFixture {
    vectors: Vec<GoldenVector>,
}

#[derive(Debug, Deserialize)]
struct GoldenVector {
    sidereal_longitude_deg: f64,
    expected_d3_rashi: String,
}

#[test]
fn golden_drekkana_reference_vectors_match_expected_rashis() {
    let raw = std::fs::read_to_string("../../tests/golden/drekkana_reference_points.json")
        .expect("golden fixture must exist");
    let fixture: GoldenFixture = serde_json::from_str(&raw).expect("golden fixture must parse");

    for vector in fixture.vectors {
        assert_eq!(
            drekkana_sign(vector.sidereal_longitude_deg),
            parse_rashi(&vector.expected_d3_rashi),
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

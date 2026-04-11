use astro_core::time::{julian_day, utc_julian_day_to_tdb_julian_day};
use astro_vedic::{
    lahiri_ayanamsa_deg, moon_sidereal_division_from_tropical, sidereal_longitude_deg, Nakshatra,
    Rashi,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GoldenFixture {
    vectors: Vec<GoldenVector>,
}

#[derive(Debug, Deserialize)]
struct GoldenVector {
    utc: String,
    tropical_longitude_deg: f64,
    expected_ayanamsa_deg: f64,
    expected_sidereal_longitude_deg: f64,
    expected_rashi: String,
    expected_nakshatra: String,
    expected_pada: u8,
}

#[test]
fn lahiri_manual_reference_vectors_match() {
    let raw = std::fs::read_to_string("../../tests/golden/lahiri_moon_nakshatra.json")
        .expect("golden fixture must exist");
    let fixture: GoldenFixture = serde_json::from_str(&raw).expect("golden fixture must parse");

    for vector in fixture.vectors {
        let datetime = chrono::DateTime::parse_from_rfc3339(&vector.utc)
            .expect("utc must parse")
            .with_timezone(&chrono::Utc);
        let jd_tdb = utc_julian_day_to_tdb_julian_day(julian_day(datetime));

        let ayanamsa = lahiri_ayanamsa_deg(jd_tdb);
        let sidereal = sidereal_longitude_deg(vector.tropical_longitude_deg, jd_tdb);
        let division = moon_sidereal_division_from_tropical(vector.tropical_longitude_deg, jd_tdb);

        assert!((ayanamsa - vector.expected_ayanamsa_deg).abs() < 1.0e-9);
        assert!((sidereal - vector.expected_sidereal_longitude_deg).abs() < 1.0e-9);
        assert_eq!(division.rashi, parse_rashi(&vector.expected_rashi));
        assert_eq!(division.nakshatra, parse_nakshatra(&vector.expected_nakshatra));
        assert_eq!(division.pada.0, vector.expected_pada);
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

fn parse_nakshatra(value: &str) -> Nakshatra {
    match value {
        "ashwini" => Nakshatra::Ashwini,
        "bharani" => Nakshatra::Bharani,
        "krittika" => Nakshatra::Krittika,
        "rohini" => Nakshatra::Rohini,
        "mrigashira" => Nakshatra::Mrigashira,
        "ardra" => Nakshatra::Ardra,
        "punarvasu" => Nakshatra::Punarvasu,
        "pushya" => Nakshatra::Pushya,
        "ashlesha" => Nakshatra::Ashlesha,
        "magha" => Nakshatra::Magha,
        "purvaphalguni" => Nakshatra::PurvaPhalguni,
        "uttaraphalguni" => Nakshatra::UttaraPhalguni,
        "hasta" => Nakshatra::Hasta,
        "chitra" => Nakshatra::Chitra,
        "swati" => Nakshatra::Swati,
        "vishakha" => Nakshatra::Vishakha,
        "anuradha" => Nakshatra::Anuradha,
        "jyeshtha" => Nakshatra::Jyeshtha,
        "mula" => Nakshatra::Mula,
        "purvaashadha" => Nakshatra::PurvaAshadha,
        "uttaraashadha" => Nakshatra::UttaraAshadha,
        "shravana" => Nakshatra::Shravana,
        "dhanishta" => Nakshatra::Dhanishta,
        "shatabhisha" => Nakshatra::Shatabhisha,
        "purvabhadrapada" => Nakshatra::PurvaBhadrapada,
        "uttarabhadrapada" => Nakshatra::UttaraBhadrapada,
        "revati" => Nakshatra::Revati,
        other => panic!("unknown nakshatra: {other}"),
    }
}

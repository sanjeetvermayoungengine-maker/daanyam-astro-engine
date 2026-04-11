use std::{fs, path::PathBuf};

use serde_json::Value;

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .expect("crate manifest dir must resolve");
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .expect("workspace root must resolve")
}

fn read_json(path: &str) -> Value {
    let bytes = fs::read_to_string(workspace_root().join(path)).expect("json fixture must load");
    serde_json::from_str(&bytes).expect("json fixture must parse")
}

#[test]
fn openapi_artifact_matches_live_spec_source() {
    let artifact = read_json("dist/openapi.json");
    let generated = astro_api::openapi_spec();
    assert_eq!(artifact, generated);
    assert_eq!(artifact["openapi"], "3.1.0");
    assert!(artifact["paths"]["/chart/sidereal"]["post"].is_object());
    assert!(artifact["paths"]["/positions"]["post"].is_object());
    assert!(artifact["paths"]["/positions/sidereal"]["post"].is_object());
}

#[test]
fn example_contract_artifacts_parse_and_expose_required_fields() {
    let chart = read_json("docs/examples/mobile_chart_compact_sidereal_only.json");
    assert_eq!(chart["data"]["schema_version"], "chart_sidereal_v1");
    assert!(chart["data"]["summary"]["placement_table"].is_array());
    assert!(chart["metadata"]["engine_semantic_version"].is_string());

    let sidereal_positions =
        read_json("docs/examples/mobile_positions_sidereal_compact_sidereal_only.json");
    assert!(sidereal_positions["data"]["positions"].is_array());
    assert!(sidereal_positions["data"]["positions"][0]["sidereal_longitude_deg"].is_number());
    assert!(sidereal_positions["metadata"]["engine_semantic_version"].is_string());

    let tropical_positions = read_json("docs/examples/mobile_positions_tropical_compact.json");
    assert!(tropical_positions["data"]["positions"].is_array());
    assert!(tropical_positions["data"]["positions"][0]["position"]["longitude_deg"].is_number());
    assert!(tropical_positions["metadata"]["engine_semantic_version"].is_string());
}

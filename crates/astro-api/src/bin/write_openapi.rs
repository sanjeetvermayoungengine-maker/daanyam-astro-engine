use std::{fs, path::PathBuf};

fn main() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must resolve");
    let dist_dir = workspace_root.join("dist");
    fs::create_dir_all(&dist_dir).expect("dist directory must be creatable");

    let openapi_path = dist_dir.join("openapi.json");
    let bytes =
        serde_json::to_vec_pretty(&astro_api::openapi_spec()).expect("openapi spec must serialize");
    fs::write(&openapi_path, bytes).expect("openapi artifact must write");
    println!("{}", openapi_path.display());
}

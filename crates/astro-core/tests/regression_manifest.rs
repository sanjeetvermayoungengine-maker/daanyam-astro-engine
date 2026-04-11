use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

mod docs_support;

use docs_support::{
    assert_contains, assert_contains_all, extract_backticked_json_filenames,
    extract_markdown_links, read_repo_doc, workspace_root,
};

#[test]
fn regression_manifest_is_vector_aware() {
    let manifest = read_repo_doc("tests/regression/manifest.json");
    let value: serde_json::Value =
        serde_json::from_str(&manifest).expect("regression manifest must be valid json");

    let groups = value["groups"].as_array().expect("groups field must be an array");
    let fail_on_vectors_present = value["policy"]["fail_on_vectors_present"]
        .as_bool()
        .expect("policy.fail_on_vectors_present must be a boolean");

    assert!(fail_on_vectors_present);
    assert!(!groups.is_empty(), "regression manifest should include official vector groups");
}

#[test]
fn regression_docs_reference_future_plans() {
    let readme = read_repo_doc("README.md");
    let adr_index = read_repo_doc("docs/adr/README.md");
    let chart_doc = read_repo_doc("docs/api_chart_sidereal.md");
    let topocentric_plan = read_repo_doc("docs/topocentric_plan.md");
    let topocentric_policy_adr = read_repo_doc("docs/adr/0003-topocentric-observer-policy.md");
    let mean_nodes_doc = read_repo_doc("docs/mean_nodes.md");
    let positions_doc = read_repo_doc("docs/api_positions.md");
    let sidereal_positions_doc = read_repo_doc("docs/api_positions_sidereal.md");
    let regression_readme = read_repo_doc("tests/regression/README.md");
    let topocentric_readme = read_repo_doc("tests/regression/topocentric/README.md");
    let mean_nodes_readme = read_repo_doc("tests/regression/mean_nodes/README.md");

    assert_contains_all(&adr_index, &["| ID |", "| Title |", "| Status |", "| Link |"]);
    assert_contains_all(&adr_index, &["0002", "API Schema Versioning", "0003"]);
    assert_contains(&adr_index, "Topocentric Observer Policy");
    assert_contains(&readme, "docs/adr/README.md");
    assert_contains(&readme, "docs/adr/0002-api-schema-versioning.md");
    assert_contains(&readme, "docs/api_chart_sidereal.md");
    assert_contains(&chart_doc, "data.extensions");
    assert_contains(&chart_doc, "includes `data.extensions` as an empty object `{}`");
    assert_contains_all(
        &chart_doc,
        &[
            "docs/adr/README.md",
            "0002-api-schema-versioning.md",
            "0003-topocentric-observer-policy.md",
        ],
    );
    assert_contains(&topocentric_plan, "tests/regression/topocentric/");
    assert_contains(&topocentric_plan, "Source of truth for filenames");
    assert_contains(&topocentric_policy_adr, "## Status");
    assert_contains(&topocentric_policy_adr, "Proposed");
    assert_contains(&topocentric_policy_adr, "docs/topocentric_plan.md");
    assert_contains(&topocentric_policy_adr, "tests/regression/topocentric/README.md");
    assert_contains(&mean_nodes_doc, "unsupported");
    assert_contains(&positions_doc, "## Extensions");
    assert_contains(&sidereal_positions_doc, "## Extensions");
    assert_contains(&regression_readme, "docs/topocentric_plan.md");
    assert_contains(&regression_readme, "tests/regression/topocentric/README.md");
    assert_contains(&topocentric_readme, "docs/topocentric_plan.md");
    let frozen_filenames = extract_backticked_json_filenames(&topocentric_readme);
    assert!(
        !frozen_filenames.is_empty(),
        "topocentric README must declare frozen fixture filenames"
    );
    // If this fails, update BOTH docs/topocentric_plan.md and tests/regression/topocentric/README.md.
    let plan_positions = frozen_filenames
        .iter()
        .map(|filename| {
            topocentric_plan
                .find(filename)
                .unwrap_or_else(|| panic!("topocentric plan is missing filename {filename}"))
        })
        .collect::<Vec<_>>();
    assert!(
        plan_positions.windows(2).all(|window| window[0] < window[1]),
        "topocentric plan filenames must match the README ordering"
    );
    assert_contains(&mean_nodes_readme, "docs/mean_nodes.md");
}

#[test]
fn adr_index_matches_adr_files() {
    let adr_index = read_repo_doc("docs/adr/README.md");
    let repo_root = workspace_root();
    let adr_dir = repo_root.join("docs/adr");
    let adr_links = extract_markdown_links(&adr_index);
    let table_rows = adr_index.lines().filter_map(parse_adr_index_row).collect::<Vec<_>>();

    let adr_files = fs::read_dir(&adr_dir)
        .expect("ADR directory must exist")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .filter(|path| path.file_name().is_some_and(|name| name != "README.md"))
        .collect::<Vec<PathBuf>>();

    assert!(!adr_files.is_empty(), "ADR directory must contain indexed ADR files");
    assert!(!table_rows.is_empty(), "ADR index must contain at least one data row");
    let row_ids = table_rows.iter().map(|row| row.numeric_id).collect::<Vec<_>>();
    let unique_ids = row_ids.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(unique_ids.len(), row_ids.len(), "ADR index ids must be unique");
    let mut sorted_ids = row_ids.clone();
    sorted_ids.sort_unstable();
    assert_eq!(row_ids, sorted_ids, "ADR index rows must be sorted by ascending numeric ADR id");

    for path in &adr_files {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("ADR file name must be valid UTF-8");
        let stem = path
            .file_stem()
            .and_then(|name| name.to_str())
            .expect("ADR file stem must be valid UTF-8");
        let id = stem.split('-').next().expect("ADR stem must start with numeric identifier");

        assert!(adr_index.contains(file_name), "ADR index must list file {}", file_name);
        assert!(adr_index.contains(id), "ADR index must list ADR id {}", id);
    }

    for row in &table_rows {
        assert!(
            matches!(row.status.as_str(), "Accepted" | "Proposed"),
            "ADR index row {} uses unsupported status {}",
            row.id,
            row.status
        );
        assert!(
            extract_markdown_links(&row.link).iter().any(|target| target.ends_with(".md")),
            "ADR index row {} must contain a markdown link to an ADR file",
            row.id
        );
    }

    for link_target in adr_links {
        if !link_target.contains("docs/adr/") || !link_target.ends_with(".md") {
            continue;
        }

        let relative_path = link_target
            .strip_prefix(repo_root.to_string_lossy().as_ref())
            .or_else(|| link_target.strip_prefix("/Users/sanjeet/Documents/Playground/"))
            .unwrap_or(&link_target);
        assert!(
            repo_root.join(relative_path.trim_start_matches('/')).exists(),
            "ADR index references missing file: {}",
            link_target
        );
    }
}

struct AdrIndexRow {
    id: String,
    numeric_id: u32,
    status: String,
    link: String,
}

fn parse_adr_index_row(line: &str) -> Option<AdrIndexRow> {
    let trimmed = line.trim();
    if !trimmed.starts_with("| 000") {
        return None;
    }

    let columns =
        trimmed.split('|').map(str::trim).filter(|column| !column.is_empty()).collect::<Vec<_>>();
    assert!(columns.len() >= 4, "ADR index row must have at least four columns: {}", line);
    // Parse rule: ADR ids are the leading zero-padded decimal digits from the first table column.
    let numeric_id = columns[0].parse::<u32>().unwrap_or_else(|_| {
        panic!("ADR index id must be a zero-padded decimal number: {}", columns[0])
    });

    Some(AdrIndexRow {
        id: columns[0].to_owned(),
        numeric_id,
        status: columns[2].to_owned(),
        link: columns[3].to_owned(),
    })
}

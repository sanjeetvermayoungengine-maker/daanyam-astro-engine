use std::fs;
use std::path::PathBuf;

pub fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .expect("crate manifest dir must resolve");
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .expect("workspace root must resolve")
}

pub fn read_repo_doc(rel_path: &str) -> String {
    let path = workspace_root().join(rel_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read repo doc {}: {}", path.display(), err))
}

pub fn assert_contains(haystack: &str, needle: &str) {
    assert!(haystack.contains(needle), "expected doc content to contain: {}", needle);
}

pub fn assert_contains_all(haystack: &str, needles: &[&str]) {
    for needle in needles {
        assert_contains(haystack, needle);
    }
}

pub fn extract_markdown_links(text: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut rest = text;

    while let Some(start) = rest.find("](") {
        let candidate = &rest[start + 2..];
        let Some(end) = candidate.find(')') else {
            break;
        };
        links.push(candidate[..end].to_owned());
        rest = &candidate[end + 1..];
    }

    links
}

pub fn extract_backticked_tokens(text: &str) -> Vec<String> {
    text.split('`')
        .enumerate()
        .filter(|(index, _segment)| index % 2 == 1)
        .map(|(_index, segment)| segment.to_owned())
        .collect()
}

pub fn extract_backticked_json_filenames(text: &str) -> Vec<String> {
    extract_backticked_tokens(text).into_iter().filter(|token| token.ends_with(".json")).collect()
}

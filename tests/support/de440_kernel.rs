use std::path::PathBuf;

pub const TEST_FALLBACK_EPHE_PATH: &str = "/tmp/astro-ephe/de440.bsp";

pub fn de440_kernel_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("ASTRO_EPHE_PATH") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }

    let fallback = PathBuf::from(TEST_FALLBACK_EPHE_PATH);
    if fallback.exists() {
        return Some(fallback);
    }

    None
}

pub fn require_de440_kernel() -> Option<PathBuf> {
    let path = de440_kernel_path();
    if let Some(path) = &path {
        if std::env::var("ASTRO_EPHE_PATH").is_err() {
            std::env::set_var("ASTRO_EPHE_PATH", path);
        }
    } else {
        eprintln!(
            "skipping DE440-backed test: ASTRO_EPHE_PATH is unset and fallback ephemeris file is missing at {TEST_FALLBACK_EPHE_PATH}"
        );
    }

    path
}

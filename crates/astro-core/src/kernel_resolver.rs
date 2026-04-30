use std::{
    env,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use reqwest::{header, Client, StatusCode};
use serde::Deserialize;
use thiserror::Error;
use tokio::{
    fs,
    io::{AsyncWriteExt, BufWriter},
    time::sleep,
};

const ASTRO_EPHE_PATH: &str = "ASTRO_EPHE_PATH";
const ASTRO_EPHE_GCS_URI: &str = "ASTRO_EPHE_GCS_URI";
const KERNEL_GCS_URI: &str = "KERNEL_GCS_URI";
const ASTRO_EPHE_CACHE_DIR: &str = "ASTRO_EPHE_CACHE_DIR";
const ASTRO_GCS_BEARER_TOKEN: &str = "ASTRO_GCS_BEARER_TOKEN";
const ASTRO_GCS_DOWNLOAD_BASE_URL: &str = "ASTRO_GCS_DOWNLOAD_BASE_URL";
const DEFAULT_CACHE_DIR: &str = "/tmp/ephe";
const DEFAULT_DOWNLOAD_BASE_URL: &str = "https://storage.googleapis.com";
const DOWNLOAD_RETRY_ATTEMPTS: usize = 3;
const INITIAL_RETRY_DELAY_MS: u64 = 250;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelResolution {
    pub path: PathBuf,
    pub source: String,
    pub downloaded: bool,
    pub elapsed: Duration,
}

#[derive(Debug, Error)]
pub enum KernelResolverError {
    #[error(
        "set ASTRO_EPHE_PATH to a readable local de440.bsp file or a gs://bucket/object URI, or set ASTRO_EPHE_GCS_URI"
    )]
    MissingSource,
    #[error("local ephemeris path does not exist: {0}")]
    LocalPathMissing(PathBuf),
    #[error("local ephemeris path is not a file: {0}")]
    LocalPathNotFile(PathBuf),
    #[error("gs:// URI is invalid: {0}")]
    InvalidGcsUri(String),
    #[error("I/O error while {context}: {message}")]
    Io { context: &'static str, message: String },
    #[error("request to {url} failed: {message}")]
    Request { url: String, message: String },
    #[error("download from {url} returned HTTP {status}")]
    HttpStatus { url: String, status: StatusCode },
    #[error("metadata server token request failed: {0}")]
    MetadataToken(String),
    #[error("download failed after {attempts} attempts: {last_error}")]
    RetriesExhausted { attempts: usize, last_error: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GcsLocation {
    bucket: String,
    object: String,
}

#[derive(Debug, Deserialize)]
struct MetadataTokenResponse {
    access_token: String,
}

pub async fn resolve_kernel_from_env() -> Result<KernelResolution, KernelResolverError> {
    let source = env::var(ASTRO_EPHE_PATH)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| env::var(ASTRO_EPHE_GCS_URI).ok().filter(|value| !value.trim().is_empty()))
        .or_else(|| env::var(KERNEL_GCS_URI).ok().filter(|value| !value.trim().is_empty()))
        .ok_or(KernelResolverError::MissingSource)?;

    if source.starts_with("gs://") {
        resolve_gcs_kernel(&source).await
    } else {
        resolve_local_kernel(PathBuf::from(source)).await
    }
}

async fn resolve_local_kernel(path: PathBuf) -> Result<KernelResolution, KernelResolverError> {
    let started = Instant::now();
    let metadata = fs::metadata(&path).await.map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => KernelResolverError::LocalPathMissing(path.clone()),
        _ => KernelResolverError::Io {
            context: "reading local ephemeris metadata",
            message: error.to_string(),
        },
    })?;
    if !metadata.is_file() {
        return Err(KernelResolverError::LocalPathNotFile(path));
    }

    Ok(KernelResolution {
        path,
        source: "local_path".to_owned(),
        downloaded: false,
        elapsed: started.elapsed(),
    })
}

async fn resolve_gcs_kernel(uri: &str) -> Result<KernelResolution, KernelResolverError> {
    let started = Instant::now();
    let location = parse_gcs_uri(uri)?;
    let destination = cache_path_for_location(&location);

    if fs::try_exists(&destination).await.map_err(|error| KernelResolverError::Io {
        context: "checking cached ephemeris path",
        message: error.to_string(),
    })? {
        return Ok(KernelResolution {
            path: destination,
            source: uri.to_owned(),
            downloaded: false,
            elapsed: started.elapsed(),
        });
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).await.map_err(|error| KernelResolverError::Io {
            context: "creating ephemeris cache directory",
            message: error.to_string(),
        })?;
    }

    let client = Client::builder().timeout(Duration::from_secs(60)).build().map_err(|error| {
        KernelResolverError::Request { url: download_url(&location), message: error.to_string() }
    })?;

    let token = maybe_bearer_token(&client).await?;
    let url = download_url(&location);
    let temp_path = destination.with_extension("download");
    let mut last_error = None;

    for attempt in 1..=DOWNLOAD_RETRY_ATTEMPTS {
        match download_to_path(&client, &url, token.as_deref(), &temp_path).await {
            Ok(()) => {
                fs::rename(&temp_path, &destination).await.map_err(|error| {
                    KernelResolverError::Io {
                        context: "moving downloaded ephemeris into place",
                        message: error.to_string(),
                    }
                })?;
                return Ok(KernelResolution {
                    path: destination,
                    source: uri.to_owned(),
                    downloaded: true,
                    elapsed: started.elapsed(),
                });
            }
            Err(error) => {
                last_error = Some(error.to_string());
                let _ = fs::remove_file(&temp_path).await;
                if attempt < DOWNLOAD_RETRY_ATTEMPTS {
                    sleep(Duration::from_millis(INITIAL_RETRY_DELAY_MS * attempt as u64)).await;
                }
            }
        }
    }

    Err(KernelResolverError::RetriesExhausted {
        attempts: DOWNLOAD_RETRY_ATTEMPTS,
        last_error: last_error.unwrap_or_else(|| "download failed".to_owned()),
    })
}

async fn download_to_path(
    client: &Client,
    url: &str,
    bearer_token: Option<&str>,
    destination: &Path,
) -> Result<(), KernelResolverError> {
    let mut request = client.get(url);
    if let Some(token) = bearer_token {
        request = request.bearer_auth(token);
    }

    let mut response = request.send().await.map_err(|error| KernelResolverError::Request {
        url: url.to_owned(),
        message: error.to_string(),
    })?;

    if !response.status().is_success() {
        return Err(KernelResolverError::HttpStatus {
            url: url.to_owned(),
            status: response.status(),
        });
    }

    let file = fs::File::create(destination).await.map_err(|error| KernelResolverError::Io {
        context: "creating downloaded ephemeris file",
        message: error.to_string(),
    })?;
    let mut writer = BufWriter::new(file);
    while let Some(chunk) = response.chunk().await.map_err(|error| {
        KernelResolverError::Request { url: url.to_owned(), message: error.to_string() }
    })? {
        writer.write_all(&chunk).await.map_err(|error| KernelResolverError::Io {
            context: "writing downloaded ephemeris file",
            message: error.to_string(),
        })?;
    }
    writer.flush().await.map_err(|error| KernelResolverError::Io {
        context: "flushing downloaded ephemeris file",
        message: error.to_string(),
    })?;
    Ok(())
}

async fn maybe_bearer_token(client: &Client) -> Result<Option<String>, KernelResolverError> {
    if let Ok(token) = env::var(ASTRO_GCS_BEARER_TOKEN) {
        let token = token.trim();
        if !token.is_empty() {
            return Ok(Some(token.to_owned()));
        }
    }

    let metadata_host =
        env::var("GCP_METADATA_HOST").unwrap_or_else(|_| "metadata.google.internal".to_owned());
    let token_url = format!(
        "http://{metadata_host}/computeMetadata/v1/instance/service-accounts/default/token"
    );
    let response = client
        .get(&token_url)
        .header(header::HeaderName::from_static("metadata-flavor"), "Google")
        .timeout(Duration::from_secs(2))
        .send()
        .await;

    let response = match response {
        Ok(response) => response,
        Err(_) => return Ok(None),
    };

    if !response.status().is_success() {
        return Ok(None);
    }

    let token: MetadataTokenResponse = response
        .json()
        .await
        .map_err(|error| KernelResolverError::MetadataToken(error.to_string()))?;
    Ok(Some(token.access_token))
}

fn parse_gcs_uri(uri: &str) -> Result<GcsLocation, KernelResolverError> {
    let remainder = uri
        .strip_prefix("gs://")
        .ok_or_else(|| KernelResolverError::InvalidGcsUri(uri.to_owned()))?;
    let (bucket, object) = remainder
        .split_once('/')
        .ok_or_else(|| KernelResolverError::InvalidGcsUri(uri.to_owned()))?;
    if bucket.is_empty() || object.is_empty() {
        return Err(KernelResolverError::InvalidGcsUri(uri.to_owned()));
    }
    Ok(GcsLocation { bucket: bucket.to_owned(), object: object.to_owned() })
}

fn cache_path_for_location(location: &GcsLocation) -> PathBuf {
    let mut path = PathBuf::from(
        env::var(ASTRO_EPHE_CACHE_DIR).unwrap_or_else(|_| DEFAULT_CACHE_DIR.to_owned()),
    );
    path.push(&location.bucket);
    for part in location.object.split('/') {
        path.push(part);
    }
    path
}

fn download_url(location: &GcsLocation) -> String {
    let base = env::var(ASTRO_GCS_DOWNLOAD_BASE_URL)
        .unwrap_or_else(|_| DEFAULT_DOWNLOAD_BASE_URL.to_owned());
    let object = utf8_percent_encode(&location.object, NON_ALPHANUMERIC).to_string();
    format!("{base}/{}/{}", location.bucket, object)
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, OnceLock};

    use super::*;

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().expect("env lock poisoned")
    }

    #[tokio::test(flavor = "current_thread")]
    async fn resolves_existing_local_kernel_path() {
        let _guard = env_lock();
        let temp_dir =
            std::env::temp_dir().join(format!("astro-core-kernel-local-{}", std::process::id()));
        fs::create_dir_all(&temp_dir).await.expect("temp dir must exist");
        let kernel_path = temp_dir.join("de440.bsp");
        fs::write(&kernel_path, b"de440").await.expect("kernel file must be writable");
        env::set_var(ASTRO_EPHE_PATH, &kernel_path);
        env::remove_var(ASTRO_EPHE_GCS_URI);

        let resolution = resolve_kernel_from_env().await.expect("local kernel must resolve");
        assert_eq!(resolution.path, kernel_path);
        assert!(!resolution.downloaded);

        env::remove_var(ASTRO_EPHE_PATH);
        let _ = fs::remove_file(&resolution.path).await;
        let _ = fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn reuses_cached_kernel_from_gcs_uri() {
        let _guard = env_lock();
        let cache_dir =
            std::env::temp_dir().join(format!("astro-core-kernel-cache-{}", std::process::id()));
        let cached_kernel_path = cache_dir.join("daanyam-ephe").join("de440.bsp");
        fs::create_dir_all(cached_kernel_path.parent().expect("cache parent must exist"))
            .await
            .expect("cache parent must be creatable");
        fs::write(&cached_kernel_path, b"de440").await.expect("cached kernel must be writable");

        env::remove_var(ASTRO_EPHE_PATH);
        env::set_var(ASTRO_EPHE_GCS_URI, "gs://daanyam-ephe/de440.bsp");
        env::set_var(ASTRO_EPHE_CACHE_DIR, &cache_dir);

        let resolution = resolve_kernel_from_env().await.expect("cached gcs kernel must resolve");
        assert_eq!(resolution.path, cached_kernel_path);
        assert!(!resolution.downloaded);
        assert_eq!(fs::read(&resolution.path).await.expect("downloaded file must exist"), b"de440");

        env::remove_var(ASTRO_EPHE_GCS_URI);
        env::remove_var(ASTRO_EPHE_CACHE_DIR);
        let _ = fs::remove_dir_all(&cache_dir).await;
    }
}

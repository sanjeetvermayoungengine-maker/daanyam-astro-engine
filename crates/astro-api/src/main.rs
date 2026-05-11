use std::{env, net::SocketAddr};

use astro_api::{app_router, demo_state, ApiState};
use astro_core::{
    kernel_resolver::{resolve_kernel_from_env, KernelResolution},
    time::julian_day,
    De440Backend,
};
use chrono::{TimeZone, Utc};
use sha2::Digest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackendMode {
    De440,
    Demo,
}

impl BackendMode {
    fn from_env(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("demo") {
            "de440" => Ok(Self::De440),
            "demo" => Ok(Self::Demo),
            other => {
                Err(format!("invalid ASTRO_BACKEND value `{other}`; expected `de440` or `demo`"))
            }
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::De440 => "de440",
            Self::Demo => "demo",
        }
    }
}

fn bind_addr_from_env_vars(host: Option<&str>, port: Option<&str>) -> Result<SocketAddr, String> {
    let host = host.unwrap_or("127.0.0.1");
    let port = port.unwrap_or("3000");
    let port =
        port.parse::<u16>().map_err(|error| format!("invalid PORT value `{port}`: {error}"))?;
    format!("{host}:{port}")
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid HOST/PORT combination `{host}:{port}`: {error}"))
}

fn env_var_is_true(name: &str) -> bool {
    matches!(
        env::var(name).ok().as_deref(),
        Some("1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON")
    )
}

fn production_env_var() -> Option<&'static str> {
    ["ENVIRONMENT", "NODE_ENV"].into_iter().find(|name| {
        env::var(name).ok().is_some_and(|value| value.eq_ignore_ascii_case("production"))
    })
}

fn ensure_demo_backend_allowed(backend_mode: BackendMode) -> Result<(), String> {
    if backend_mode == BackendMode::Demo
        && production_env_var().is_some()
        && !env_var_is_true("ALLOW_DEMO_BACKEND")
    {
        let env_name = production_env_var().expect("production env var must exist");
        return Err(format!(
            "refusing to start with ASTRO_BACKEND=demo when {env_name}=production; set ALLOW_DEMO_BACKEND=true to override"
        ));
    }
    Ok(())
}

fn coverage_window_confirmed(backend: &De440Backend) -> bool {
    let (coverage_start_jd, coverage_end_jd) = backend.coverage_range_jd();
    let coverage_start = julian_day(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap());
    let coverage_end = julian_day(Utc.with_ymd_and_hms(2150, 1, 1, 0, 0, 0).unwrap());
    coverage_start_jd <= coverage_start && coverage_end_jd >= coverage_end
}

async fn app_state_from_env() -> Result<(ApiState, BackendMode, Option<KernelResolution>), String> {
    let backend_mode = BackendMode::from_env(env::var("ASTRO_BACKEND").ok().as_deref())?;
    ensure_demo_backend_allowed(backend_mode)?;
    match backend_mode {
        BackendMode::Demo => Ok((demo_state(), backend_mode, None)),
        BackendMode::De440 => {
            let resolution = resolve_kernel_from_env().await.map_err(|error| {
                format!("failed to resolve DE440 kernel from runtime environment: {error}")
            })?;
            let backend = De440Backend::from_path(&resolution.path).map_err(|error| {
                format!(
                    "failed to initialize DE440 backend from resolved path `{}`: {error}",
                    resolution.path.display()
                )
            })?;
            if !coverage_window_confirmed(&backend) {
                return Err(format!(
                    "resolved DE440 kernel at `{}` does not cover the required 2024–2150 window",
                    resolution.path.display()
                ));
            }
            let mut kernel_hasher = sha2::Sha256::new();
            kernel_hasher.update(resolution.source.as_bytes());
            kernel_hasher.update(resolution.path.display().to_string().as_bytes());
            let kernel_hash = format!("{:x}", kernel_hasher.finalize());
            let state = ApiState::new(
                std::sync::Arc::new(backend),
                astro_core::EngineConfig::default(),
                env!("CARGO_PKG_VERSION"),
            )
            .with_kernel_provenance(kernel_hash, resolution.elapsed.as_secs_f64());
            Ok((state, backend_mode, Some(resolution)))
        }
    }
}

#[tokio::main]
async fn main() {
    let (state, backend_mode, kernel_resolution) =
        app_state_from_env().await.expect("runtime backend initialization must succeed");
    if let Some(kernel_resolution) = kernel_resolution {
        eprintln!(
            "INFO de440 kernel loaded from {} in {}ms, 2024–2150 coverage confirmed",
            kernel_resolution.source,
            kernel_resolution.elapsed.as_millis()
        );
    }
    eprintln!("astro-api starting with ASTRO_BACKEND={}", backend_mode.as_str());
    let app = app_router(state);
    let addr =
        bind_addr_from_env_vars(env::var("HOST").ok().as_deref(), env::var("PORT").ok().as_deref())
            .expect("HOST and PORT must form a valid socket address");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("failed to bind server socket");
    axum::serve(listener, app).await.expect("server failed");
}

#[cfg(test)]
mod tests {
    use super::{
        app_state_from_env, bind_addr_from_env_vars, ensure_demo_backend_allowed, BackendMode,
    };
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().expect("env lock poisoned")
    }

    #[test]
    fn bind_addr_defaults_to_localhost_3000() {
        assert_eq!(
            bind_addr_from_env_vars(None, None).expect("default bind address must parse"),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3000)
        );
    }

    #[test]
    fn bind_addr_uses_explicit_host_and_port() {
        assert_eq!(
            bind_addr_from_env_vars(Some("0.0.0.0"), Some("8080"))
                .expect("explicit bind address must parse"),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 8080)
        );
    }

    #[test]
    fn backend_mode_defaults_to_demo() {
        assert_eq!(BackendMode::from_env(None).expect("default backend"), BackendMode::Demo);
    }

    #[test]
    fn backend_mode_rejects_unknown_value() {
        let error = BackendMode::from_env(Some("invalid")).expect_err("invalid backend must fail");
        assert!(error.contains("ASTRO_BACKEND"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn app_state_uses_demo_without_kernel() {
        let _guard = env_lock();
        std::env::remove_var("ASTRO_BACKEND");
        std::env::remove_var("ASTRO_EPHE_PATH");
        std::env::remove_var("ASTRO_EPHE_GCS_URI");
        std::env::remove_var("ENVIRONMENT");
        std::env::remove_var("NODE_ENV");
        std::env::remove_var("ALLOW_DEMO_BACKEND");
        let (_, mode, kernel_resolution) =
            app_state_from_env().await.expect("demo backend must initialize without kernel");
        assert_eq!(mode, BackendMode::Demo);
        assert!(kernel_resolution.is_none());
    }

    #[test]
    fn production_environment_rejects_demo_backend_by_default() {
        let _guard = env_lock();
        std::env::set_var("ENVIRONMENT", "production");
        std::env::remove_var("NODE_ENV");
        std::env::remove_var("ALLOW_DEMO_BACKEND");
        let error =
            ensure_demo_backend_allowed(BackendMode::Demo).expect_err("demo must be rejected");
        assert!(error.contains("ALLOW_DEMO_BACKEND=true"));
        std::env::remove_var("ENVIRONMENT");
    }

    #[test]
    fn node_env_production_rejects_demo_backend_by_default() {
        let _guard = env_lock();
        std::env::remove_var("ENVIRONMENT");
        std::env::set_var("NODE_ENV", "production");
        std::env::remove_var("ALLOW_DEMO_BACKEND");
        let error =
            ensure_demo_backend_allowed(BackendMode::Demo).expect_err("demo must be rejected");
        assert!(error.contains("NODE_ENV=production"));
        std::env::remove_var("NODE_ENV");
    }

    #[test]
    fn production_environment_allows_demo_backend_with_explicit_override() {
        let _guard = env_lock();
        std::env::set_var("ENVIRONMENT", "production");
        std::env::set_var("ALLOW_DEMO_BACKEND", "true");
        ensure_demo_backend_allowed(BackendMode::Demo)
            .expect("explicit override must allow demo backend");
        std::env::remove_var("ENVIRONMENT");
        std::env::remove_var("ALLOW_DEMO_BACKEND");
    }

    #[test]
    fn production_environment_allows_de440_backend_without_override() {
        let _guard = env_lock();
        std::env::set_var("ENVIRONMENT", "production");
        std::env::remove_var("ALLOW_DEMO_BACKEND");
        ensure_demo_backend_allowed(BackendMode::De440)
            .expect("real backend should be allowed in production");
        std::env::remove_var("ENVIRONMENT");
    }
}

use std::{env, net::SocketAddr};

use astro_api::{app_router, de440_state_from_env, demo_state, ApiState};

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

fn app_state_from_env() -> Result<(ApiState, BackendMode, bool), String> {
    let backend_mode = BackendMode::from_env(env::var("ASTRO_BACKEND").ok().as_deref())?;
    ensure_demo_backend_allowed(backend_mode)?;
    match backend_mode {
        BackendMode::Demo => Ok((demo_state(), backend_mode, false)),
        BackendMode::De440 => {
            let kernel_path = env::var("ASTRO_EPHE_PATH").map_err(|_| {
                "ASTRO_BACKEND=de440 requires ASTRO_EPHE_PATH to point to a readable de440.bsp file"
                    .to_owned()
            })?;
            let state = de440_state_from_env().map_err(|error| {
                format!(
                    "failed to initialize DE440 backend from ASTRO_EPHE_PATH=`{kernel_path}`: {error}"
                )
            })?;
            Ok((state, backend_mode, true))
        }
    }
}

#[tokio::main]
async fn main() {
    let (state, backend_mode, kernel_loaded) =
        app_state_from_env().expect("runtime backend initialization must succeed");
    eprintln!(
        "astro-api starting with ASTRO_BACKEND={} kernel_loaded={kernel_loaded}",
        backend_mode.as_str()
    );
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

    #[test]
    fn app_state_uses_demo_without_kernel() {
        let _guard = env_lock();
        std::env::remove_var("ASTRO_BACKEND");
        std::env::remove_var("ASTRO_EPHE_PATH");
        std::env::remove_var("ENVIRONMENT");
        std::env::remove_var("NODE_ENV");
        std::env::remove_var("ALLOW_DEMO_BACKEND");
        let (_, mode, kernel_loaded) =
            app_state_from_env().expect("demo backend must initialize without kernel");
        assert_eq!(mode, BackendMode::Demo);
        assert!(!kernel_loaded);
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

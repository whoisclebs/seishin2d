use std::{
    error::Error,
    path::{Path, PathBuf},
    sync::Once,
};

use tracing_subscriber::EnvFilter;

type PlatformResult<T> = Result<T, Box<dyn Error>>;

#[cfg(not(target_arch = "wasm32"))]
pub fn project_path(path: &Path) -> std::io::Result<PathBuf> {
    std::fs::canonicalize(path)
}

#[cfg(target_arch = "wasm32")]
pub fn project_path(path: &Path) -> std::io::Result<PathBuf> {
    Ok(path.to_path_buf())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn discover_project_file() -> PlatformResult<PathBuf> {
    let current_dir = std::env::current_dir()?;

    for directory in current_dir.ancestors() {
        let candidate = directory.join("Seishin.toml");

        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err("Seishin.toml not found. Expected a Seishin.toml file in the current directory or a parent directory. Use App::from_project(path) for an explicit project path.".into())
}

#[cfg(target_arch = "wasm32")]
pub fn discover_project_file() -> PlatformResult<PathBuf> {
    Ok(PathBuf::from("Seishin.toml"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_to_string(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

#[cfg(target_arch = "wasm32")]
pub fn read_to_string(path: &Path) -> std::io::Result<String> {
    use web_sys::XmlHttpRequest;

    let url = path.to_string_lossy().replace('\\', "/");
    let request = XmlHttpRequest::new().map_err(|_| std::io::ErrorKind::Other)?;
    request
        .open_with_async("GET", &url, false)
        .map_err(|_| std::io::ErrorKind::Other)?;
    request.send().map_err(|_| std::io::ErrorKind::Other)?;

    match request.status().unwrap_or(0) {
        200..=299 => request
            .response_text()
            .map_err(|_| std::io::ErrorKind::Other)?
            .ok_or_else(|| std::io::ErrorKind::UnexpectedEof.into()),
        404 => Err(std::io::ErrorKind::NotFound.into()),
        _ => Err(std::io::ErrorKind::Other.into()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ensure_readable_file(path: &Path) -> std::io::Result<()> {
    if path.is_file() {
        Ok(())
    } else {
        Err(std::io::ErrorKind::NotFound.into())
    }
}

#[cfg(target_arch = "wasm32")]
pub fn ensure_readable_file(path: &Path) -> std::io::Result<()> {
    read_to_string(path).map(|_| ())
}

pub fn install_logging(init: &'static Once, default_filter: String) {
    init.call_once(move || {
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));

        #[cfg(target_arch = "wasm32")]
        let subscriber = tracing_subscriber::fmt().without_time();

        #[cfg(not(target_arch = "wasm32"))]
        let subscriber = tracing_subscriber::fmt();

        let _ = subscriber.with_env_filter(env_filter).try_init();
    });
}

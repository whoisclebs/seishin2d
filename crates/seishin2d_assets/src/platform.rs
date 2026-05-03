use std::{io, path::PathBuf};

use crate::AssetError;

#[cfg(not(target_arch = "wasm32"))]
pub fn asset_root(path: &std::path::Path) -> Result<PathBuf, AssetError> {
    let canonical = std::fs::canonicalize(path)
        .map_err(|_| AssetError::InvalidAssetRoot(path.to_path_buf()))?;

    if !canonical.is_dir() {
        return Err(AssetError::InvalidAssetRoot(canonical));
    }

    Ok(canonical)
}

#[cfg(target_arch = "wasm32")]
pub fn asset_root(path: &std::path::Path) -> Result<PathBuf, AssetError> {
    Ok(path.to_path_buf())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_bytes(path: &std::path::Path) -> Result<Vec<u8>, AssetError> {
    std::fs::read(path).map_err(|error| map_io_error(path.to_path_buf(), error))
}

#[cfg(target_arch = "wasm32")]
pub fn read_bytes(path: &std::path::Path) -> Result<Vec<u8>, AssetError> {
    use web_sys::XmlHttpRequest;

    let url = path.to_string_lossy().replace('\\', "/");
    let request = XmlHttpRequest::new().map_err(|_| AssetError::Io {
        path: path.to_path_buf(),
        kind: io::ErrorKind::Other,
    })?;
    request
        .open_with_async("GET", &url, false)
        .map_err(|_| AssetError::Io {
            path: path.to_path_buf(),
            kind: io::ErrorKind::Other,
        })?;
    request
        .override_mime_type("text/plain; charset=x-user-defined")
        .map_err(|_| AssetError::Io {
            path: path.to_path_buf(),
            kind: io::ErrorKind::Other,
        })?;
    request.send().map_err(|_| AssetError::Io {
        path: path.to_path_buf(),
        kind: io::ErrorKind::Other,
    })?;

    let status = request.status().unwrap_or(0);
    if status == 404 {
        return Err(AssetError::NotFound(path.to_path_buf()));
    }

    if !(200..300).contains(&status) {
        return Err(AssetError::Io {
            path: path.to_path_buf(),
            kind: io::ErrorKind::Other,
        });
    }

    let response = request.response_text().map_err(|_| AssetError::Io {
        path: path.to_path_buf(),
        kind: io::ErrorKind::Other,
    })?;
    let bytes = response
        .ok_or_else(|| AssetError::Io {
            path: path.to_path_buf(),
            kind: io::ErrorKind::UnexpectedEof,
        })?
        .encode_utf16()
        .map(|code_unit| (code_unit & 0xff) as u8)
        .collect();

    Ok(bytes)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn canonical_asset_path(
    root: &std::path::Path,
    joined: PathBuf,
) -> Result<PathBuf, AssetError> {
    if !joined.exists() {
        return Err(AssetError::NotFound(joined));
    }

    let canonical = std::fs::canonicalize(&joined).map_err(|error| map_io_error(joined, error))?;

    if !canonical.starts_with(root) {
        return Err(AssetError::PathOutsideRoot(canonical));
    }

    Ok(canonical)
}

#[cfg(target_arch = "wasm32")]
pub fn canonical_asset_path(
    _root: &std::path::Path,
    joined: PathBuf,
) -> Result<PathBuf, AssetError> {
    Ok(joined)
}

#[cfg(not(target_arch = "wasm32"))]
fn map_io_error(path: PathBuf, error: io::Error) -> AssetError {
    match error.kind() {
        io::ErrorKind::NotFound => AssetError::NotFound(path),
        kind => AssetError::Io { path, kind },
    }
}

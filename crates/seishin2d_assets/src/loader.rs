use std::io;

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
use std::fs;

use crate::{decode_image, AssetError, AssetPath, AssetRoot, ImageData};

#[derive(Debug, Clone)]
pub struct AssetLoader {
    root: AssetRoot,
}

impl AssetLoader {
    pub fn new(root: AssetRoot) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &AssetRoot {
        &self.root
    }

    pub fn load_image(&self, asset_path: &AssetPath) -> Result<ImageData, AssetError> {
        #[cfg(target_arch = "wasm32")]
        {
            let url = self.root.resolve(asset_path);
            let bytes = fetch_bytes(&url)?;

            return decode_image(&url, &bytes);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let disk_path = self.resolve_existing_path(asset_path)?;
            let bytes =
                fs::read(&disk_path).map_err(|error| map_io_error(disk_path.clone(), error))?;

            decode_image(&disk_path, &bytes)
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn resolve_existing_path(&self, asset_path: &AssetPath) -> Result<PathBuf, AssetError> {
        let joined = self.root.resolve(asset_path);

        if !joined.exists() {
            return Err(AssetError::NotFound(joined));
        }

        let canonical = fs::canonicalize(&joined).map_err(|error| map_io_error(joined, error))?;

        if !canonical.starts_with(self.root.path()) {
            return Err(AssetError::PathOutsideRoot(canonical));
        }

        Ok(canonical)
    }
}

#[cfg(target_arch = "wasm32")]
fn fetch_bytes(path: &std::path::Path) -> Result<Vec<u8>, AssetError> {
    use js_sys::Uint8Array;
    use web_sys::{XmlHttpRequest, XmlHttpRequestResponseType};

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
    request.set_response_type(XmlHttpRequestResponseType::Arraybuffer);
    request.send().map_err(|_| AssetError::Io {
        path: path.to_path_buf(),
        kind: io::ErrorKind::Other,
    })?;

    if request.status().unwrap_or(0) == 404 {
        return Err(AssetError::NotFound(path.to_path_buf()));
    }

    if !(200..300).contains(&request.status().unwrap_or(0)) {
        return Err(AssetError::Io {
            path: path.to_path_buf(),
            kind: io::ErrorKind::Other,
        });
    }

    let response = request.response().map_err(|_| AssetError::Io {
        path: path.to_path_buf(),
        kind: io::ErrorKind::Other,
    })?;
    let bytes = Uint8Array::new(&response).to_vec();

    Ok(bytes)
}

#[cfg(not(target_arch = "wasm32"))]
fn map_io_error(path: PathBuf, error: io::Error) -> AssetError {
    match error.kind() {
        io::ErrorKind::NotFound => AssetError::NotFound(path),
        kind => AssetError::Io { path, kind },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{codecs::png::PngEncoder, ColorType, ImageEncoder};
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn valid_image_file_under_root_loads_without_gpu_dependencies() {
        let root_dir = unique_test_dir();
        let sprite_path = root_dir.join("sprites").join("player.png");

        fs::create_dir_all(sprite_path.parent().expect("sprite parent"))
            .expect("create asset tree");
        fs::write(&sprite_path, valid_png_bytes()).expect("write image fixture");

        let loader = AssetLoader::new(AssetRoot::new(&root_dir).expect("asset root"));
        let asset_path = AssetPath::new("sprites/player.png").expect("asset path");

        let image = loader.load_image(&asset_path).expect("image should load");

        assert_eq!(image.width(), 1);
        assert_eq!(image.height(), 1);
        assert_eq!(image.pixels_rgba8().len(), 4);

        cleanup_test_dir(root_dir);
    }

    #[test]
    fn missing_files_return_controlled_error() {
        let root_dir = unique_test_dir();
        fs::create_dir_all(&root_dir).expect("create root");

        let loader = AssetLoader::new(AssetRoot::new(&root_dir).expect("asset root"));
        let asset_path = AssetPath::new("sprites/missing.png").expect("asset path");
        let expected_path = loader.root().path().join("sprites").join("missing.png");

        let error = loader
            .load_image(&asset_path)
            .expect_err("missing file must fail");

        assert_eq!(error, AssetError::NotFound(expected_path));

        cleanup_test_dir(root_dir);
    }

    #[test]
    fn invalid_image_bytes_return_controlled_decode_error() {
        let root_dir = unique_test_dir();
        let sprite_path = root_dir.join("sprites").join("corrupt.png");

        fs::create_dir_all(sprite_path.parent().expect("sprite parent"))
            .expect("create asset tree");
        fs::write(&sprite_path, b"not a png").expect("write corrupt image");

        let loader = AssetLoader::new(AssetRoot::new(&root_dir).expect("asset root"));
        let asset_path = AssetPath::new("sprites/corrupt.png").expect("asset path");
        let expected_path = loader.root().path().join("sprites").join("corrupt.png");

        let error = loader
            .load_image(&asset_path)
            .expect_err("decode must fail");

        assert_eq!(error, AssetError::ImageDecode(expected_path));

        cleanup_test_dir(root_dir);
    }

    fn unique_test_dir() -> PathBuf {
        let unique = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("valid clock")
            .as_nanos();
        std::env::temp_dir().join(format!("seishin2d_assets_test_{nanos}_{unique}"))
    }

    fn cleanup_test_dir(path: PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    fn valid_png_bytes() -> Vec<u8> {
        let mut bytes = Vec::new();
        PngEncoder::new(&mut bytes)
            .write_image(&[255, 0, 0, 255], 1, 1, ColorType::Rgba8)
            .expect("encode png fixture");
        bytes
    }
}

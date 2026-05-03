mod app;
mod platform;

#[cfg(target_arch = "wasm32")]
pub use platform::preload_web_resources;
#[cfg(target_arch = "wasm32")]
pub use seishin_assets::preload_web_assets;
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen_futures::spawn_local;

pub use app::{
    run, ActiveDialogue, App, Assets, CharacterData, CharacterDialogueData, Component2D,
    ComponentRegistry, DialogueData, DialogueState, Entity, EntityMut, FrameContext, Game2D,
    GameResult, GameplayInput, InputActions, InputQuery, LogLevel, RenderContext, ResourceToml,
    Resources, SpriteBuilder, SpriteBundle, SpriteRenderer, StartupContext, Texture, Vec2, World,
};

#[cfg(target_arch = "wasm32")]
#[derive(Debug, serde::Deserialize)]
pub struct WebManifest {
    #[serde(default)]
    pub resources: Vec<String>,
    #[serde(default)]
    pub assets: Vec<String>,
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_web_manifest(path: &str) -> Result<WebManifest, wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window =
        web_sys::window().ok_or_else(|| wasm_bindgen::JsValue::from_str("window unavailable"))?;
    let response = JsFuture::from(window.fetch_with_str(path)).await?;
    let response = response.dyn_into::<web_sys::Response>()?;
    if !response.ok() {
        return Err(wasm_bindgen::JsValue::from_str(&format!(
            "failed to fetch {path}: HTTP {}",
            response.status()
        )));
    }

    let text = JsFuture::from(response.text()?).await?;
    let text = text
        .as_string()
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("web manifest response was not text"))?;
    let manifest = text
        .parse::<toml::Value>()
        .map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))?;
    Ok(WebManifest {
        resources: manifest_array(&manifest, "resources")?,
        assets: manifest_array(&manifest, "assets")?,
    })
}

#[cfg(target_arch = "wasm32")]
fn manifest_array(manifest: &toml::Value, key: &str) -> Result<Vec<String>, wasm_bindgen::JsValue> {
    manifest
        .get(key)
        .and_then(toml::Value::as_array)
        .ok_or_else(|| wasm_bindgen::JsValue::from_str(&format!("web manifest missing `{key}`")))?
        .iter()
        .map(|value| {
            value.as_str().map(ToOwned::to_owned).ok_or_else(|| {
                wasm_bindgen::JsValue::from_str(&format!(
                    "web manifest `{key}` entry is not a string"
                ))
            })
        })
        .collect()
}

pub mod assets {
    pub use seishin_assets::*;
}

pub mod audio {
    pub use seishin_audio::*;
}

pub mod core {
    pub use seishin_core::*;
}

pub mod input {
    pub use seishin_input::*;
}

pub mod physics {
    pub use seishin_physics::*;
}

pub mod render {
    pub use seishin_render::*;
}

pub mod runtime {
    pub use seishin_runtime::*;
}

pub mod prelude {
    pub use crate::{
        run, ActiveDialogue, App, Assets, CharacterData, CharacterDialogueData, Component2D,
        ComponentRegistry, DialogueData, DialogueState, Entity, EntityMut, FrameContext, Game2D,
        GameResult, GameplayInput, InputActions, InputQuery, LogLevel, RenderContext, ResourceToml,
        Resources, SpriteBuilder, SpriteBundle, SpriteRenderer, StartupContext, Texture, Vec2,
        World,
    };
    pub use seishin_assets::{AssetHandle, AssetLoader, AssetPath, AssetRoot};
    pub use seishin_audio::{AudioSkipReason, AudioSystem, PlaybackResult, SoundAsset};
    pub use seishin_core::{
        Engine, EngineConfig, EngineError, EngineResult, EntityId, Game, Transform2D, UpdateContext,
    };
    pub use seishin_input::{InputState, KeyCode};
    pub use seishin_physics::Collider2D;
    pub use seishin_render::{
        Camera2D, ClearColor, RenderError, RenderSize, RenderState, Sprite, TextureData, TextureId,
    };
    pub use seishin_runtime::{
        run_desktop, run_headless, DesktopGame, DesktopRunConfig, DesktopRuntimeError,
        FixedTimestep, HeadlessRunConfig, WindowConfig, WindowSize,
    };
}

#[macro_export]
macro_rules! seishin_main {
    ($game:ty) => {
        #[cfg(not(target_arch = "wasm32"))]
        fn main() -> $crate::GameResult<()> {
            $crate::run::<$game>()
        }

        #[cfg(target_arch = "wasm32")]
        fn main() {}

        #[cfg(target_arch = "wasm32")]
        #[wasm_bindgen::prelude::wasm_bindgen(start)]
        pub fn wasm_start() {
            $crate::spawn_local(async {
                let manifest = match $crate::fetch_web_manifest("web-manifest.json").await {
                    Ok(manifest) => manifest,
                    Err(error) => {
                        report_web_startup_error(&format!("seishin web manifest preload failed: {error:?}"));
                        return;
                    }
                };

                if let Err(error) = $crate::preload_web_resources(&manifest.resources).await {
                    report_web_startup_error(&format!(
                        "seishin web resource preload failed: {error:?}"
                    ));
                    return;
                }

                if let Err(error) = $crate::preload_web_assets(&manifest.assets).await {
                    report_web_startup_error(&format!(
                        "seishin web asset preload failed: {error:?}"
                    ));
                    return;
                }

                if let Err(error) = $crate::run::<$game>() {
                    report_web_startup_error(&format!("seishin web startup failed: {error}"));
                }
            });

            fn report_web_startup_error(message: &str) {
                web_sys::console::error_1(&message.into());

                if let Some(document) = web_sys::window().and_then(|window| window.document()) {
                    if let Ok(element) = document.create_element("pre") {
                        element.set_text_content(Some(message));
                        let _ = element.set_attribute(
                            "style",
                            "margin:16px;padding:12px;color:#ffb4b4;background:#240909;border:1px solid #7f1d1d;white-space:pre-wrap;",
                        );
                        if let Some(body) = document.body() {
                            let _ = body.append_child(&element);
                        }
                    }
                }
            }
        }
    };
}

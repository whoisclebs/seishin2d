mod app;
mod platform;

pub use app::{
    run, ActiveDialogue, App, Assets, CharacterData, CharacterDialogueData, Component2D,
    ComponentRegistry, DialogueData, DialogueState, Entity, EntityMut, FrameContext, Game2D,
    GameResult, GameplayInput, InputActions, InputQuery, LogLevel, RenderContext, ResourceToml,
    Resources, SpriteBuilder, SpriteBundle, SpriteRenderer, StartupContext, Texture, Vec2, World,
};

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
            if let Err(error) = $crate::run::<$game>() {
                let message = format!("seishin web startup failed: {error}");
                web_sys::console::error_1(&message.clone().into());

                if let Some(document) = web_sys::window().and_then(|window| window.document()) {
                    if let Ok(element) = document.create_element("pre") {
                        element.set_text_content(Some(&message));
                        let _ = element.set_attribute(
                            "style",
                            "margin:16px;padding:12px;color:#ffb4b4;background:#240909;border:1px solid #7f1d1d;white-space:pre-wrap;",
                        );
                        if let Some(body) = document.body() {
                            let _ = body.append_child(&element);
                        }
                    }
                }

                wasm_bindgen::throw_str(&message);
            }
        }
    };
}

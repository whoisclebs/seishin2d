mod app;

pub use app::{
    run, ActiveDialogue, App, Assets, CharacterData, CharacterDialogueData, Component2D,
    ComponentRegistry, DialogueData, DialogueState, Entity, EntityMut, FrameContext, Game2D,
    GameResult, GameplayInput, InputActions, InputQuery, LogLevel, RenderContext, ResourceToml,
    Resources, SpriteBuilder, SpriteBundle, SpriteRenderer, StartupContext, Texture, Vec2, World,
};

pub mod assets {
    pub use seishin2d_assets::*;
}

pub mod audio {
    pub use seishin2d_audio::*;
}

pub mod core {
    pub use seishin2d_core::*;
}

pub mod input {
    pub use seishin2d_input::*;
}

pub mod physics {
    pub use seishin2d_physics::*;
}

pub mod render {
    pub use seishin2d_render::*;
}

pub mod runtime {
    pub use seishin2d_runtime::*;
}

pub mod prelude {
    pub use crate::{
        run, ActiveDialogue, App, Assets, CharacterData, CharacterDialogueData, Component2D,
        ComponentRegistry, DialogueData, DialogueState, Entity, EntityMut, FrameContext, Game2D,
        GameResult, GameplayInput, InputActions, InputQuery, LogLevel, RenderContext, ResourceToml,
        Resources, SpriteBuilder, SpriteBundle, SpriteRenderer, StartupContext, Texture, Vec2,
        World,
    };
    pub use seishin2d_assets::{AssetHandle, AssetLoader, AssetPath, AssetRoot};
    pub use seishin2d_audio::{AudioSkipReason, AudioSystem, PlaybackResult, SoundAsset};
    pub use seishin2d_core::{
        Engine, EngineConfig, EngineError, EngineResult, EntityId, Game, Transform2D, UpdateContext,
    };
    pub use seishin2d_input::{InputState, KeyCode};
    pub use seishin2d_physics::Collider2D;
    pub use seishin2d_render::{
        Camera2D, ClearColor, RenderError, RenderSize, RenderState, Sprite, TextureData, TextureId,
    };
    pub use seishin2d_runtime::{
        run_desktop, run_headless, DesktopGame, DesktopRunConfig, DesktopRuntimeError,
        FixedTimestep, HeadlessRunConfig, WindowConfig, WindowSize,
    };
}

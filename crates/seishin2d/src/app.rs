use std::{error::Error, path::PathBuf};

use seishin2d_assets::{AssetHandle, AssetLoader, AssetPath, AssetRoot};
use seishin2d_audio::{AudioSystem, PlaybackResult, SoundAsset};
use seishin2d_core::{Engine, EngineConfig, EngineResult, Game, UpdateContext};
use seishin2d_input::{InputState, KeyCode};
use seishin2d_render::{Camera2D, ClearColor, RenderState, Sprite, TextureData, TextureId};
use seishin2d_runtime::{run_desktop, DesktopGame, DesktopRunConfig, FixedTimestep, WindowConfig};

pub type GameResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, Clone)]
pub struct App {
    title: String,
    width: u32,
    height: u32,
    target_fps: u32,
    asset_root: PathBuf,
}

impl App {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 1280,
            height: 720,
            target_fps: 60,
            asset_root: PathBuf::from("assets"),
        }
    }

    pub fn window_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn target_fps(mut self, target_fps: u32) -> Self {
        self.target_fps = target_fps;
        self
    }

    pub fn asset_root(mut self, asset_root: impl Into<PathBuf>) -> Self {
        self.asset_root = asset_root.into();
        self
    }

    pub fn run<G: Game2D>(self) -> GameResult<()> {
        let engine = Engine::new(EngineConfig::new(&self.title).with_target_fps(self.target_fps))?;
        let asset_root = AssetRoot::new(&self.asset_root)?;
        let mut startup = StartupContext::new(asset_root);
        let game = G::new(&mut startup)?;
        let adapter = Game2DAdapter::new(game, startup.audio);

        run_desktop(
            engine,
            adapter,
            DesktopRunConfig::new(WindowConfig::new(self.title, self.width, self.height))
                .with_timestep(FixedTimestep::from_fps(self.target_fps)),
        )?;

        Ok(())
    }
}

pub trait Game2D: Sized + 'static {
    fn new(context: &mut StartupContext) -> GameResult<Self>;

    fn update(&mut self, context: &mut FrameContext<'_>) -> GameResult<()>;

    fn render(&self, context: &mut RenderContext);

    fn shutdown(&mut self) -> GameResult<()> {
        Ok(())
    }
}

pub struct StartupContext {
    assets: Assets,
    audio: AudioSystem,
}

impl StartupContext {
    fn new(asset_root: AssetRoot) -> Self {
        Self {
            assets: Assets::new(asset_root),
            audio: AudioSystem::new(),
        }
    }

    pub fn load_texture(&mut self, path: impl AsRef<str>) -> GameResult<Texture> {
        self.assets.load_texture(path)
    }

    pub fn load_sound(&mut self, path: impl AsRef<str>) -> GameResult<AssetHandle<SoundAsset>> {
        let path = AssetPath::new(path.as_ref())?;
        Ok(self.audio.load_sound(self.assets.root(), &path)?)
    }

    pub fn audio_backend_error(&self) -> Option<&str> {
        self.audio.backend_error()
    }
}

pub struct Assets {
    loader: AssetLoader,
    next_texture_id: u64,
}

impl Assets {
    pub fn new(root: AssetRoot) -> Self {
        Self {
            loader: AssetLoader::new(root),
            next_texture_id: 1,
        }
    }

    pub fn root(&self) -> &AssetRoot {
        self.loader.root()
    }

    pub fn load_texture(&mut self, path: impl AsRef<str>) -> GameResult<Texture> {
        let path = AssetPath::new(path.as_ref())?;
        let image = self.loader.load_image(&path)?;
        let texture_id = TextureId::new(self.next_texture_id);
        self.next_texture_id += 1;
        let data = TextureData::rgba8(
            texture_id,
            image.width(),
            image.height(),
            image.pixels_rgba8().to_vec(),
        )?;

        Ok(Texture {
            id: texture_id,
            data,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Texture {
    id: TextureId,
    data: TextureData,
}

impl Texture {
    pub fn id(&self) -> TextureId {
        self.id
    }

    pub fn data(&self) -> &TextureData {
        &self.data
    }
}

pub struct FrameContext<'a> {
    input: &'a InputState,
    audio: &'a mut AudioSystem,
    frame: u64,
    delta_seconds: f32,
}

impl FrameContext<'_> {
    pub fn input(&self) -> &InputState {
        self.input
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }

    pub fn delta_seconds(&self) -> f32 {
        self.delta_seconds
    }

    pub fn axis(&self, negative: KeyCode, positive: KeyCode) -> f32 {
        match (self.input.pressed(negative), self.input.pressed(positive)) {
            (true, false) => -1.0,
            (false, true) => 1.0,
            _ => 0.0,
        }
    }

    pub fn play_sound(&mut self, sound: AssetHandle<SoundAsset>) -> PlaybackResult {
        self.audio.play_sound(sound)
    }
}

#[derive(Debug, Clone)]
pub struct RenderContext {
    clear_color: ClearColor,
    camera: Camera2D,
    textures: Vec<TextureData>,
    sprites: Vec<Sprite>,
}

impl RenderContext {
    fn new() -> Self {
        Self {
            clear_color: ClearColor::BLACK,
            camera: Camera2D::default(),
            textures: Vec::new(),
            sprites: Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.textures.clear();
        self.sprites.clear();
    }

    pub fn clear(&mut self, color: ClearColor) {
        self.clear_color = color;
    }

    pub fn camera(&mut self, camera: Camera2D) {
        self.camera = camera;
    }

    pub fn texture(&mut self, texture: &Texture) {
        self.textures.push(texture.data().clone());
    }

    pub fn sprite(&mut self, sprite: Sprite) {
        self.sprites.push(sprite);
    }

    fn state(&self) -> RenderState<'_> {
        RenderState {
            clear_color: self.clear_color,
            camera: self.camera,
            textures: &self.textures,
            sprites: &self.sprites,
        }
    }
}

struct Game2DAdapter<G> {
    game: G,
    input: InputState,
    audio: AudioSystem,
    render: RenderContext,
}

impl<G: Game2D> Game2DAdapter<G> {
    fn new(game: G, audio: AudioSystem) -> Self {
        Self {
            game,
            input: InputState::default(),
            audio,
            render: RenderContext::new(),
        }
    }
}

impl<G: Game2D> DesktopGame for Game2DAdapter<G> {
    fn input_state(&mut self) -> &mut InputState {
        &mut self.input
    }

    fn render_state(&self) -> RenderState<'_> {
        self.render.state()
    }
}

impl<G: Game2D> Game for Game2DAdapter<G> {
    fn update(&mut self, _engine: &mut Engine, context: UpdateContext) -> EngineResult<()> {
        let mut frame = FrameContext {
            input: &self.input,
            audio: &mut self.audio,
            frame: context.frame,
            delta_seconds: context.delta_seconds,
        };

        self.game
            .update(&mut frame)
            .map_err(|error| seishin2d_core::EngineError::Runtime(error.to_string()))?;

        self.render.reset();
        self.game.render(&mut self.render);

        Ok(())
    }

    fn shutdown(&mut self, _engine: &mut Engine) -> EngineResult<()> {
        self.game
            .shutdown()
            .map_err(|error| seishin2d_core::EngineError::Runtime(error.to_string()))
    }
}

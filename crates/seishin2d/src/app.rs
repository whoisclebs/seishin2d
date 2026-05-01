use std::{
    collections::HashMap,
    error::Error,
    fs,
    path::{Path, PathBuf},
    sync::Once,
};

use seishin2d_assets::{AssetHandle, AssetLoader, AssetPath, AssetRoot};
use seishin2d_audio::{AudioSystem, PlaybackResult, SoundAsset};
use seishin2d_core::{
    Engine, EngineConfig, EngineResult, EntityId, Game, Transform2D, UpdateContext,
};
use seishin2d_input::{InputState, KeyCode};
use seishin2d_render::{Camera2D, ClearColor, RenderState, Sprite, TextureData, TextureId};
use seishin2d_runtime::{run_desktop, DesktopGame, DesktopRunConfig, FixedTimestep, WindowConfig};
use serde::Deserialize;
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;

pub type GameResult<T> = Result<T, Box<dyn Error>>;
pub type Entity = EntityId;

#[derive(Debug, Clone, Deserialize)]
pub struct CharacterData {
    pub id: String,
    pub display_name: String,
    pub sprite: Option<String>,
    pub dialogue: Option<CharacterDialogueData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CharacterDialogueData {
    pub default: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DialogueData {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Resources {
    paths: ProjectPaths,
}

#[derive(Debug, Clone)]
pub struct ResourceToml {
    value: toml::Value,
}

impl ResourceToml {
    pub fn value(&self) -> &toml::Value {
        &self.value
    }

    pub fn get(&self, key: &str) -> Option<&toml::Value> {
        self.value.get(key)
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(toml::Value::as_str)
    }

    pub fn f32(&self, key: &str) -> Option<f32> {
        self.get(key).and_then(|value| match value {
            toml::Value::Float(value) => Some(*value as f32),
            toml::Value::Integer(value) => Some(*value as f32),
            _ => None,
        })
    }

    pub fn bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(toml::Value::as_bool)
    }
}

impl Resources {
    fn new(paths: ProjectPaths) -> Self {
        Self { paths }
    }

    pub fn character(&self, path: impl AsRef<str>) -> GameResult<CharacterData> {
        self.load(path)
    }

    pub fn dialogue(&self, path: impl AsRef<str>) -> GameResult<DialogueData> {
        self.load(path)
    }

    pub fn toml(&self, path: impl AsRef<str>) -> GameResult<ResourceToml> {
        Ok(ResourceToml {
            value: self.load(path)?,
        })
    }

    pub fn load<T>(&self, path: impl AsRef<str>) -> GameResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let path = path.as_ref();
        let resolved = self.paths.resolve_resource(path)?;
        let source = fs::read_to_string(&resolved).map_err(|error| {
            PathDiagnosticError::resource(
                path.to_string(),
                resolved.clone(),
                &self.paths.resource_root,
                error,
            )
        })?;

        toml::from_str(&source).map_err(|error| {
            PathDiagnosticError::resource(
                path.to_string(),
                resolved,
                &self.paths.resource_root,
                error,
            )
            .into()
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct DialogueState {
    active: Option<ActiveDialogue>,
}

impl DialogueState {
    pub fn open(&mut self, speaker: impl Into<String>, dialogue: DialogueData) {
        let active = ActiveDialogue {
            speaker: speaker.into(),
            dialogue,
        };
        info!(speaker = %active.speaker, text = %active.dialogue.text, "dialogue opened");
        self.active = Some(active);
    }

    pub fn close(&mut self) {
        if self.active.is_some() {
            info!("dialogue closed");
        }

        self.active = None;
    }

    pub fn advance_or_close(&mut self) {
        self.close();
    }

    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub fn active(&self) -> Option<&ActiveDialogue> {
        self.active.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct ActiveDialogue {
    pub speaker: String,
    pub dialogue: DialogueData,
}

static LOGGING_INIT: Once = Once::new();

#[derive(Debug, Clone)]
pub struct App {
    title: String,
    width: u32,
    height: u32,
    target_fps: u32,
    asset_root: PathBuf,
    resource_root: PathBuf,
    user_root: PathBuf,
    main_scene: Option<String>,
    clear_color: ClearColor,
    logging: LoggingConfig,
    input_actions: InputActions,
}

impl App {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 1280,
            height: 720,
            target_fps: 60,
            asset_root: PathBuf::from("assets"),
            resource_root: PathBuf::from("resources"),
            user_root: PathBuf::from("user"),
            main_scene: None,
            clear_color: ClearColor::BLACK,
            logging: LoggingConfig::default(),
            input_actions: InputActions::default(),
        }
    }

    pub fn from_project(path: impl AsRef<Path>) -> GameResult<Self> {
        let path = fs::canonicalize(path.as_ref())?;
        let project = ProjectConfig::from_path(&path)?;
        let project_dir = path.parent().unwrap_or_else(|| Path::new("."));

        Ok(Self::from_project_config(project, project_dir))
    }

    pub fn discover_project() -> GameResult<Self> {
        let project_path = discover_project_file()?;
        Self::from_project(project_path)
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

    pub fn resource_root(mut self, resource_root: impl Into<PathBuf>) -> Self {
        self.resource_root = resource_root.into();
        self
    }

    pub fn clear_color(mut self, clear_color: ClearColor) -> Self {
        self.clear_color = clear_color;
        self
    }

    pub fn with_default_logging(mut self) -> Self {
        self.logging.enabled = true;
        self
    }

    pub fn log_level(mut self, level: LogLevel) -> Self {
        self.logging.enabled = true;
        self.logging.default_filter = level.as_filter().to_string();
        self
    }

    pub fn run<G: Game2D>(self) -> GameResult<()> {
        self.logging.install();

        let paths = ProjectPaths::new(self.asset_root, self.resource_root, self.user_root);
        let _user_root = paths.user_root();
        let engine = Engine::new(EngineConfig::new(&self.title).with_target_fps(self.target_fps))?;
        if let Some(main_scene) = self.main_scene.as_deref() {
            validate_main_scene(main_scene, &paths)?;
        }

        let asset_root = AssetRoot::new(&paths.asset_root)?;
        let mut startup = StartupContext::new(
            asset_root,
            self.input_actions,
            self.clear_color,
            paths,
            self.main_scene,
        );

        if let Some(error) = startup.audio_backend_error() {
            warn!(%error, "audio unavailable, game will continue silently");
        }

        let game = G::new(&mut startup)?;
        startup.load_main_scene()?;
        let runtime_parts = startup.into_runtime_parts();
        let adapter = Game2DAdapter::new(game, runtime_parts);

        run_desktop(
            engine,
            adapter,
            DesktopRunConfig::new(WindowConfig::new(self.title, self.width, self.height))
                .with_timestep(FixedTimestep::from_fps(self.target_fps)),
        )?;

        Ok(())
    }

    fn from_project_config(project: ProjectConfig, project_dir: &Path) -> Self {
        let game = project.game.unwrap_or_default();
        let window = project.window.unwrap_or_default();
        let assets = project.assets.unwrap_or_default();
        let resources = project.resources.unwrap_or_default();
        let user = project.user.unwrap_or_default();
        let logging = project.logging.unwrap_or_default();
        let input_actions = project
            .input
            .map(InputActions::from_config)
            .unwrap_or_default();

        let asset_root = project_dir.join(assets.root.unwrap_or_else(|| "assets".to_string()));
        let resource_root =
            project_dir.join(resources.root.unwrap_or_else(|| "resources".to_string()));
        let user_root = project_dir.join(user.root.unwrap_or_else(|| "user".to_string()));

        Self {
            title: game.name.unwrap_or_else(|| "seishin2d".to_string()),
            width: window.width.unwrap_or(1280),
            height: window.height.unwrap_or(720),
            target_fps: window.target_fps.unwrap_or(60),
            asset_root,
            resource_root,
            user_root,
            main_scene: game.main_scene,
            clear_color: window
                .clear_color
                .as_deref()
                .and_then(parse_clear_color)
                .unwrap_or(ClearColor::BLACK),
            logging: LoggingConfig {
                enabled: true,
                default_filter: logging.default_filter.unwrap_or_else(|| "info".to_string()),
            },
            input_actions,
        }
    }
}

pub fn run<G: Game2D>() -> GameResult<()> {
    App::discover_project()?.run::<G>()
}

#[derive(Debug, Clone)]
struct LoggingConfig {
    enabled: bool,
    default_filter: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_filter: "info".to_string(),
        }
    }
}

impl LoggingConfig {
    fn install(&self) {
        if !self.enabled {
            return;
        }

        let default_filter = self.default_filter.clone();

        LOGGING_INIT.call_once(move || {
            let env_filter = EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(default_filter));
            let _ = tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .try_init();
        });
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    fn as_filter(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }
}

pub trait Game2D: Sized + 'static {
    fn new(context: &mut StartupContext) -> GameResult<Self>;

    fn update(&mut self, _context: &mut FrameContext<'_>) -> GameResult<()> {
        Ok(())
    }

    fn render(&self, _context: &mut RenderContext) {}

    fn shutdown(&mut self) -> GameResult<()> {
        Ok(())
    }
}

pub struct StartupContext {
    assets: Assets,
    audio: AudioSystem,
    world: World,
    components: ComponentRegistry,
    component_instances: Vec<RuntimeComponent>,
    paths: ProjectPaths,
    main_scene: Option<String>,
    main_scene_loaded: bool,
    input_actions: InputActions,
    clear_color: ClearColor,
}

impl StartupContext {
    fn new(
        asset_root: AssetRoot,
        input_actions: InputActions,
        clear_color: ClearColor,
        paths: ProjectPaths,
        main_scene: Option<String>,
    ) -> Self {
        Self {
            assets: Assets::new(asset_root),
            audio: AudioSystem::new(),
            world: World::default(),
            components: ComponentRegistry::default(),
            component_instances: Vec::new(),
            paths,
            main_scene,
            main_scene_loaded: false,
            input_actions,
            clear_color,
        }
    }

    pub fn assets(&mut self) -> &mut Assets {
        &mut self.assets
    }

    pub fn world(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn components(&mut self) -> &mut ComponentRegistry {
        &mut self.components
    }

    pub fn load_main_scene(&mut self) -> GameResult<()> {
        if self.main_scene_loaded {
            return Ok(());
        }

        let Some(main_scene) = self.main_scene.clone() else {
            return Ok(());
        };

        load_main_scene(&main_scene, self)?;
        self.main_scene_loaded = true;

        Ok(())
    }

    pub fn spawn(&mut self, bundle: SpriteBundle) -> GameResult<Entity> {
        Ok(self.world.spawn_sprite(bundle))
    }

    pub fn sprite(&mut self, texture_path: impl Into<String>) -> SpriteBuilder<'_> {
        SpriteBuilder::new(self, texture_path.into())
    }

    pub fn load_texture(&mut self, path: impl AsRef<str>) -> GameResult<Texture> {
        self.assets.load_texture(path)
    }

    pub fn load_sound(&mut self, path: impl AsRef<str>) -> GameResult<AssetHandle<SoundAsset>> {
        self.assets.sound(&mut self.audio, path)
    }

    pub fn sound(&mut self, path: impl AsRef<str>) -> GameResult<AssetHandle<SoundAsset>> {
        self.load_sound(path)
    }

    pub fn audio_backend_error(&self) -> Option<&str> {
        self.audio.backend_error()
    }

    fn into_runtime_parts(self) -> RuntimeParts {
        RuntimeParts {
            audio: self.audio,
            world: self.world,
            input_actions: self.input_actions,
            resources: Resources::new(self.paths),
            dialogue: DialogueState::default(),
            component_instances: self.component_instances,
            clear_color: self.clear_color,
        }
    }
}

struct RuntimeParts {
    audio: AudioSystem,
    world: World,
    input_actions: InputActions,
    resources: Resources,
    dialogue: DialogueState,
    component_instances: Vec<RuntimeComponent>,
    clear_color: ClearColor,
}

pub trait Component2D {
    fn update(&mut self, entity: Entity, context: &mut FrameContext<'_>) -> GameResult<()>;
}

type ComponentFactory = fn(&toml::Value) -> GameResult<Box<dyn Component2D>>;

pub struct RuntimeComponent {
    entity: Entity,
    component: Box<dyn Component2D>,
}

#[derive(Clone, Default)]
pub struct ComponentRegistry {
    factories: HashMap<String, ComponentFactory>,
}

impl ComponentRegistry {
    pub fn register<T: Component2D + Default + 'static>(
        &mut self,
        name: impl Into<String>,
    ) -> GameResult<()> {
        let name = name.into();

        if name.trim().is_empty() {
            return Err("component registration name must not be empty".into());
        }

        self.factories.insert(name, |_| Ok(Box::<T>::default()));
        Ok(())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.factories.contains_key(name)
    }

    fn instantiate(&self, component: &CustomComponentRef) -> GameResult<Box<dyn Component2D>> {
        let Some(factory) = self.factories.get(&component.type_name) else {
            return Err(format!("unknown component type '{}'", component.type_name).into());
        };

        factory(&component.config)
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

    pub fn texture(&mut self, path: impl AsRef<str>) -> GameResult<Texture> {
        self.load_texture(path)
    }

    pub fn sound(
        &self,
        audio: &mut AudioSystem,
        path: impl AsRef<str>,
    ) -> GameResult<AssetHandle<SoundAsset>> {
        let requested = path.as_ref().to_string();
        let virtual_path = VirtualPath::parse(&requested)?;
        ensure_asset_scheme(&virtual_path)?;
        let path = AssetPath::new(virtual_path.relative_path())?;
        let resolved = self.loader.root().resolve(&path);

        Ok(audio.load_sound(self.root(), &path).map_err(|error| {
            PathDiagnosticError::asset(requested, resolved, self.loader.root().path(), error)
        })?)
    }

    pub fn load_texture(&mut self, path: impl AsRef<str>) -> GameResult<Texture> {
        let requested = path.as_ref().to_string();
        let virtual_path = VirtualPath::parse(&requested)?;
        ensure_asset_scheme(&virtual_path)?;
        let path = AssetPath::new(virtual_path.relative_path())?;
        let image = self.loader.load_image(&path).map_err(|error| {
            let resolved = self.loader.root().resolve(&path);
            PathDiagnosticError::asset(
                requested.clone(),
                resolved,
                self.loader.root().path(),
                error,
            )
        })?;
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

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub const fn splat(value: f32) -> Self {
        Self { x: value, y: value }
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpriteRenderer {
    texture: Texture,
    size: Vec2,
}

impl SpriteRenderer {
    pub fn new(texture: Texture, size: Vec2) -> Self {
        Self { texture, size }
    }
}

#[derive(Debug, Clone)]
pub struct SpriteBundle {
    pub texture: Texture,
    pub transform: Transform2D,
    pub size: Vec2,
}

impl SpriteBundle {
    pub fn new(texture: Texture) -> Self {
        Self {
            texture,
            transform: Transform2D::default(),
            size: Vec2::splat(32.0),
        }
    }
}

#[derive(Debug, Clone)]
struct EntityRecord {
    name: Option<String>,
    tags: Vec<String>,
    data_refs: HashMap<String, String>,
    custom_components: Vec<CustomComponentRef>,
    transform: Transform2D,
    renderer: Option<SpriteRenderer>,
}

#[derive(Debug, Clone)]
struct CustomComponentRef {
    type_name: String,
    config: toml::Value,
}

#[derive(Debug, Clone)]
struct SceneEntityRuntime {
    name: Option<String>,
    tags: Vec<String>,
    data_refs: HashMap<String, String>,
    custom_components: Vec<CustomComponentRef>,
    transform: Transform2D,
    sprite: Option<SpriteRenderer>,
}

#[derive(Debug, Clone)]
pub struct World {
    next_entity: u64,
    entities: HashMap<Entity, EntityRecord>,
}

impl Default for World {
    fn default() -> Self {
        Self {
            next_entity: 1,
            entities: HashMap::new(),
        }
    }
}

impl World {
    pub fn spawn_sprite(&mut self, bundle: SpriteBundle) -> Entity {
        self.spawn_scene_entity(SceneEntityRuntime {
            name: None,
            tags: Vec::new(),
            data_refs: HashMap::new(),
            custom_components: Vec::new(),
            transform: bundle.transform,
            sprite: Some(SpriteRenderer::new(bundle.texture, bundle.size)),
        })
    }

    fn spawn_scene_entity(&mut self, runtime: SceneEntityRuntime) -> Entity {
        let entity = EntityId::new(self.next_entity);
        self.next_entity += 1;

        self.entities.insert(
            entity,
            EntityRecord {
                name: runtime.name,
                tags: runtime.tags,
                data_refs: runtime.data_refs,
                custom_components: runtime.custom_components,
                transform: runtime.transform,
                renderer: runtime.sprite,
            },
        );

        entity
    }

    pub fn first_with_tag(&self, tag: &str) -> Option<Entity> {
        self.entities_with_tag(tag).into_iter().next()
    }

    pub fn first_interactable(&self) -> Option<Entity> {
        self.first_with_tag("interactable")
    }

    pub fn entities_with_tag(&self, tag: &str) -> Vec<Entity> {
        let mut entities = self
            .entities
            .iter()
            .filter_map(|(entity, record)| {
                record
                    .tags
                    .iter()
                    .any(|value| value == tag)
                    .then_some(*entity)
            })
            .collect::<Vec<_>>();
        entities.sort();
        entities
    }

    pub fn entity_by_name(&self, name: &str) -> Option<Entity> {
        self.entities.iter().find_map(|(entity, record)| {
            record
                .name
                .as_deref()
                .is_some_and(|value| value == name)
                .then_some(*entity)
        })
    }

    pub fn tags(&self, entity: Entity) -> Option<&[String]> {
        self.entities
            .get(&entity)
            .map(|record| record.tags.as_slice())
    }

    pub fn transform(&self, entity: Entity) -> Option<Transform2D> {
        self.entities.get(&entity).map(|record| record.transform)
    }

    pub fn name(&self, entity: Entity) -> Option<&str> {
        self.entities
            .get(&entity)
            .and_then(|record| record.name.as_deref())
    }

    pub fn data_ref(&self, entity: Entity, key: &str) -> Option<&str> {
        self.entities
            .get(&entity)
            .and_then(|record| record.data_refs.get(key))
            .map(String::as_str)
    }

    pub fn has_custom_component(&self, entity: Entity, type_name: &str) -> bool {
        self.entities.get(&entity).is_some_and(|record| {
            record
                .custom_components
                .iter()
                .any(|component| component.type_name == type_name)
        })
    }

    pub fn custom_component_config(&self, entity: Entity, type_name: &str) -> Option<&toml::Value> {
        self.entities.get(&entity).and_then(|record| {
            record
                .custom_components
                .iter()
                .find(|component| component.type_name == type_name)
                .map(|component| &component.config)
        })
    }

    pub fn has_component<T: Component2D + 'static>(&self, _entity: Entity) -> bool {
        false
    }

    pub fn translate(&mut self, entity: Entity, delta: Vec2) {
        if let Some(record) = self.entities.get_mut(&entity) {
            record.transform = record.transform.translated(delta.x, delta.y);
        }
    }

    pub fn set_position(&mut self, entity: Entity, x: f32, y: f32) {
        if let Some(record) = self.entities.get_mut(&entity) {
            record.transform.x = x;
            record.transform.y = y;
        }
    }

    pub fn entity(&mut self, entity: Entity) -> EntityMut<'_> {
        EntityMut {
            world: self,
            entity,
        }
    }

    fn render_into(&self, render: &mut RenderContext) {
        for record in self.entities.values() {
            let Some(renderer) = &record.renderer else {
                continue;
            };

            render.texture(&renderer.texture);
            render.sprite(Sprite::new(
                renderer.texture.id(),
                record.transform,
                renderer.size.x,
                renderer.size.y,
            ));
        }
    }
}

pub struct EntityMut<'a> {
    world: &'a mut World,
    entity: Entity,
}

impl EntityMut<'_> {
    pub fn translate(&mut self, delta: Vec2) -> &mut Self {
        self.world.translate(self.entity, delta);
        self
    }

    pub fn set_position(&mut self, x: f32, y: f32) -> &mut Self {
        self.world.set_position(self.entity, x, y);
        self
    }
}

pub struct SpriteBuilder<'a> {
    context: &'a mut StartupContext,
    texture_path: String,
    transform: Transform2D,
    size: Vec2,
}

impl<'a> SpriteBuilder<'a> {
    fn new(context: &'a mut StartupContext, texture_path: String) -> Self {
        Self {
            context,
            texture_path,
            transform: Transform2D::default(),
            size: Vec2::splat(32.0),
        }
    }

    pub fn position(mut self, x: f32, y: f32) -> Self {
        self.transform.x = x;
        self.transform.y = y;
        self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.size = Vec2::new(width, height);
        self
    }

    pub fn spawn(self) -> GameResult<Entity> {
        let texture = self.context.assets.texture(&self.texture_path)?;
        self.context.spawn(SpriteBundle {
            texture,
            transform: self.transform,
            size: self.size,
        })
    }
}

pub struct FrameContext<'a> {
    input: &'a InputState,
    input_actions: &'a InputActions,
    audio: &'a mut AudioSystem,
    world: &'a mut World,
    resources: &'a Resources,
    dialogue: &'a mut DialogueState,
    frame: u64,
    delta_seconds: f32,
}

impl FrameContext<'_> {
    pub fn input(&self) -> GameplayInput<'_> {
        GameplayInput {
            state: self.input,
            actions: self.input_actions,
        }
    }

    pub fn world(&mut self) -> &mut World {
        self.world
    }

    pub fn resources(&self) -> &Resources {
        self.resources
    }

    pub fn dialogue(&mut self) -> &mut DialogueState {
        self.dialogue
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }

    pub fn delta_seconds(&self) -> f32 {
        self.delta_seconds
    }

    pub fn axis(&self, negative: KeyCode, positive: KeyCode) -> f32 {
        axis(self.input, negative, positive)
    }

    pub fn play_sound(&mut self, sound: AssetHandle<SoundAsset>) -> PlaybackResult {
        self.audio.play_sound(sound)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GameplayInput<'a> {
    state: &'a InputState,
    actions: &'a InputActions,
}

impl GameplayInput<'_> {
    pub fn pressed<Q: InputQuery>(&self, query: Q) -> bool {
        query.pressed(self.state, self.actions)
    }

    pub fn just_pressed<Q: InputQuery>(&self, query: Q) -> bool {
        query.just_pressed(self.state, self.actions)
    }

    pub fn just_released<Q: InputQuery>(&self, query: Q) -> bool {
        query.just_released(self.state, self.actions)
    }

    pub fn axis2d(&self, name: &str) -> Vec2 {
        self.actions.axis2d(self.state, name)
    }
}

pub trait InputQuery {
    fn pressed(self, state: &InputState, actions: &InputActions) -> bool;
    fn just_pressed(self, state: &InputState, actions: &InputActions) -> bool;
    fn just_released(self, state: &InputState, actions: &InputActions) -> bool;
}

impl InputQuery for KeyCode {
    fn pressed(self, state: &InputState, _actions: &InputActions) -> bool {
        state.pressed(self)
    }

    fn just_pressed(self, state: &InputState, _actions: &InputActions) -> bool {
        state.just_pressed(self)
    }

    fn just_released(self, state: &InputState, _actions: &InputActions) -> bool {
        state.just_released(self)
    }
}

impl InputQuery for &str {
    fn pressed(self, state: &InputState, actions: &InputActions) -> bool {
        actions.button_pressed(state, self)
    }

    fn just_pressed(self, state: &InputState, actions: &InputActions) -> bool {
        actions.button_just_pressed(state, self)
    }

    fn just_released(self, state: &InputState, actions: &InputActions) -> bool {
        actions.button_just_released(state, self)
    }
}

#[derive(Debug, Clone, Default)]
pub struct InputActions {
    axis2d: HashMap<String, Axis2dAction>,
    buttons: HashMap<String, ButtonAction>,
}

impl InputActions {
    fn from_config(config: InputConfig) -> Self {
        let mut axis2d = HashMap::new();
        let mut buttons = HashMap::new();

        for (name, action) in config.actions {
            match action {
                InputActionConfig::Axis2d {
                    left,
                    right,
                    up,
                    down,
                } => {
                    axis2d.insert(
                        name,
                        Axis2dAction {
                            left: parse_key_list(left),
                            right: parse_key_list(right),
                            up: parse_key_list(up),
                            down: parse_key_list(down),
                        },
                    );
                }
                InputActionConfig::Button { keys } => {
                    buttons.insert(
                        name,
                        ButtonAction {
                            keys: parse_key_list(keys),
                        },
                    );
                }
            }
        }

        Self { axis2d, buttons }
    }

    fn axis2d(&self, input: &InputState, name: &str) -> Vec2 {
        let Some(action) = self.axis2d.get(name) else {
            debug!(action = name, "axis2d input action not found or not axis2d");
            return Vec2::ZERO;
        };

        Vec2::new(
            action_axis(input, &action.left, &action.right),
            action_axis(input, &action.up, &action.down),
        )
    }

    fn button_pressed(&self, input: &InputState, name: &str) -> bool {
        self.button(name)
            .is_some_and(|action| action.keys.iter().any(|key| input.pressed(*key)))
    }

    fn button_just_pressed(&self, input: &InputState, name: &str) -> bool {
        self.button(name)
            .is_some_and(|action| action.keys.iter().any(|key| input.just_pressed(*key)))
    }

    fn button_just_released(&self, input: &InputState, name: &str) -> bool {
        self.button(name)
            .is_some_and(|action| action.keys.iter().any(|key| input.just_released(*key)))
    }

    fn button(&self, name: &str) -> Option<&ButtonAction> {
        let action = self.buttons.get(name);

        if action.is_none() {
            debug!(action = name, "button input action not found or not button");
        }

        action
    }
}

#[derive(Debug, Clone, Default)]
struct Axis2dAction {
    left: Vec<KeyCode>,
    right: Vec<KeyCode>,
    up: Vec<KeyCode>,
    down: Vec<KeyCode>,
}

#[derive(Debug, Clone, Default)]
struct ButtonAction {
    keys: Vec<KeyCode>,
}

#[derive(Debug, Clone)]
pub struct RenderContext {
    clear_color: ClearColor,
    camera: Camera2D,
    textures: Vec<TextureData>,
    sprites: Vec<Sprite>,
}

impl RenderContext {
    fn new(clear_color: ClearColor) -> Self {
        Self {
            clear_color,
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
    input_actions: InputActions,
    audio: AudioSystem,
    world: World,
    resources: Resources,
    dialogue: DialogueState,
    component_instances: Vec<RuntimeComponent>,
    render: RenderContext,
}

impl<G: Game2D> Game2DAdapter<G> {
    fn new(game: G, runtime_parts: RuntimeParts) -> Self {
        let mut render = RenderContext::new(runtime_parts.clear_color);
        runtime_parts.world.render_into(&mut render);

        Self {
            game,
            input: InputState::default(),
            input_actions: runtime_parts.input_actions,
            audio: runtime_parts.audio,
            world: runtime_parts.world,
            resources: runtime_parts.resources,
            dialogue: runtime_parts.dialogue,
            component_instances: runtime_parts.component_instances,
            render,
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
            input_actions: &self.input_actions,
            audio: &mut self.audio,
            world: &mut self.world,
            resources: &self.resources,
            dialogue: &mut self.dialogue,
            frame: context.frame,
            delta_seconds: context.delta_seconds,
        };

        update_builtin_dialogue_interaction(&mut frame)
            .map_err(|error| seishin2d_core::EngineError::Runtime(error.to_string()))?;

        for runtime_component in &mut self.component_instances {
            runtime_component
                .component
                .update(runtime_component.entity, &mut frame)
                .map_err(|error| seishin2d_core::EngineError::Runtime(error.to_string()))?;
        }

        self.game
            .update(&mut frame)
            .map_err(|error| seishin2d_core::EngineError::Runtime(error.to_string()))?;

        self.render.reset();
        self.world.render_into(&mut self.render);
        self.game.render(&mut self.render);

        Ok(())
    }

    fn shutdown(&mut self, _engine: &mut Engine) -> EngineResult<()> {
        self.game
            .shutdown()
            .map_err(|error| seishin2d_core::EngineError::Runtime(error.to_string()))
    }
}

fn update_builtin_dialogue_interaction(context: &mut FrameContext<'_>) -> GameResult<()> {
    if !context.input().just_pressed("interact") {
        return Ok(());
    }

    if context.dialogue().is_active() {
        context.dialogue().advance_or_close();
        return Ok(());
    }

    let character_ref = {
        let world = context.world();
        world
            .first_interactable()
            .and_then(|entity| world.data_ref(entity, "character"))
            .map(ToOwned::to_owned)
    };

    let Some(character_ref) = character_ref else {
        debug!("interact pressed but no interactable dialogue target was found");
        return Ok(());
    };

    let character = context.resources().character(&character_ref)?;
    let Some(dialogue_ref) = character
        .dialogue
        .as_ref()
        .map(|dialogue| &dialogue.default)
    else {
        debug!(character = %character.id, "interactable character has no default dialogue");
        return Ok(());
    };
    let dialogue = context.resources().dialogue(dialogue_ref)?;

    context.dialogue().open(character.display_name, dialogue);

    Ok(())
}

#[derive(Debug, Deserialize)]
struct ProjectConfig {
    game: Option<GameConfig>,
    window: Option<WindowProjectConfig>,
    resources: Option<ResourcesConfig>,
    assets: Option<AssetsConfig>,
    user: Option<UserConfig>,
    logging: Option<LoggingProjectConfig>,
    input: Option<InputConfig>,
}

impl ProjectConfig {
    fn from_path(path: &Path) -> GameResult<Self> {
        let source = fs::read_to_string(path)?;
        Ok(toml::from_str(&source)?)
    }
}

#[derive(Debug, Default, Deserialize)]
struct GameConfig {
    name: Option<String>,
    main_scene: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct WindowProjectConfig {
    width: Option<u32>,
    height: Option<u32>,
    target_fps: Option<u32>,
    clear_color: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct AssetsConfig {
    root: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ResourcesConfig {
    root: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct UserConfig {
    root: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct LoggingProjectConfig {
    default_filter: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct InputConfig {
    actions: HashMap<String, InputActionConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum InputActionConfig {
    #[serde(rename = "axis2d")]
    Axis2d {
        #[serde(default)]
        left: Vec<String>,
        #[serde(default)]
        right: Vec<String>,
        #[serde(default)]
        up: Vec<String>,
        #[serde(default)]
        down: Vec<String>,
    },
    #[serde(rename = "button")]
    Button {
        #[serde(default)]
        keys: Vec<String>,
    },
}

#[derive(Debug, Default, Deserialize)]
struct SceneConfig {
    #[serde(default)]
    entities: Vec<SceneEntityConfig>,
}

#[derive(Debug, Default, Deserialize)]
struct SceneEntityConfig {
    name: Option<String>,
    prefab: Option<String>,
    transform: Option<SceneTransformConfig>,
    tags: Option<TagsConfig>,
    data: Option<HashMap<String, String>>,
    sprite: Option<SceneSpriteConfig>,
    #[serde(default)]
    components: Vec<CustomComponentConfig>,
}

#[derive(Debug, Default, Deserialize)]
struct PrefabConfig {
    #[serde(default)]
    components: HashMap<String, toml::Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TagsConfig {
    #[serde(default)]
    values: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
struct SceneTransformConfig {
    x: Option<f32>,
    y: Option<f32>,
    rotation_radians: Option<f32>,
    scale_x: Option<f32>,
    scale_y: Option<f32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct SceneSpriteConfig {
    texture: Option<String>,
    width: Option<f32>,
    height: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
struct CustomComponentConfig {
    #[serde(rename = "type")]
    type_name: String,
    #[serde(flatten)]
    config: HashMap<String, toml::Value>,
}

#[derive(Debug)]
struct PathDiagnosticError {
    kind: PathDiagnosticKind,
    requested: String,
    resolved: PathBuf,
    root: PathBuf,
    source: Box<dyn Error + Send + Sync>,
}

#[derive(Debug, Clone, Copy)]
enum PathDiagnosticKind {
    Asset,
    Resource,
}

impl PathDiagnosticError {
    fn asset(
        requested: String,
        resolved: PathBuf,
        root: &Path,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            kind: PathDiagnosticKind::Asset,
            requested,
            resolved,
            root: root.to_path_buf(),
            source: Box::new(source),
        }
    }

    fn resource(
        requested: String,
        resolved: PathBuf,
        root: &Path,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            kind: PathDiagnosticKind::Resource,
            requested,
            resolved,
            root: root.to_path_buf(),
            source: Box::new(source),
        }
    }
}

impl std::fmt::Display for PathDiagnosticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (label, root_label, scheme_hint, other_scheme_hint) = match self.kind {
            PathDiagnosticKind::Asset => (
                "Asset",
                "Configured asset root",
                "Use asset:// for images, audio, video, and fonts.",
                "Use res:// for resources/configuration/scene files.",
            ),
            PathDiagnosticKind::Resource => (
                "Resource",
                "Configured resource root",
                "Use res:// for resources/configuration/scene files.",
                "Use asset:// for images, audio, video, and fonts.",
            ),
        };

        write!(
            f,
            "{label} not found or could not be loaded: {}\n\nResolved path:\n  {}\n\n{root_label}:\n  {}\n\nSuggestions:\n  - Check if the file exists.\n  - Check Seishin.toml root configuration.\n  - {scheme_hint}\n  - {other_scheme_hint}\n\nCause: {}",
            self.requested,
            self.resolved.display(),
            self.root.display(),
            self.source
        )
    }
}

impl Error for PathDiagnosticError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

#[derive(Debug, Clone)]
struct ProjectPaths {
    asset_root: PathBuf,
    resource_root: PathBuf,
    user_root: PathBuf,
}

impl ProjectPaths {
    fn new(asset_root: PathBuf, resource_root: PathBuf, user_root: PathBuf) -> Self {
        Self {
            asset_root,
            resource_root,
            user_root,
        }
    }

    fn resolve_resource(&self, requested: &str) -> GameResult<PathBuf> {
        let virtual_path = VirtualPath::parse(requested)?;
        ensure_resource_scheme(&virtual_path)?;
        let asset_path = AssetPath::new(virtual_path.relative_path())?;
        Ok(self.resource_root.join(asset_path.as_path()))
    }

    fn user_root(&self) -> &Path {
        &self.user_root
    }

    #[cfg(test)]
    fn resolve_asset(&self, requested: &str) -> GameResult<PathBuf> {
        let virtual_path = VirtualPath::parse(requested)?;
        ensure_asset_scheme(&virtual_path)?;
        let asset_path = AssetPath::new(virtual_path.relative_path())?;
        Ok(self.asset_root.join(asset_path.as_path()))
    }

    #[cfg(test)]
    fn resolve_user(&self, requested: &str) -> GameResult<PathBuf> {
        let virtual_path = VirtualPath::parse(requested)?;

        if virtual_path.scheme != VirtualScheme::User {
            return Err(format!(
                "user data paths must use user:// so they resolve under the user data root: {}",
                virtual_path.requested
            )
            .into());
        }

        let asset_path = AssetPath::new(virtual_path.relative_path())?;
        Ok(self.user_root.join(asset_path.as_path()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VirtualScheme {
    Asset,
    Resource,
    User,
    Relative,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VirtualPath<'a> {
    scheme: VirtualScheme,
    relative_path: &'a str,
    requested: &'a str,
}

impl<'a> VirtualPath<'a> {
    fn parse(requested: &'a str) -> GameResult<Self> {
        if let Some(relative_path) = requested.strip_prefix("asset://") {
            return Ok(Self {
                scheme: VirtualScheme::Asset,
                relative_path,
                requested,
            });
        }

        if let Some(relative_path) = requested.strip_prefix("res://") {
            return Ok(Self {
                scheme: VirtualScheme::Resource,
                relative_path,
                requested,
            });
        }

        if let Some(relative_path) = requested.strip_prefix("user://") {
            return Ok(Self {
                scheme: VirtualScheme::User,
                relative_path,
                requested,
            });
        }

        if requested.contains("://") {
            return Err(format!(
                "unsupported virtual path scheme in {requested}; expected asset://, res://, or user://"
            )
            .into());
        }

        Ok(Self {
            scheme: VirtualScheme::Relative,
            relative_path: requested,
            requested,
        })
    }

    fn relative_path(&self) -> &str {
        self.relative_path
    }
}

fn ensure_asset_scheme(path: &VirtualPath<'_>) -> GameResult<()> {
    match path.scheme {
        VirtualScheme::Asset | VirtualScheme::Relative => Ok(()),
        VirtualScheme::Resource => Err(format!(
            "possible wrong scheme: you used {}. Sprites, audio, video, and fonts are assets. Try asset://{}",
            path.requested,
            path.relative_path()
        )
        .into()),
        VirtualScheme::User => Err(format!(
            "user:// paths are reserved for writable user data and cannot be loaded as assets: {}",
            path.requested
        )
        .into()),
    }
}

fn ensure_resource_scheme(path: &VirtualPath<'_>) -> GameResult<()> {
    match path.scheme {
        VirtualScheme::Resource => Ok(()),
        VirtualScheme::Asset => Err(format!(
            "possible wrong scheme: you used {}. Configuration, scenes, prefabs, and data files are resources. Try res://{}",
            path.requested,
            path.relative_path()
        )
        .into()),
        VirtualScheme::User => Err(format!(
            "user:// paths are reserved for writable user data and cannot be loaded as resources: {}",
            path.requested
        )
        .into()),
        VirtualScheme::Relative => Err(format!(
            "resource paths must use res:// so they resolve under [resources].root: {}",
            path.requested
        )
        .into()),
    }
}

fn discover_project_file() -> GameResult<PathBuf> {
    let current_dir = std::env::current_dir()?;

    for directory in current_dir.ancestors() {
        let candidate = directory.join("Seishin.toml");

        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    let mut candidates = Vec::new();
    let examples_dir = current_dir.join("examples");

    if let Ok(entries) = fs::read_dir(examples_dir) {
        for entry in entries.flatten() {
            let candidate = entry.path().join("Seishin.toml");

            if candidate.is_file() {
                candidates.push(candidate);
            }
        }
    }

    match candidates.as_slice() {
        [project] => Ok(project.clone()),
        [] => Err("Seishin.toml not found. Expected a Seishin.toml file in the current directory, a parent directory, or exactly one examples/* project.".into()),
        _ => Err("multiple Seishin.toml files found; use App::from_project(path)".into()),
    }
}

fn validate_main_scene(main_scene: &str, paths: &ProjectPaths) -> GameResult<()> {
    let resolved = paths.resolve_resource(main_scene)?;

    if !resolved.is_file() {
        return Err(PathDiagnosticError::resource(
            main_scene.to_string(),
            resolved,
            &paths.resource_root,
            std::io::Error::new(std::io::ErrorKind::NotFound, "main scene file not found"),
        )
        .into());
    }

    Ok(())
}

fn load_main_scene(main_scene: &str, startup: &mut StartupContext) -> GameResult<()> {
    let scene = load_scene_config(main_scene, &startup.paths)?;

    for entity in scene.entities {
        let runtime = build_scene_entity(entity, startup)?;
        let custom_components = runtime.custom_components.clone();
        let entity = startup.world.spawn_scene_entity(runtime);

        for component in custom_components {
            let instance = startup.components.instantiate(&component)?;
            startup.component_instances.push(RuntimeComponent {
                entity,
                component: instance,
            });
        }
    }

    Ok(())
}

fn load_scene_config(path: &str, paths: &ProjectPaths) -> GameResult<SceneConfig> {
    let resolved = paths.resolve_resource(path)?;
    let source = fs::read_to_string(&resolved).map_err(|error| {
        PathDiagnosticError::resource(
            path.to_string(),
            resolved.clone(),
            &paths.resource_root,
            error,
        )
    })?;

    toml::from_str(&source).map_err(|error| {
        PathDiagnosticError::resource(path.to_string(), resolved, &paths.resource_root, error)
            .into()
    })
}

fn load_prefab_config(path: &str, paths: &ProjectPaths) -> GameResult<PrefabConfig> {
    let resolved = paths.resolve_resource(path)?;
    let source = fs::read_to_string(&resolved).map_err(|error| {
        PathDiagnosticError::resource(
            path.to_string(),
            resolved.clone(),
            &paths.resource_root,
            error,
        )
    })?;

    toml::from_str(&source).map_err(|error| {
        PathDiagnosticError::resource(path.to_string(), resolved, &paths.resource_root, error)
            .into()
    })
}

fn build_scene_entity(
    entity: SceneEntityConfig,
    startup: &mut StartupContext,
) -> GameResult<SceneEntityRuntime> {
    let mut blueprint = match entity.prefab.as_deref() {
        Some(prefab_path) => {
            EntityBlueprint::from_prefab(load_prefab_config(prefab_path, &startup.paths)?)
        }
        None => EntityBlueprint::default(),
    };

    blueprint.apply_scene(entity);
    blueprint.validate_custom_components(&startup.components)?;
    blueprint.validate_data_refs(&startup.paths)?;
    blueprint.into_runtime(&mut startup.assets)
}

#[derive(Debug, Default)]
struct EntityBlueprint {
    name: Option<String>,
    tags: Option<Vec<String>>,
    data_refs: HashMap<String, String>,
    transform: Transform2D,
    sprite: Option<SceneSpriteConfig>,
    custom_components: Vec<CustomComponentConfig>,
}

impl EntityBlueprint {
    fn from_prefab(prefab: PrefabConfig) -> Self {
        let mut blueprint = Self::default();

        for (name, value) in prefab.components {
            match name.as_str() {
                "name" => {
                    blueprint.name = value
                        .get("value")
                        .and_then(toml::Value::as_str)
                        .map(ToOwned::to_owned);
                }
                "tags" => {
                    blueprint.tags =
                        value
                            .get("values")
                            .and_then(toml::Value::as_array)
                            .map(|values| {
                                values
                                    .iter()
                                    .filter_map(toml::Value::as_str)
                                    .map(ToOwned::to_owned)
                                    .collect()
                            });
                }
                "transform" => {
                    if let Ok(transform) = value.try_into() {
                        blueprint.transform = merge_transform(blueprint.transform, transform);
                    }
                }
                "sprite" => {
                    blueprint.sprite = value.try_into().ok();
                }
                _ => {
                    if let Some(type_name) = value.get("type").and_then(toml::Value::as_str) {
                        blueprint.custom_components.push(CustomComponentConfig {
                            type_name: type_name.to_string(),
                            config: value
                                .as_table()
                                .map(|table| table.clone().into_iter().collect())
                                .unwrap_or_default(),
                        });
                    }
                }
            }
        }

        blueprint
    }

    fn apply_scene(&mut self, scene: SceneEntityConfig) {
        if scene.name.is_some() {
            self.name = scene.name;
        }

        if let Some(tags) = scene.tags {
            self.tags = Some(tags.values);
        }

        if let Some(data) = scene.data {
            self.data_refs.extend(data);
        }

        if scene.transform.is_some() {
            self.transform = merge_transform(self.transform, scene.transform.unwrap_or_default());
        }

        if let Some(sprite) = scene.sprite {
            self.sprite = Some(merge_sprite(self.sprite.take(), sprite));
        }

        for component in scene.components {
            self.custom_components
                .retain(|existing| existing.type_name != component.type_name);
            self.custom_components.push(component);
        }
    }

    fn validate_custom_components(&self, registry: &ComponentRegistry) -> GameResult<()> {
        for component in &self.custom_components {
            if !registry.contains(&component.type_name) {
                let name = self.name.as_deref().unwrap_or("<unnamed>");
                return Err(format!(
                    "unknown component type '{}' while loading entity '{}'; register it with ctx.components().register::<T>(\"{}\") before ctx.load_main_scene()",
                    component.type_name, name, component.type_name
                )
                .into());
            }
        }

        Ok(())
    }

    fn validate_data_refs(&self, paths: &ProjectPaths) -> GameResult<()> {
        for value in self.data_refs.values() {
            let resolved = paths.resolve_resource(value)?;

            if !resolved.is_file() {
                return Err(PathDiagnosticError::resource(
                    value.clone(),
                    resolved,
                    &paths.resource_root,
                    std::io::Error::new(std::io::ErrorKind::NotFound, "data resource not found"),
                )
                .into());
            }
        }

        Ok(())
    }

    fn into_runtime(self, assets: &mut Assets) -> GameResult<SceneEntityRuntime> {
        let sprite = match self.sprite {
            Some(sprite) => match sprite.texture {
                Some(texture) => Some(SpriteRenderer::new(
                    assets.texture(texture)?,
                    Vec2::new(sprite.width.unwrap_or(32.0), sprite.height.unwrap_or(32.0)),
                )),
                None => None,
            },
            None => None,
        };

        Ok(SceneEntityRuntime {
            name: self.name,
            tags: self.tags.unwrap_or_default(),
            data_refs: self.data_refs,
            custom_components: self
                .custom_components
                .into_iter()
                .map(|component| CustomComponentRef {
                    type_name: component.type_name,
                    config: toml::Value::Table(component.config.into_iter().collect()),
                })
                .collect(),
            transform: self.transform,
            sprite,
        })
    }
}

fn merge_sprite(
    base: Option<SceneSpriteConfig>,
    override_value: SceneSpriteConfig,
) -> SceneSpriteConfig {
    let mut base = base.unwrap_or_default();

    if override_value.texture.is_some() {
        base.texture = override_value.texture;
    }

    if override_value.width.is_some() {
        base.width = override_value.width;
    }

    if override_value.height.is_some() {
        base.height = override_value.height;
    }

    base
}

fn merge_transform(mut base: Transform2D, override_value: SceneTransformConfig) -> Transform2D {
    if let Some(x) = override_value.x {
        base.x = x;
    }

    if let Some(y) = override_value.y {
        base.y = y;
    }

    if let Some(rotation_radians) = override_value.rotation_radians {
        base.rotation_radians = rotation_radians;
    }

    if let Some(scale_x) = override_value.scale_x {
        base.scale_x = scale_x;
    }

    if let Some(scale_y) = override_value.scale_y {
        base.scale_y = scale_y;
    }

    base
}

fn parse_clear_color(value: &str) -> Option<ClearColor> {
    match value.to_ascii_lowercase().as_str() {
        "black" => Some(ClearColor::BLACK),
        "cornflower" | "cornflowerblue" => Some(ClearColor::CORNFLOWER),
        _ => None,
    }
}

fn parse_key_list(keys: Vec<String>) -> Vec<KeyCode> {
    keys.into_iter()
        .filter_map(|key| parse_key_code(&key))
        .collect()
}

fn parse_key_code(key: &str) -> Option<KeyCode> {
    match key {
        "ArrowUp" => Some(KeyCode::ArrowUp),
        "ArrowDown" => Some(KeyCode::ArrowDown),
        "ArrowLeft" => Some(KeyCode::ArrowLeft),
        "ArrowRight" => Some(KeyCode::ArrowRight),
        "KeyW" | "W" => Some(KeyCode::KeyW),
        "KeyA" | "A" => Some(KeyCode::KeyA),
        "KeyS" | "S" => Some(KeyCode::KeyS),
        "KeyD" | "D" => Some(KeyCode::KeyD),
        "Space" => Some(KeyCode::Space),
        "Enter" => Some(KeyCode::Enter),
        "Escape" => Some(KeyCode::Escape),
        _ => None,
    }
}

fn action_axis(input: &InputState, negative: &[KeyCode], positive: &[KeyCode]) -> f32 {
    let negative_pressed = negative.iter().any(|key| input.pressed(*key));
    let positive_pressed = positive.iter().any(|key| input.pressed(*key));

    match (negative_pressed, positive_pressed) {
        (true, false) => -1.0,
        (false, true) => 1.0,
        _ => 0.0,
    }
}

fn axis(input: &InputState, negative: KeyCode, positive: KeyCode) -> f32 {
    match (input.pressed(negative), input.pressed(positive)) {
        (true, false) => -1.0,
        (false, true) => 1.0,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_paths_resolve_under_distinct_roots() {
        let paths = ProjectPaths::new(
            PathBuf::from("/project/assets"),
            PathBuf::from("/project/resources"),
            PathBuf::from("/user/seishin2d"),
        );

        assert_eq!(
            paths
                .resolve_asset("asset://sprites/player.png")
                .expect("asset path"),
            PathBuf::from("/project/assets/sprites/player.png")
        );
        assert_eq!(
            paths
                .resolve_resource("res://scenes/main.scene.toml")
                .expect("resource path"),
            PathBuf::from("/project/resources/scenes/main.scene.toml")
        );
        assert_eq!(
            paths
                .resolve_user("user://save_001.dat")
                .expect("user path"),
            PathBuf::from("/user/seishin2d/save_001.dat")
        );

        assert!(paths.resolve_asset("res://sprites/player.png").is_err());
        assert!(paths
            .resolve_resource("asset://scenes/main.scene.toml")
            .is_err());
    }

    #[test]
    fn input_actions_map_axis2d_from_named_keys() {
        let config = InputConfig {
            actions: HashMap::from([(
                "move".to_string(),
                InputActionConfig::Axis2d {
                    left: vec!["ArrowLeft".to_string(), "KeyA".to_string()],
                    right: vec!["ArrowRight".to_string(), "KeyD".to_string()],
                    up: vec!["ArrowUp".to_string(), "KeyW".to_string()],
                    down: vec!["ArrowDown".to_string(), "KeyS".to_string()],
                },
            )]),
        };
        let actions = InputActions::from_config(config);
        let mut input = InputState::default();

        input.press(KeyCode::KeyD);
        input.press(KeyCode::KeyW);

        assert_eq!(actions.axis2d(&input, "move"), Vec2::new(1.0, -1.0));
        assert_eq!(actions.axis2d(&input, "missing"), Vec2::ZERO);
    }

    #[test]
    fn input_actions_map_button_just_pressed_from_named_keys() {
        let config = InputConfig {
            actions: HashMap::from([(
                "interact".to_string(),
                InputActionConfig::Button {
                    keys: vec!["Space".to_string(), "Enter".to_string()],
                },
            )]),
        };
        let actions = InputActions::from_config(config);
        let mut input = InputState::default();

        input.press(KeyCode::Enter);

        assert!(actions.button_pressed(&input, "interact"));
        assert!(actions.button_just_pressed(&input, "interact"));
        assert!(!actions.button_just_released(&input, "interact"));
    }

    #[test]
    fn world_renders_spawned_sprite_entities() {
        let texture = Texture {
            id: TextureId::new(7),
            data: TextureData::rgba8(TextureId::new(7), 1, 1, vec![255, 255, 255, 255])
                .expect("valid texture"),
        };
        let mut world = World::default();
        let entity = world.spawn_sprite(SpriteBundle {
            texture,
            transform: Transform2D::from_translation(1.0, 2.0),
            size: Vec2::splat(16.0),
        });

        world.translate(entity, Vec2::new(3.0, 4.0));
        world.entity(entity).translate(Vec2::new(1.0, 1.0));

        let mut render = RenderContext::new(ClearColor::BLACK);
        world.render_into(&mut render);
        let state = render.state();

        assert_eq!(state.textures.len(), 1);
        assert_eq!(state.sprites.len(), 1);
        assert_eq!(state.sprites[0].texture_id, TextureId::new(7));
        assert_eq!(state.sprites[0].transform.x, 5.0);
        assert_eq!(state.sprites[0].transform.y, 7.0);
        assert_eq!(state.sprites[0].width, 16.0);
        assert_eq!(state.sprites[0].height, 16.0);
    }

    #[test]
    fn world_queries_non_renderable_entities() {
        let mut world = World::default();
        let entity = world.spawn_scene_entity(SceneEntityRuntime {
            name: Some("Trigger".to_string()),
            tags: vec!["trigger".to_string()],
            data_refs: HashMap::new(),
            custom_components: Vec::new(),
            transform: Transform2D::default(),
            sprite: None,
        });

        assert_eq!(world.entity_by_name("Trigger"), Some(entity));
        assert_eq!(world.first_with_tag("trigger"), Some(entity));

        let mut render = RenderContext::new(ClearColor::BLACK);
        world.render_into(&mut render);
        assert!(render.state().sprites.is_empty());
    }

    #[test]
    fn scene_transform_overrides_are_field_level() {
        let base = Transform2D {
            x: 1.0,
            y: 2.0,
            rotation_radians: 0.5,
            scale_x: 3.0,
            scale_y: 4.0,
        };

        let merged = merge_transform(
            base,
            SceneTransformConfig {
                x: Some(9.0),
                ..Default::default()
            },
        );

        assert_eq!(merged.x, 9.0);
        assert_eq!(merged.y, 2.0);
        assert_eq!(merged.rotation_radians, 0.5);
        assert_eq!(merged.scale_x, 3.0);
        assert_eq!(merged.scale_y, 4.0);
    }

    #[test]
    fn main_scene_loads_prefabs_names_tags_and_data_refs() {
        let mut startup = basic_2d_startup();

        startup
            .components()
            .register::<TestController>("PlayerController")
            .expect("register component");
        startup.load_main_scene().expect("load scene");

        let player = startup
            .world()
            .entity_by_name("Player")
            .expect("player entity");
        let merchant = startup
            .world()
            .entity_by_name("Merchant")
            .expect("merchant entity");

        assert_eq!(startup.world().first_with_tag("player"), Some(player));
        assert!(startup.world().entities_with_tag("npc").contains(&merchant));
        assert!(startup
            .world()
            .has_custom_component(player, "PlayerController"));
        assert_eq!(
            startup.world().data_ref(merchant, "character"),
            Some("res://data/characters/merchant.toml")
        );
    }

    #[test]
    fn scene_loaded_player_moves_from_input_action() {
        let mut startup = basic_2d_startup();

        startup
            .components()
            .register::<TestController>("PlayerController")
            .expect("register component");
        startup.load_main_scene().expect("load scene");

        let resources = Resources::new(startup.paths.clone());
        let mut dialogue = DialogueState::default();
        let mut world = startup.world;
        let input_actions = startup.input_actions;
        let mut input = InputState::default();
        let mut audio = startup.audio;
        let player = world.first_with_tag("player").expect("player tag");
        let before = world.transform(player).expect("player transform");

        input.press(KeyCode::KeyD);
        let mut frame = FrameContext {
            input: &input,
            input_actions: &input_actions,
            audio: &mut audio,
            world: &mut world,
            resources: &resources,
            dialogue: &mut dialogue,
            frame: 1,
            delta_seconds: 1.0,
        };
        let movement = frame.input().axis2d("move");
        let displacement = movement * TestController::DEFAULT_SPEED * frame.delta_seconds();

        frame.world().entity(player).translate(displacement);

        let after = frame.world().transform(player).expect("player transform");
        assert!(after.x > before.x);
        assert_eq!(after.y, before.y);
    }

    #[test]
    fn dialogue_resources_load_from_character_data() {
        let startup = basic_2d_startup();
        let resources = Resources::new(startup.paths.clone());

        let character = resources
            .character("res://data/characters/merchant.toml")
            .expect("merchant character");
        let dialogue_path = character
            .dialogue
            .as_ref()
            .expect("dialogue ref")
            .default
            .as_str();
        let dialogue = resources.dialogue(dialogue_path).expect("dialogue data");

        assert_eq!(character.display_name, "Merchant");
        assert_eq!(dialogue.id, "merchant_intro");
        assert!(dialogue.text.contains("prototype village"));
    }

    #[test]
    fn generic_toml_resources_are_accessible_to_components() {
        let startup = basic_2d_startup();
        let resources = Resources::new(startup.paths.clone());
        let config = resources
            .toml("res://data/components/player_controller.toml")
            .expect("player controller config");

        assert_eq!(config.f32("speed"), Some(180.0));
    }

    #[test]
    fn unknown_scene_component_reports_clear_error() {
        let mut startup = basic_2d_startup();

        let error = startup
            .load_main_scene()
            .expect_err("unregistered PlayerController must fail");

        assert!(error
            .to_string()
            .contains("unknown component type 'PlayerController'"));
    }

    #[derive(Default)]
    struct TestController;

    impl TestController {
        const DEFAULT_SPEED: f32 = 180.0;
    }

    impl Component2D for TestController {
        fn update(&mut self, entity: Entity, context: &mut FrameContext<'_>) -> GameResult<()> {
            context.world().translate(entity, Vec2::new(1.0, 0.0));
            Ok(())
        }
    }

    fn basic_2d_startup() -> StartupContext {
        let project_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/basic_2d/Seishin.toml");
        let app = App::from_project(project_path).expect("basic_2d project");
        let paths = ProjectPaths::new(
            app.asset_root.clone(),
            app.resource_root.clone(),
            app.user_root.clone(),
        );
        let asset_root = AssetRoot::new(&app.asset_root).expect("asset root");

        StartupContext::new(
            asset_root,
            app.input_actions.clone(),
            app.clear_color,
            paths,
            Some("res://scenes/main.scene.toml".to_string()),
        )
    }
}

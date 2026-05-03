use std::time::{Duration, Instant};

use seishin2d_core::{Engine, Game};
use seishin2d_input::{InputState, KeyCode};
use seishin2d_render::{RenderSize, RenderState, Renderer};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use winit::dpi::{LogicalSize, Size};
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode as WinitKeyCode, PhysicalKey};
use winit::platform::web::{EventLoopExtWebSys, WindowBuilderExtWebSys};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::WindowBuilder;

use crate::{DesktopRuntimeError, FixedTimestep};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

impl WindowSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowConfig {
    pub title: String,
    pub size: WindowSize,
}

impl WindowConfig {
    pub fn new(title: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            title: title.into(),
            size: WindowSize::new(width, height),
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "seishin2d".to_string(),
            size: WindowSize::new(1280, 720),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DesktopRunConfig {
    pub window: WindowConfig,
    pub timestep: FixedTimestep,
}

impl DesktopRunConfig {
    pub fn new(window: WindowConfig) -> Self {
        Self {
            window,
            ..Self::default()
        }
    }

    pub fn with_timestep(mut self, timestep: FixedTimestep) -> Self {
        self.timestep = timestep;
        self
    }
}

impl Default for DesktopRunConfig {
    fn default() -> Self {
        Self {
            window: WindowConfig::default(),
            timestep: FixedTimestep::from_fps(60),
        }
    }
}

pub trait DesktopGame: Game + 'static {
    fn input_state(&mut self) -> &mut InputState;

    fn render_state(&self) -> RenderState<'_>;
}

pub fn run_desktop<G: DesktopGame>(
    engine: Engine,
    game: G,
    config: DesktopRunConfig,
) -> Result<(), DesktopRuntimeError> {
    let event_loop = EventLoop::new()?;
    let canvas = create_canvas(config.window.size.width, config.window.size.height)?;
    let window = WindowBuilder::new()
        .with_title(config.window.title.clone())
        .with_inner_size(Size::Logical(LogicalSize::new(
            config.window.size.width as f64,
            config.window.size.height as f64,
        )))
        .with_canvas(Some(canvas))
        .with_focusable(true)
        .build(&event_loop)?;

    spawn_local(async move {
        let window_id = window.id();
        let initial_size = window.inner_size();
        let raw_display_handle = match window.display_handle() {
            Ok(handle) => handle.as_raw(),
            Err(error) => {
                log_web_error(&format!("display handle unavailable: {error}"));
                return;
            }
        };
        let raw_window_handle = match window.window_handle() {
            Ok(handle) => handle.as_raw(),
            Err(error) => {
                log_web_error(&format!("window handle unavailable: {error}"));
                return;
            }
        };

        let mut engine = engine;
        let mut game = game;
        let mut renderer = match unsafe {
            Renderer::new(
                raw_display_handle,
                raw_window_handle,
                RenderSize::new(initial_size.width, initial_size.height),
            )
            .await
        } {
            Ok(renderer) => renderer,
            Err(error) => {
                log_web_error(&format!("renderer initialization failed: {error}"));
                return;
            }
        };

        if let Err(error) = game.ready(&mut engine) {
            log_web_error(&format!("game initialization failed: {error}"));
            return;
        }

        let mut input = DesktopInputFrame::default();
        let mut shutdown = false;
        let mut exit_requested = false;
        let timestep = Duration::from_secs_f32(config.timestep.delta_seconds);
        let max_frame_time = Duration::from_millis(250);
        let mut last_frame = Instant::now();
        let mut accumulator = Duration::ZERO;

        event_loop.spawn(move |event, event_loop| {
            let _keep_window_alive = &window;
            event_loop.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent {
                    window_id: current_window_id,
                    event,
                } if current_window_id == window_id => match event {
                    WindowEvent::CloseRequested => {
                        exit_requested = true;
                    }
                    WindowEvent::Resized(size) => {
                        renderer.resize(RenderSize::new(size.width, size.height));
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        input.apply_keyboard_input(event.physical_key, event.state);
                    }
                    _ => {}
                },
                Event::AboutToWait => {
                    let now = Instant::now();
                    let frame_time = now.duration_since(last_frame).min(max_frame_time);
                    last_frame = now;
                    accumulator += frame_time;

                    let escape_requested = input.begin_game_frame(game.input_state());

                    if exit_requested || escape_requested {
                        shutdown_web_game(&mut game, &mut engine, &mut shutdown);
                        event_loop.exit();
                        return;
                    }

                    while accumulator >= timestep {
                        let update_result = engine
                            .tick(config.timestep.delta_seconds)
                            .and_then(|context| game.update(&mut engine, context));

                        if let Err(error) = update_result {
                            log_web_error(&format!("update failed: {error}"));
                            shutdown_web_game(&mut game, &mut engine, &mut shutdown);
                            event_loop.exit();
                            return;
                        }

                        accumulator -= timestep;
                    }

                    match renderer.render(game.render_state()) {
                        Ok(()) => input.end_game_frame(game.input_state()),
                        Err(error) => {
                            log_web_error(&format!("render failed: {error}"));
                            shutdown_web_game(&mut game, &mut engine, &mut shutdown);
                            event_loop.exit();
                        }
                    }
                }
                Event::LoopExiting if !shutdown => {
                    shutdown_web_game(&mut game, &mut engine, &mut shutdown);
                }
                _ => {}
            }
        });
    });

    Ok(())
}

fn create_canvas(
    width: u32,
    height: u32,
) -> Result<web_sys::HtmlCanvasElement, DesktopRuntimeError> {
    let window = web_sys::window().ok_or_else(|| {
        DesktopRuntimeError::Render(seishin2d_render::RenderError::SurfaceCreation(
            "browser window unavailable".to_string(),
        ))
    })?;
    let document = window.document().ok_or_else(|| {
        DesktopRuntimeError::Render(seishin2d_render::RenderError::SurfaceCreation(
            "document unavailable".to_string(),
        ))
    })?;
    let body = document.body().ok_or_else(|| {
        DesktopRuntimeError::Render(seishin2d_render::RenderError::SurfaceCreation(
            "document body unavailable".to_string(),
        ))
    })?;
    let canvas = document
        .create_element("canvas")
        .map_err(|error| {
            DesktopRuntimeError::Render(seishin2d_render::RenderError::SurfaceCreation(format!(
                "canvas creation failed: {error:?}"
            )))
        })?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| {
            DesktopRuntimeError::Render(seishin2d_render::RenderError::SurfaceCreation(
                "created element was not a canvas".to_string(),
            ))
        })?;

    canvas.set_width(width);
    canvas.set_height(height);
    canvas
        .set_attribute(
            "style",
            "display:block;margin:auto;width:960px;height:540px;max-width:100vw;max-height:100vh;background:#6495ed;outline:1px solid #2f4f7f;",
        )
        .map_err(|error| {
            DesktopRuntimeError::Render(seishin2d_render::RenderError::SurfaceCreation(format!(
                "canvas style failed: {error:?}"
            )))
        })?;
    body.append_child(&canvas).map_err(|error| {
        DesktopRuntimeError::Render(seishin2d_render::RenderError::SurfaceCreation(format!(
            "canvas append failed: {error:?}"
        )))
    })?;

    Ok(canvas)
}

fn shutdown_web_game<G: DesktopGame>(game: &mut G, engine: &mut Engine, shutdown: &mut bool) {
    if *shutdown {
        return;
    }

    if let Err(error) = game.shutdown(engine) {
        log_web_error(&format!("game shutdown failed: {error}"));
    }

    *shutdown = true;
}

fn map_winit_key_code(key: PhysicalKey) -> Option<KeyCode> {
    match key {
        PhysicalKey::Code(WinitKeyCode::ArrowUp) => Some(KeyCode::ArrowUp),
        PhysicalKey::Code(WinitKeyCode::ArrowDown) => Some(KeyCode::ArrowDown),
        PhysicalKey::Code(WinitKeyCode::ArrowLeft) => Some(KeyCode::ArrowLeft),
        PhysicalKey::Code(WinitKeyCode::ArrowRight) => Some(KeyCode::ArrowRight),
        PhysicalKey::Code(WinitKeyCode::KeyW) => Some(KeyCode::KeyW),
        PhysicalKey::Code(WinitKeyCode::KeyA) => Some(KeyCode::KeyA),
        PhysicalKey::Code(WinitKeyCode::KeyS) => Some(KeyCode::KeyS),
        PhysicalKey::Code(WinitKeyCode::KeyD) => Some(KeyCode::KeyD),
        PhysicalKey::Code(WinitKeyCode::Space) => Some(KeyCode::Space),
        PhysicalKey::Code(WinitKeyCode::Enter) => Some(KeyCode::Enter),
        PhysicalKey::Code(WinitKeyCode::Escape) => Some(KeyCode::Escape),
        _ => None,
    }
}

#[derive(Debug, Default, Clone)]
struct DesktopInputFrame {
    state: InputState,
    escape_requested: bool,
}

impl DesktopInputFrame {
    fn apply_keyboard_input(&mut self, physical_key: PhysicalKey, element_state: ElementState) {
        let Some(key) = map_winit_key_code(physical_key) else {
            return;
        };

        match element_state {
            ElementState::Pressed => {
                self.state.press(key);

                if key == KeyCode::Escape {
                    self.escape_requested = true;
                }
            }
            ElementState::Released => {
                self.state.release(key);
            }
        }
    }

    fn begin_game_frame(&mut self, game_input: &mut InputState) -> bool {
        *game_input = self.state.clone();
        self.escape_requested
    }

    fn end_game_frame(&mut self, game_input: &InputState) {
        self.state = game_input.clone();
        self.state.end_frame();
        self.escape_requested = false;
    }
}

fn log_web_error(message: &str) {
    web_sys::console::error_1(&message.into());
}

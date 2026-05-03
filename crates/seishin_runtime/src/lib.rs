#[cfg(not(target_arch = "wasm32"))]
mod desktop;
mod error;
mod headless;
mod time;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
pub use desktop::{run_desktop, DesktopGame, DesktopRunConfig, WindowConfig, WindowSize};
pub use error::DesktopRuntimeError;
pub use headless::{run_headless, HeadlessRunConfig};
pub use time::FixedTimestep;
#[cfg(target_arch = "wasm32")]
pub use web::{run_desktop, DesktopGame, DesktopRunConfig, WindowConfig, WindowSize};

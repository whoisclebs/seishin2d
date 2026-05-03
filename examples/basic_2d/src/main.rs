use seishin2d::prelude::*;

mod components;

use components::PlayerController;

struct Game;

impl Game2D for Game {
    fn new(ctx: &mut StartupContext) -> GameResult<Self> {
        ctx.components()
            .register::<PlayerController>("PlayerController")?;

        Ok(Self)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> GameResult<()> {
    seishin2d::run::<Game>()
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn wasm_start() {
    if let Err(error) = seishin2d::run::<Game>() {
        panic!("seishin2d web startup failed: {error}");
    }
}

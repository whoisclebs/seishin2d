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
        report_web_startup_error(&format!("seishin2d web startup failed: {error}"));
    }
}

#[cfg(target_arch = "wasm32")]
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
    wasm_bindgen::throw_str(message);
}

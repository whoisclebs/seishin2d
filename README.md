<a id="readme-top"></a>

<br />
<div align="center">
  <a href="https://github.com/seishin2d/seishin2d">
    <img src=".github/assets/seishin2d.png" alt="seishin2d logo" width="128" height="128">
  </a>

  <h1 align="center">seishin2d</h1>

  <p align="center">
    A small Rust-first 2D game engine prototype with desktop rendering, assets, audio, and a future-safe FFI boundary.
    <br />
    <a href="docs/architecture.md"><strong>Explore the architecture »</strong></a>
    <br />
    <br />
    <a href="examples/basic_2d">View Example</a>
    &middot;
    <a href="https://github.com/seishin2d/seishin2d/issues/new?template=bug_report.yml">Report Bug</a>
    &middot;
    <a href="https://github.com/seishin2d/seishin2d/issues/new?template=feature_request.yml">Request Feature</a>
  </p>
</div>

`seishin2d` is a native 2D game engine prototype written in Rust. The project starts with a small desktop MVP and is designed around a stable public API boundary so future language bindings can call into the engine through C ABI / FFI instead of touching Rust internals.

The current MVP opens a desktop window, renders a sprite, loads assets from disk, handles keyboard input, plays a simple sound, and exposes a small future-safe FFI lifecycle boundary.

## Table of Contents

- [About](#about)
- [Current Status](#current-status)
- [Built With](#built-with)
- [Getting Started](#getting-started)
- [Usage](#usage)
- [Project Layout](#project-layout)
- [Architecture](#architecture)
- [Development](#development)
- [Testing](#testing)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License](#license)
- [Acknowledgments](#acknowledgments)

## About

`seishin2d` is not trying to compete with Godot, Unity, or Bevy. It is a focused engine-learning project with a pragmatic architecture:

```txt
Rust Engine Core
  -> Stable public Rust API
  -> C ABI / FFI
  -> Future bindings
```

Design goals:

- keep gameplay code independent from renderer internals;
- keep backend crates such as `wgpu`, `winit`, `kira`, and `image` behind engine APIs;
- expose simple handles, IDs, and C-compatible values across FFI;
- avoid Bevy-scale architecture until the MVP proves a real need;
- keep the developer experience pleasant through the `seishin2d` facade crate and `seishin2d::prelude::*`.

Initial targets:

- Windows desktop
- Linux desktop

Future targets:

- Android
- Go bindings through C ABI / FFI
- additional tooling after the engine core stabilizes

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Current Status

The repository currently contains the MVP vertical slice:

- desktop window through `winit`;
- update/render loop;
- keyboard input;
- `wgpu`-backed clear pass and sprite rendering;
- simple camera support;
- asset loading from disk;
- simple audio playback with graceful degradation;
- playable `examples/basic_2d` example;
- C ABI lifecycle smoke boundary in `seishin2d_ffi`.

Manual visual/audio validation is still required on a desktop session after automated checks pass.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Built With

- [Rust](https://www.rust-lang.org/)
- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [`winit`](https://crates.io/crates/winit) for desktop window and events
- [`wgpu`](https://crates.io/crates/wgpu) for GPU rendering
- [`image`](https://crates.io/crates/image) for image loading
- [`kira`](https://crates.io/crates/kira) for audio playback
- [`raw-window-handle`](https://crates.io/crates/raw-window-handle) for window/render integration
- [`bytemuck`](https://crates.io/crates/bytemuck) for safe GPU buffer casts

Planned or future-facing:

- `bevy_ecs` may be evaluated later, but it is intentionally deferred for the first MVP slice.
- `rapier2d` may be evaluated later for physics.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Getting Started

### Prerequisites

Install Rust with `rustup`:

```sh
rustup toolchain install stable
rustup default stable
```

This workspace currently uses:

- Rust edition: `2021`
- MSRV declared in Cargo: `1.75`

On Linux, `winit`, `wgpu`, and `kira` may require system packages for windowing, graphics, and audio depending on your distribution. Typical dependencies include X11/Wayland, Vulkan or GPU driver support, ALSA/PulseAudio/PipeWire development packages.

### Clone

```sh
git clone <repo-url>
cd seishin2d
```

### Build

```sh
cargo build --workspace
```

### Run The Demo

```sh
cargo run -p seishin2d_basic_2d
```

Controls:

- Arrow keys: move sprite
- Space: play sound, or log a controlled skip if audio is unavailable
- Escape: close the demo

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Usage

Game/example code should normally depend on the facade crate and import the prelude:

```rust
use seishin2d::prelude::*;

struct Game {
    player_texture: Texture,
    player: Sprite,
}

impl Game2D for Game {
    fn new(ctx: &mut StartupContext) -> GameResult<Self> {
        let player_texture = ctx.load_texture("sprites/player.png")?;
        let player = Sprite::new(
            player_texture.id(),
            Transform2D::from_translation(0.0, 0.0),
            96.0,
            96.0,
        );

        Ok(Self { player_texture, player })
    }

    fn update(&mut self, ctx: &mut FrameContext<'_>) -> GameResult<()> {
        let speed = 180.0 * ctx.delta_seconds();
        self.player.transform.x += ctx.axis(KeyCode::ArrowLeft, KeyCode::ArrowRight) * speed;
        self.player.transform.y += ctx.axis(KeyCode::ArrowUp, KeyCode::ArrowDown) * speed;
        Ok(())
    }

    fn render(&self, ctx: &mut RenderContext) {
        ctx.clear(ClearColor::CORNFLOWER);
        ctx.texture(&self.player_texture);
        ctx.sprite(self.player);
    }
}

fn main() -> GameResult<()> {
    App::new("my seishin2d game")
        .window_size(960, 540)
        .asset_root("assets")
        .run::<Game>()
}
```

See the complete example in [`examples/basic_2d`](examples/basic_2d).

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Project Layout

```txt
crates/
  seishin2d/          Facade crate and gameplay prelude
  seishin2d_core/     Engine config, lifecycle, transforms, IDs, core errors
  seishin2d_runtime/  Headless and desktop runtime orchestration
  seishin2d_render/   2D render types and wgpu renderer
  seishin2d_input/    Normalized input state
  seishin2d_assets/   Asset paths, roots, handles, image loading
  seishin2d_audio/    Audio facade and private kira backend
  seishin2d_physics/  Placeholder for future 2D collision/physics
  seishin2d_ffi/      C ABI boundary with opaque handles
examples/
  minimal/            Headless loop smoke example
  basic_2d/           Playable MVP demo
bindings/
  go/                 Future Go binding notes
docs/
  architecture.md     Architecture notes and crate boundaries
  adr/                Architecture decision records
tools/
  xtask/              Internal automation helper
```

Each crate keeps `src/lib.rs` as a small facade and places implementation in domain modules such as `engine`, `desktop`, `renderer`, `loader`, `state`, or `ffi`.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Architecture

The engine is split into focused crates. The facade crate `seishin2d` is the preferred entry point for game code. Lower-level subsystem crates remain available for internal composition and advanced users.

Important rules:

- Gameplay code should not depend on `wgpu`, `winit`, `kira`, or `image` directly.
- FFI must not expose Rust references, generics, traits, lifetimes, `Vec<T>`, `HashMap`, `String`, backend types, or ECS internals.
- `_reversa_sdd/` is reference material, not a requirement to clone Bevy.
- Android and Go are future-facing design constraints, not MVP implementation requirements.

More detail:

- [`docs/architecture.md`](docs/architecture.md)
- [`docs/adr/`](docs/adr)
- [`bindings/go/README.md`](bindings/go/README.md)

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Development

Recommended workflow before submitting changes:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --workspace
```

Useful commands:

```sh
cargo run -p seishin2d_basic_2d
cargo run -p xtask -- check
cargo test -p seishin2d_core
cargo test -p seishin2d_render
```

Rust repository practices used here:

- workspace-level dependency declarations;
- `Cargo.lock` tracked for reproducible workspace/example builds;
- MSRV declared with `rust-version`;
- strict Clippy gate with `-D warnings`;
- `xtask` reserved for internal automation;
- crate-level tests for pure logic;
- manual checklist for render/audio behavior that cannot be fully unit-tested.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Testing

Automated baseline:

```sh
cargo test --workspace --all-targets
cargo build --workspace
```

Full local gate:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --workspace
```

Manual demo checklist:

- Run `cargo run -p seishin2d_basic_2d`.
- Confirm a desktop window opens.
- Confirm the sprite appears over a clear background.
- Confirm arrow keys move the sprite.
- Press Space and confirm sound plays or a controlled skip is logged.
- Press Escape or close the window and confirm clean shutdown.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Roadmap

- [x] Rust workspace with modular engine crates
- [x] Facade crate with `seishin2d::prelude::*`
- [x] Desktop window and event loop
- [x] Basic input state
- [x] Asset root/path handling and image loading
- [x] Minimal wgpu renderer
- [x] Sprite rendering and camera math
- [x] Simple audio playback/degradation
- [x] Playable `basic_2d` MVP example
- [x] C ABI lifecycle smoke boundary
- [ ] Improve render batching and multi-sprite correctness
- [ ] Add stronger asset symlink/path regression tests
- [ ] Improve frame pacing/redraw policy
- [ ] Add a higher-level scene/entity API
- [ ] Evaluate `bevy_ecs` after MVP needs are clearer
- [ ] Add Go binding over the C ABI
- [ ] Explore Android runtime support

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Contributing

This project is still early. Contributions should keep the MVP philosophy intact: small, explicit, validated changes.

Suggested flow:

1. Open an issue or discussion for larger changes.
2. Create a feature branch.
3. Keep PRs focused on one subsystem or vertical slice.
4. Run the full local gate before submitting.
5. Include manual validation notes when touching windowing, rendering, input, or audio.

Please avoid:

- exposing backend internals in public APIs;
- expanding FFI before the safe Rust API is stable;
- adding broad ECS/editor/hot-reload abstractions before they are required;
- mixing unrelated refactors and features in one change.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## License

Distributed under the MIT License. See [`LICENSE`](LICENSE) for more information.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Acknowledgments

- The README structure is inspired by [Best-README-Template](https://github.com/othneildrew/Best-README-Template).
- The `_reversa_sdd/` corpus is used as architecture reference material for engine patterns and risks.
- Rust game development projects and crates such as `wgpu`, `winit`, `kira`, and `image` provide the foundation for this prototype.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

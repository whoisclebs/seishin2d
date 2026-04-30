# Roadmap

This roadmap tracks the intended direction for `seishin2d`. It is not a release promise. Priorities may change as the engine architecture is validated through examples and real usage.

## MVP Completed

- [x] Rust workspace with modular engine crates.
- [x] Facade crate with `seishin2d::prelude::*`.
- [x] Desktop window and event loop.
- [x] Basic input state with pressed and transition semantics.
- [x] Asset root/path handling and image loading.
- [x] Minimal `wgpu` renderer.
- [x] Sprite rendering and camera math.
- [x] Simple audio playback with graceful degradation.
- [x] Playable `examples/basic_2d` MVP example.
- [x] C ABI lifecycle smoke boundary.
- [x] Repository hygiene: README, license, contribution docs, issue/PR templates, CI, Dependabot, changelog, and Rust formatting config.

## Near Term

- [ ] Improve render batching and multi-sprite correctness.
- [ ] Add stronger asset symlink/path regression tests.
- [ ] Improve frame pacing and redraw policy.
- [ ] Reduce logging noise in the demo and replace `println!` with a proper tracing setup.
- [ ] Add a higher-level scene/entity API on top of the current `Game2D` facade.
- [ ] Add more ergonomic sprite helpers, such as `Sprite::from_texture`.
- [ ] Add smoke tests for the facade crate API.
- [ ] Document manual desktop validation expectations for Windows and Linux.

## Mid Term

- [ ] Evaluate whether `bevy_ecs` is needed after the MVP API is exercised by more examples.
- [ ] Add a simple 2D collision layer or integrate a physics backend behind `seishin2d_physics`.
- [ ] Add more asset formats and clearer asset error diagnostics.
- [ ] Add renderer resilience tests for resize/minimize/surface loss paths where practical.
- [ ] Expand the C ABI only after equivalent safe Rust APIs stabilize.
- [ ] Add Go binding proof of concept over the C ABI.

## Long Term

- [ ] Explore Android runtime support.
- [ ] Add editor/tooling experiments only after the runtime and asset model are stable.
- [ ] Investigate hot reload after the asset pipeline has stronger invariants.
- [ ] Add more complete documentation and tutorials.

## Explicit Non-Goals For Now

- Cloning Bevy, Godot, or Unity.
- Exposing renderer/window/audio backend internals in public gameplay APIs.
- Exposing Rust collections, references, traits, generics, or lifetimes across FFI.
- Adding a large ECS/plugin/editor architecture before the need is proven.

//! Shared example initialisation logic.
#![allow(unused)]
use glutin::{
    context::PossiblyCurrentContext,
    surface::{GlSurface, Surface, SurfaceAttributes, SurfaceAttributesBuilder, WindowSurface},
};
use std::{env, num::NonZeroU32};
use winit::window::Window;

/// Setup env vars, init logging & notify about --release performance.
pub fn init_example(example_name: &str) {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "gfx_glyph=warn");
    }
    env_logger::init();

    // disables vsync sometimes
    if cfg!(target_os = "linux") && env::var("vblank_mode").is_err() {
        env::set_var("vblank_mode", "0");
    }

    if cfg!(debug_assertions) && env::var("yes_i_really_want_debug_mode").is_err() {
        eprintln!(
            "Note: Release mode will improve performance greatly.\n    \
             e.g. use `cargo run --example {example_name} --release`"
        );
    }
}

fn main() {
    eprintln!("\"init\" isn't an example. Try: cargo run --example paragraph");
}

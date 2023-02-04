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
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "gfx_glyph=warn");
    }
    env_logger::init();

    if cfg!(debug_assertions) && env::var_os("yes_i_really_want_debug_mode").is_none() {
        eprintln!(
            "Note: Release mode will improve performance greatly.\n    \
             e.g. use `cargo run --example {example_name} --release`"
        );
    }

    // disables vsync maybe
    if env::var_os("vblank_mode").is_none() {
        env::set_var("vblank_mode", "0");
    }
}

fn main() {
    eprintln!("\"init\" isn't an example. Try: cargo run --example paragraph");
}

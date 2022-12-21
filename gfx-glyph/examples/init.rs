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

/// [`Window`] extensions for working with [`glutin`] surfaces.
pub trait WindowExt {
    /// Resize the surface to the window inner size.
    ///
    /// No-op if either window size is zero.
    fn resize_surface(&self, surface: &Surface<WindowSurface>, context: &PossiblyCurrentContext);
}

impl WindowExt for Window {
    fn resize_surface(&self, surface: &Surface<WindowSurface>, context: &PossiblyCurrentContext) {
        if let Some((w, h)) = self.inner_size().non_zero() {
            surface.resize(context, w, h)
        }
    }
}

/// [`winit::dpi::PhysicalSize<u32>`] non-zero extensions.
pub trait NonZeroU32PhysicalSize {
    /// Converts to non-zero `(width, height)`.
    fn non_zero(self) -> Option<(NonZeroU32, NonZeroU32)>;
}
impl NonZeroU32PhysicalSize for winit::dpi::PhysicalSize<u32> {
    fn non_zero(self) -> Option<(NonZeroU32, NonZeroU32)> {
        let w = NonZeroU32::new(self.width)?;
        let h = NonZeroU32::new(self.height)?;
        Some((w, h))
    }
}

fn main() {
    eprintln!("\"init\" isn't an example. Try: cargo run --example paragraph");
}

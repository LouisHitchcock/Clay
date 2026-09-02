//! Throwaway harness for Clay's Windows surface compositing path (Phase 1).
//!
//! An animating texture is produced on a **separate** D3D11 device, shared into this
//! process as an NT handle, and drawn through `gpui::surface`. The separate device is the
//! whole point: it is what CEF does from its GPU process, and it is what the GPUI side has
//! to cope with. Producing the texture on GPUI's own device would prove nothing.
//!
//! The layout deliberately exercises the two things the Phase 1 gate cares about beyond
//! "pixels appear": the surface is larger than a clipping parent, so `content_mask` has to
//! cut it off, and an opaque quad overlaps it, so GPUI content has to draw on top.
//!
//! Delete this once the browser is the real producer.

#[cfg(target_os = "windows")]
#[path = "example_support/fonts.rs"]
mod example_support;

#[cfg(target_os = "windows")]
mod imp {
    use anyhow::{Context as _, Result};
    use gpui::{
        App, Bounds, Context, DevicePixels, SharedTexture, Window, WindowBounds, WindowOptions,
        div, prelude::*, px, rgb, size, surface,
    };
    use gpui_platform::application;
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::Graphics::{
        Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0},
        Direct3D11::*,
        Dxgi::{Common::*, IDXGIResource1},
    };
    use windows::core::Interface;

    const TEXTURE_SIZE: u32 = 256;

    /// Access flags for `IDXGIResource1::CreateSharedHandle`. windows-rs does not expose
    /// the `DXGI_SHARED_RESOURCE_*` constants, so they are spelled out here.
    const DXGI_SHARED_RESOURCE_READ: u32 = 0x8000_0000;
    const DXGI_SHARED_RESOURCE_WRITE: u32 = 0x1;

    /// Stands in for CEF's GPU process: owns its own device and repaints a texture that
    /// GPUI only ever sees through a shared handle.
    struct Producer {
        context: ID3D11DeviceContext,
        texture: ID3D11Texture2D,
        handle: isize,
        pixels: Vec<u8>,
    }

    impl Producer {
        fn new() -> Result<Self> {
            let mut device = None;
            let mut context = None;
            unsafe {
                D3D11CreateDevice(
                    None,
                    D3D_DRIVER_TYPE_HARDWARE,
                    HMODULE::default(),
                    D3D11_CREATE_DEVICE_FLAG(0),
                    Some(&[D3D_FEATURE_LEVEL_11_0]),
                    D3D11_SDK_VERSION,
                    Some(&mut device),
                    None,
                    Some(&mut context),
                )
                .context("creating the producer D3D11 device")?;
            }
            let device = device.context("producer device was not created")?;
            let context = context.context("producer device context was not created")?;

            // SHARED_NTHANDLE has to be paired with SHARED, and the texture needs
            // SHADER_RESOURCE binding because the consumer samples it.
            let desc = D3D11_TEXTURE2D_DESC {
                Width: TEXTURE_SIZE,
                Height: TEXTURE_SIZE,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                CPUAccessFlags: 0,
                MiscFlags: (D3D11_RESOURCE_MISC_SHARED_NTHANDLE.0
                    | D3D11_RESOURCE_MISC_SHARED.0) as u32,
            };

            let mut texture = None;
            unsafe {
                device
                    .CreateTexture2D(&desc, None, Some(&mut texture))
                    .context("creating the shared texture")?;
            }
            let texture = texture.context("shared texture was not created")?;

            let resource: IDXGIResource1 = texture
                .cast()
                .context("querying IDXGIResource1 on the shared texture")?;
            let handle = unsafe {
                resource
                    .CreateSharedHandle(
                        None,
                        DXGI_SHARED_RESOURCE_READ | DXGI_SHARED_RESOURCE_WRITE,
                        None,
                    )
                    .context("creating a shared handle for the texture")?
            };

            Ok(Self {
                context,
                texture,
                handle: handle.0 as isize,
                pixels: vec![0; (TEXTURE_SIZE * TEXTURE_SIZE * 4) as usize],
            })
        }

        /// Repaint with a pattern that moves, so a static frame is obviously distinct from
        /// a live one, and with a bright border so clipping is easy to see.
        fn tick(&mut self, frame: u32) {
            let phase = frame % TEXTURE_SIZE;
            for y in 0..TEXTURE_SIZE {
                for x in 0..TEXTURE_SIZE {
                    let i = ((y * TEXTURE_SIZE + x) * 4) as usize;
                    let edge = x < 4 || y < 4 || x >= TEXTURE_SIZE - 4 || y >= TEXTURE_SIZE - 4;
                    let (b, g, r) = if edge {
                        (0u8, 255u8, 255u8)
                    } else {
                        let d = ((x + y + phase) % TEXTURE_SIZE) as u8;
                        (255u8.saturating_sub(d), d, ((x * 255) / TEXTURE_SIZE) as u8)
                    };
                    self.pixels[i] = b;
                    self.pixels[i + 1] = g;
                    self.pixels[i + 2] = r;
                    self.pixels[i + 3] = 255;
                }
            }

            // No fence or keyed mutex: the consumer may sample a partially written frame.
            // Acceptable for proving the path; real CEF integration needs synchronisation.
            unsafe {
                self.context.UpdateSubresource(
                    &self.texture,
                    0,
                    None,
                    self.pixels.as_ptr() as *const _,
                    TEXTURE_SIZE * 4,
                    0,
                );
                self.context.Flush();
            }
        }
    }

    struct SurfaceExample {
        producer: Producer,
        frame: u32,
    }

    impl Render for SurfaceExample {
        fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            self.frame = self.frame.wrapping_add(1);
            self.producer.tick(self.frame);
            window.request_animation_frame();

            let texture = SharedTexture {
                handle: self.producer.handle,
                size: size(
                    DevicePixels(TEXTURE_SIZE as i32),
                    DevicePixels(TEXTURE_SIZE as i32),
                ),
            };

            div()
                .size_full()
                .bg(rgb(0x1e1e2e))
                .flex()
                .flex_col()
                .gap_4()
                .p_4()
                .child("surface: clipped to 180x180, quad drawn over it")
                .text_color(rgb(0xffffff))
                .child(
                    div()
                        .relative()
                        .w(px(180.))
                        .h(px(180.))
                        .overflow_hidden()
                        .border_2()
                        .border_color(rgb(0xff00ff))
                        .child(surface(texture).w(px(300.)).h(px(300.)))
                        .child(
                            div()
                                .absolute()
                                .top(px(60.))
                                .left(px(60.))
                                .size(px(60.))
                                .bg(rgb(0xff0000)),
                        ),
                )
        }
    }

    pub fn run() {
        application().run(|cx: &mut App| {
            if !crate::example_support::load_fonts(cx) {
                return;
            }
            let producer = match Producer::new() {
                Ok(producer) => producer,
                Err(error) => {
                    eprintln!("failed to start the texture producer: {error:?}");
                    cx.quit();
                    return;
                }
            };
            let bounds = Bounds::centered(None, size(px(520.), px(360.)), cx);
            let window = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| SurfaceExample { producer, frame: 0 }),
            );
            if let Err(error) = window {
                eprintln!("failed to open a window: {error:?}");
                cx.quit();
                return;
            }
            cx.activate(true);
        });
    }
}

#[cfg(target_os = "windows")]
fn main() {
    imp::run();
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("the surface example currently only has a Windows producer");
}

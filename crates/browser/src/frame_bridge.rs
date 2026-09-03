//! Copies CEF's accelerated frames into a texture we own.
//!
//! CEF hands `on_accelerated_paint` an NT handle to a texture drawn by its GPU process, and
//! its documentation is explicit about the terms: the frames come from a pool, so the handle
//! differs from frame to frame, and "the handle's resource cannot be cached and cannot be
//! accessed outside of this callback. It should be reopened each time this callback is
//! executed and the contents should be copied to a texture owned by the client application.
//! The contents of |info| will be released back to the pool after this callback returns."
//!
//! So a shared handle cannot simply be forwarded to GPUI to open later on: by the time the
//! renderer runs, CEF has taken the texture back. This bridge does what CEF asks. It owns a
//! D3D11 device, opens CEF's texture inside the callback, blits it into a long-lived texture
//! of its own, and gives GPUI a handle to *that*. The destination handle never changes while
//! the page size holds steady, so GPUI can go on caching a view of it.
//!
//! No keyed mutex or fence guards the copy — CEF creates its textures without one, and Clay
//! runs CEF with `external_message_pump = 1`, which puts this callback on the main thread,
//! the same thread that later draws the scene. The copy and the draw are therefore already
//! ordered against each other.

use anyhow::{Context as _, Result};
use parking_lot::Mutex;
use std::sync::Arc;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HMODULE};
use windows::Win32::Graphics::{
    Direct3D::{D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1},
    Direct3D11::*,
    Dxgi::{Common::*, CreateDXGIFactory1, IDXGIAdapter, IDXGIFactory1, IDXGIResource1},
};
use windows::core::Interface as _;

/// Access flags for `IDXGIResource1::CreateSharedHandle`. windows-rs does not expose the
/// `DXGI_SHARED_RESOURCE_*` constants, so they are spelled out here.
const DXGI_SHARED_RESOURCE_READ: u32 = 0x8000_0000;
const DXGI_SHARED_RESOURCE_WRITE: u32 = 0x1;

pub struct FrameBridge {
    device: Arc<BridgeDevice>,
    target: Option<Target>,
}

/// The D3D11 device the copies run on, shared by every browser tab.
///
/// A device is expensive enough that one per tab is not worth it, and nothing about the copy is
/// tab-specific. Only the destination texture is, and that lives in `FrameBridge`.
struct BridgeDevice {
    device: ID3D11Device1,
    context: Mutex<ID3D11DeviceContext>,
}

// SAFETY: an `ID3D11Device` is free-threaded, so sharing it is sound. The immediate context is
// not, and is reachable only through the mutex above, which serialises every use of it.
unsafe impl Send for BridgeDevice {}
unsafe impl Sync for BridgeDevice {}

static DEVICE: Mutex<Option<Arc<BridgeDevice>>> = Mutex::new(None);

/// The process-wide bridge device, created on first use.
fn shared_device() -> Result<Arc<BridgeDevice>> {
    let mut slot = DEVICE.lock();
    if let Some(device) = slot.as_ref() {
        return Ok(device.clone());
    }

    let (adapter, device, context) = first_d3d11_adapter()?;
    if let Ok(desc) = unsafe { adapter.GetDesc() } {
        // Should name the same GPU as GPUI's own "Using GPU:" line. If it does not, this device
        // is on a different adapter and nothing will ever be shareable between the two — the
        // first thing to check if frames copy but never appear.
        log::info!(
            "[browser::frame_bridge] using GPU: {}",
            String::from_utf16_lossy(&desc.Description).trim_matches(char::from(0))
        );
    }

    let device = Arc::new(BridgeDevice {
        device: device
            .cast()
            .context("querying ID3D11Device1, needed to open CEF's NT shared handles")?,
        context: Mutex::new(context),
    });
    *slot = Some(device.clone());
    Ok(device)
}

/// The texture GPUI samples, and the handle it opens it by.
struct Target {
    texture: ID3D11Texture2D,
    handle: HANDLE,
    shared: gpui::SharedTexture,
    width: u32,
    height: u32,
    format: DXGI_FORMAT,
}

impl Drop for Target {
    fn drop(&mut self) {
        // Safe to close even while GPUI holds a view opened from it: an opened resource keeps
        // the underlying texture alive on its own.
        unsafe { CloseHandle(self.handle) }.ok();
    }
}

// SAFETY: the destination texture is a COM interface, which windows-rs does not mark `Send`.
// The only path to this value is through the mutex that owns it, so uses of it are serialised.
unsafe impl Send for FrameBridge {}

impl FrameBridge {
    pub fn new() -> Result<Self> {
        Ok(Self {
            device: shared_device()?,
            target: None,
        })
    }

    /// Copy the frame behind `cef_handle` into our own texture and describe the result.
    ///
    /// Must be called from inside `on_accelerated_paint`; `cef_handle` is dead once it
    /// returns.
    pub fn accept(&mut self, cef_handle: isize) -> Result<gpui::SharedTexture> {
        let source: ID3D11Texture2D = unsafe {
            self.device
                .device
                .OpenSharedResource1(HANDLE(cef_handle as *mut _))
                .context("opening CEF's shared frame texture")?
        };

        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { source.GetDesc(&mut desc) };

        // CEF's own description of the frame is the authority on size. The view dimensions we
        // asked for can be a resize ahead of the frame actually in hand.
        self.ensure_target(desc.Width, desc.Height, desc.Format)?;
        let target = self.target.as_ref().expect("ensured just above");

        let context = self.device.context.lock();
        unsafe {
            context.CopyResource(&target.texture, &source);
            // GPUI reads this texture through a different device, which will not see the work
            // until it has been submitted.
            context.Flush();
        }

        Ok(target.shared)
    }

    /// Make `self.target` a texture the frame can be copied into.
    fn ensure_target(&mut self, width: u32, height: u32, format: DXGI_FORMAT) -> Result<()> {
        let matches = self.target.as_ref().is_some_and(|target| {
            target.width == width && target.height == height && target.format == format
        });
        if !matches {
            // Dropped before the replacement is built, so the old handle is closed even if
            // creating the new texture fails.
            self.target = None;
            self.target = Some(Target::new(&self.device.device, width, height, format)?);
            log::debug!(
                "[browser::frame_bridge] frame target: {width}x{height} DXGI_FORMAT={}",
                format.0
            );
        }
        Ok(())
    }
}

/// Create a D3D11 device on the same adapter GPUI will be rendering with.
///
/// This has to match, or the shared handle the bridge hands over is meaningless: DXGI cannot
/// share a texture between two adapters. GPUI walks `EnumAdapters` from zero and keeps the
/// first adapter that gives it a D3D11 device (`gpui_windows::directx_devices`), so doing the
/// same here lands on the same one — enumeration order is stable within a process.
fn first_d3d11_adapter() -> Result<(IDXGIAdapter, ID3D11Device, ID3D11DeviceContext)> {
    let factory: IDXGIFactory1 =
        unsafe { CreateDXGIFactory1() }.context("creating a DXGI factory")?;

    let mut last_error = None;
    for index in 0.. {
        let Ok(adapter) = (unsafe { factory.EnumAdapters(index) }) else {
            break;
        };

        let mut device = None;
        let mut context = None;
        let created = unsafe {
            D3D11CreateDevice(
                &adapter,
                // Required when an adapter is given explicitly.
                D3D_DRIVER_TYPE_UNKNOWN,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&[D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0]),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )
        };

        match created {
            Ok(()) => {
                let device = device.context("D3D11 device was not created")?;
                let context = context.context("D3D11 device context was not created")?;
                return Ok((adapter, device, context));
            }
            Err(error) => last_error = Some(error),
        }
    }

    match last_error {
        Some(error) => {
            Err(error).context("creating a D3D11 device for browser frames on any adapter")
        }
        None => anyhow::bail!("no DXGI adapters to create a D3D11 device on"),
    }
}

impl Target {
    fn new(device: &ID3D11Device1, width: u32, height: u32, format: DXGI_FORMAT) -> Result<Self> {
        // SHARED_NTHANDLE has to be paired with SHARED, and GPUI samples the texture, so it
        // needs shader-resource binding.
        let desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: format,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: (D3D11_RESOURCE_MISC_SHARED_NTHANDLE.0 | D3D11_RESOURCE_MISC_SHARED.0)
                as u32,
        };

        let mut texture = None;
        unsafe {
            device
                .CreateTexture2D(&desc, None, Some(&mut texture))
                .with_context(|| {
                    format!("creating a {width}x{height} texture for browser frames")
                })?
        };
        let texture = texture.context("browser frame texture was not created")?;

        let resource: IDXGIResource1 = texture
            .cast()
            .context("querying IDXGIResource1 on the browser frame texture")?;
        let handle = unsafe {
            resource
                .CreateSharedHandle(
                    None,
                    DXGI_SHARED_RESOURCE_READ | DXGI_SHARED_RESOURCE_WRITE,
                    None,
                )
                .context("creating a shared handle for the browser frame texture")?
        };

        Ok(Self {
            shared: gpui::SharedTexture {
                id: gpui::SharedTexture::next_id(),
                handle: handle.0 as isize,
                size: gpui::size(
                    gpui::DevicePixels(width as i32),
                    gpui::DevicePixels(height as i32),
                ),
            },
            texture,
            handle,
            width,
            height,
            format,
        })
    }
}

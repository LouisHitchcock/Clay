> **RESOLVED 2026-09-03.** The blocker described below is fixed and Phase 2's gate has passed:
> a real page renders in a Clay pane on Windows. The hypothesis in section 4 was correct — CEF's
> shared texture handle is valid only for the duration of the callback. See the "Phase 2 gate
> PASSED" section of `CLAY.md` for the fix and for the three further bugs found afterwards.
> This file is kept for the diagnostic trail, not as a description of current state.

# Handover: Clay's browser renders everything except the page pixels

Read `CLAY.md` first for full project state and decision history. This file is the focused
brief on the one problem currently blocking progress.

---

## 1. What the project is

**Clay** is a fork of [Zed](https://github.com/zed-industries/zed) at
`https://github.com/LouisHitchcock/Clay`, working on branch `clay/rebrand-and-isolate`
(committed but **unpushed**). Local checkout: `C:\Users\Louis\Desktop\Code\Clay`.

It combines three things:

1. **An inline browser** — CEF/Chromium embedded as a pane item, ported from the
   [Glass](https://github.com/Glass-HQ/Glass) fork.
2. **Selected tooling from [Lathe](https://github.com/paterschris/lathe)** — multi-account AI
   sign-in, in-editor PR review, workspace groups. *Not started.*
3. **A Warp-style unified AI terminal** — one block timeline where the AI agent is the default
   input and `!` is the explicit shell escape. *Not started.*

Reference checkouts live in `_refs/` (git-ignored via `.git/info/exclude`):
`_refs/zed-upstream`, `_refs/lathe`. Glass is at `C:\Users\Louis\Desktop\Code\OLD\Glass`.

**Hard project rule:** never add `Co-Authored-By` or any AI attribution to commits. See `.rules`.

---

## 2. Where we are

- **Phase 0 (fork, rebrand, isolation from stock Zed)** — complete and committed.
- **Phase 1 (Windows D3D11 surface compositing in GPUI)** — complete and committed. Proven with
  a throwaway example (`crates/gpui/examples/surface.rs`) that produces an animating texture on a
  *separate* D3D11 device, shares it by NT handle, and composites it correctly through GPUI with
  clipping and z-ordering.
- **Phase 2 (browser port)** — nearly working. This document is about the last blocker.

### What works right now, verified on screen

- Clay builds and runs on Windows, fully isolated from the stock Zed install.
- `browser: open browser pane` (command palette) opens a real browser tab beside editor tabs.
- Browser chrome renders correctly: back/forward/stop, omnibox, download button, tab strip with
  favicons and page titles.
- **CEF genuinely loads pages.** A Google search resolved the URL, returned the page title
  ("Google - Google Search") and a favicon.
- `on_accelerated_paint` fires with `type_is_view=true info_present=true`, and one surface
  reaches the scene every frame (`1 surfaces` in the renderer batch counts).

### The one thing that does not work

The page content area shows **"Loading..." forever**, because the shared D3D11 texture cannot be
opened on the GPUI side.

---

## 3. The precise failure

```
ERROR [crates/gpui_windows/src/window.rs:991] ... 1 surfaces:
  opening a shared texture handle as both legacy (0x80070057) and NT handle: 0x80070057
```

`E_INVALIDARG` (`0x80070057`) from **both**:

- `ID3D11Device::OpenSharedResource` — the legacy path, for handles from `GetSharedHandle`
- `ID3D11Device1::OpenSharedResource1` — the NT path, for handles from `CreateSharedHandle`

So it is **not** simply a matter of having picked the wrong API. Both were tried; both reject it.

---

## 4. The decisive clue and the leading hypothesis

Diagnostic output from `on_accelerated_paint` (temporary `eprintln!`, already in the tree):

```
[browser] on_accelerated_paint called: type_is_view=true info_present=true
[browser] frame: handle=0x1d48 size=1296x910
[browser] frame: handle=0x1d34 size=1296x910
[browser] frame: handle=0x1d88 size=1296x910
[browser] frame: handle=0x1d18 size=1296x910
[browser] frame: handle=0x1d84 size=1296x910
[browser] frame: handle=0x1dac size=1296x910
[browser] frame: handle=0x1d54 size=1296x910
[browser] frame: handle=0x1d4c size=1296x910
[browser] frame: handle=0x1d88 size=1296x910
[browser] frame: handle=0x1f04 size=1296x910
```

Two observations:

1. **The handle differs on almost every frame.** Values are small — typical Windows kernel handle
   values — and occasionally repeat (`0x1d88` appears twice). This looks like CEF cycling a small
   pool of back buffers, or duplicating a handle per frame.
2. **The size is stable and correct** at 1296x910, matching the pane. So the callback data is
   sane; only the handle is unusable.

### Hypothesis

**The handle is only valid for the duration of the callback.** CEF hands over a per-frame handle
and closes or recycles it as soon as `on_accelerated_paint` returns.

The current design stores the raw handle in `PaintSurface` and opens it **later**, on the render
thread during `draw_surfaces` — by which time it is dead. That fits `E_INVALIDARG` from both open
calls exactly.

This also means the per-handle SRV cache in `directx_renderer.rs` is wrong under this model: it
would grow without bound, one entry per frame.

**Why Phase 1 did not catch this:** the test producer created one long-lived NT-handle texture
and kept it alive for the process lifetime. It validated the compositing path but not the
handle-lifetime assumption.

---

## 5. What to try next

1. **Open the shared texture inside `on_accelerated_paint`, not later.** Keep the resulting
   `ID3D11Texture2D` (or its SRV) alive and hand *that* to the renderer instead of a raw handle.
   This is where the evidence points.
   - Requires a D3D11 device in the callback. Either expose GPUI's device
     (`DirectXRendererDevices::device` in `crates/gpui_windows/src/directx_renderer.rs`, not
     currently public outside that crate), or create a second device and share between them.
   - `SharedTexture` currently carries an `isize` **deliberately**, so that `Scene` stays `Send`:
     windows-rs COM types are neither `Send` nor `Sync`. Carrying an opened texture instead needs
     care here — probably a `Send`-safe wrapper or an index into a renderer-side table.
2. **Or copy the texture during the callback** into a texture GPUI owns. Trades a GPU copy for a
   much simpler lifetime story. Slower, but a quick way to *confirm the hypothesis* before
   committing to a zero-copy design.
3. **Check CEF 145's documented lifetime** for `shared_texture_handle` in
   `cef_accelerated_paint_info_t`. If CEF documents it as callback-scoped, that settles it. The
   cef-rs bindings are at
   `~/.cargo/git/checkouts/cef-rs-*/3346010/cef/src/bindings/x86_64_pc_windows_msvc.rs`.
4. **Confirm CEF is using the real GPU.** The distribution ships `vk_swiftshader.dll` and
   `libEGL.dll`. If CEF's GPU process is compositing through ANGLE or SwiftShader rather than
   native D3D11, the texture may not be shareable with GPUI's device at all. GPUI itself is
   confirmed on the RTX 4060 with D3D11.1. An earlier run logged
   `Network service crashed or was terminated, restarting service`, which may or may not be
   related.

---

## 6. Key files

| File | Role |
|---|---|
| `crates/browser/src/render_handler.rs` | `on_accelerated_paint`; reads `info.shared_texture_handle`, builds a `SharedTexture`. **Has temporary `eprintln!` diagnostics.** |
| `crates/gpui_windows/src/directx_renderer.rs` | `draw_surfaces` and `shared_texture_view` — where the open fails. Per-handle SRV cache lives here. |
| `crates/gpui/src/scene.rs` | `SharedTexture { handle: isize, size }` and `PaintSurface`. |
| `crates/gpui/src/elements/surface.rs` | `SurfaceSource::SharedTexture`, `surface()`. |
| `crates/gpui_windows/src/shaders.hlsl` | `surface_vertex` / `surface_fragment` (working; validated with `fxc`). |
| `crates/browser/src/browser.rs` | `open_browser_tab`, `browser::init`, toolbar attach. |
| `crates/browser/src/browser_view.rs` | `BrowserView`, `BrowserPaneItem`. **Has a temporary `eprintln!` in `render`.** |
| `crates/browser/src/tab.rs` | `windowless_rendering_enabled: 1`, `shared_texture_enabled: 1` at ~line 286. |

---

## 7. Build, run, debug

```powershell
# MSVC environment is required
$vs = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools"
Import-Module (Join-Path $vs "Common7\Tools\Microsoft.VisualStudio.DevShell.dll")
Enter-VsDevShell -VsInstallPath $vs -SkipAutomaticLocation -DevCmdArguments "-arch=x64 -host_arch=x64"

cd C:\Users\Louis\Desktop\Code\Clay
cargo build -p zed          # debug. Release is far slower: codegen-units = 1, ~19 min for the zed crate alone
powershell -ExecutionPolicy Bypass -File .\script\stage-windows-cef-runtime.ps1
.\target\debug\clay.exe
```

Then `ctrl-shift-p` and pick **`browser: open browser pane`**.

**Debugging note that cost real time:** Clay's log file is buffered and only flushed on exit, so
`log::info!` is invisible while the app runs. Use `eprintln!` and capture stderr.
`_refs/run_clay.bat` launches Clay with stderr redirected to `_refs/clay_stderr.txt`.

Note also that `cargo build` fails with a file-lock error if Clay is running — close it first.

---

## 8. Bugs already found and fixed — do not re-investigate

1. **Empty `locales/`.** `Copy-Item -LiteralPath` does not expand wildcards, so the staging script
   copied no locale paks. CEF then died with `STATUS_BREAKPOINT` (`0x80000003`) *before logging
   initialised*, which looked like a startup crash. Fixed by using `-Path`.
2. **`BrowserPaneItem` held a `WeakEntity<BrowserView>`.** The only strong reference was a local
   in `open_browser_tab`, dropped on return, so `render` fell through to an empty div — a blank
   pane with a working tab and toolbar. Glass never hit this because its mode registry owned the
   view. Now a strong `Entity`.
3. **A fresh `BrowserView` has no tabs**, and with no active tab the content area falls through to
   the surface path with nothing to draw. `open_browser_tab` now calls `add_tab`.
4. **`#[cfg(unix)]` capture.** Inserting `handle_cef_subprocess()` immediately before
   `util::prevent_root_execution()` placed it under that attribute, which would have disabled CEF
   on Windows entirely.
5. **`ctrl-alt-*` keybindings do not fire** on this machine — almost certainly AltGr on a UK
   layout. `browser::OpenBrowserPane` is bound to the chord `ctrl-k ctrl-b`, which also did not
   fire; the command palette is the reliable route. Worth revisiting.

---

## 9. Known gaps unrelated to this bug

- **Right-click menus are stubbed** (`TODO`s in `tab_strip.rs` and `bookmarks.rs`). Glass's AppKit
  popup dispatches on an item *index* from a single callback; `ui::ContextMenu` takes a handler
  per entry and needs menu state plus a dismiss subscription on the owning view.
- **`SerializableItem` is not implemented**, so browser tabs do not survive a restart. Louis has
  explicitly asked for this. Use `workspace::register_serializable_item::<BrowserView>` — Zed's
  own machinery, requiring **no** changes to `crates/workspace`.
- `ZED_*` variables in `script/` and `.github/` were not renamed, so packaging is inconsistent
  with the Rust code, which now expects `CLAY_*`.
- The `GLASS_CEF_DEBUG` env var should become `CLAY_CEF_DEBUG`.
- The renderer's **"scene too large" error prefix is misleading** — it is a `with_context` wrapper
  around every batch failure, not an actual size problem. Worth rewording.
- Temporary `eprintln!` diagnostics in `render_handler.rs` and `browser_view.rs` should be removed
  once this is solved.

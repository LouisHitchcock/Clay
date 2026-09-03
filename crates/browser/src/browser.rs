//! Browser Mode for Glass
//!
//! This crate provides the browser mode functionality, integrating
//! Chromium Embedded Framework (CEF) for a full browser experience within Glass.

mod bookmarks;
mod browser_view;
mod cef_instance;
mod client;
mod context_menu_handler;
mod display_handler;
mod download_handler;
mod events;
mod find_handler;
#[cfg(target_os = "windows")]
mod frame_bridge;
pub mod history;
mod input;
mod keycodes;
mod life_span_handler;
mod load_handler;
#[cfg(target_os = "macos")]
mod macos_protocol;
mod new_tab_page;
mod omnibox;
mod page_chrome;
mod permission_handler;
mod render_handler;
mod request_handler;
mod session;
mod tab;
mod text_input;
mod toolbar;

pub use browser_view::{
    BrowserDownloadItem, BrowserPaneItem, BrowserSurfaceState, BrowserView, OpenBrowserPane,
};
pub use cef_instance::CefInstance;
pub use cef_instance::build_cef_app;
pub use tab::BrowserTab;

/// Handle CEF subprocess execution. This MUST be called very early in main(),
/// before any GUI initialization. See CefInstance::handle_subprocess() for details.
pub fn handle_cef_subprocess() -> anyhow::Result<()> {
    CefInstance::handle_subprocess()
}

use gpui::{App, AppContext as _, Entity, Focusable, Window};
use toolbar::{BrowserToolbar, BrowserToolbarStyle};
use workspace::Workspace;

fn attach_browser_toolbar_to_pane(
    pane: &Entity<workspace::Pane>,
    browser_view: &Entity<BrowserView>,
    window: &mut Window,
    cx: &mut gpui::Context<workspace::Workspace>,
) {
    let toolbar = pane.read(cx).toolbar().clone();

    // A pane keeps one toolbar item per type, so a second browser tab in the same pane reuses
    // the toolbar the first one added. It rebinds itself to whichever browser is in front; all
    // that is needed here is to point this view at it, so `FocusOmnibox` has something to
    // focus.
    let browser_toolbar = match toolbar.read(cx).item_of_type::<BrowserToolbar>() {
        Some(browser_toolbar) => browser_toolbar,
        None => {
            let (history, browser_focus_handle, active_tab) =
                browser_view.read_with(cx, |browser_view, cx| {
                    (
                        browser_view.history().clone(),
                        browser_view.focus_handle(cx),
                        browser_view.active_tab().cloned(),
                    )
                });

            toolbar.update(cx, |toolbar, cx| {
                let browser_toolbar = cx.new(|cx| {
                    BrowserToolbar::new(
                        browser_view.clone(),
                        history,
                        browser_focus_handle,
                        active_tab,
                        BrowserToolbarStyle::Pane,
                        window,
                        cx,
                    )
                });
                toolbar.add_item(browser_toolbar.clone(), window, cx);
                browser_toolbar
            })
        }
    };

    browser_view.update(cx, |browser_view, _| {
        browser_view.set_pane_toolbar(browser_toolbar);
    });
}

/// Opens a browser as an ordinary tab in the workspace's active pane, so it sits
/// alongside editor tabs rather than taking over the window as it does in Glass.
fn open_browser_tab(
    workspace: &mut Workspace,
    window: &mut Window,
    cx: &mut gpui::Context<Workspace>,
) {
    let browser_view = cx.new(|cx| BrowserView::new(cx));

    // `BrowserView::new` already leaves a tab behind — the restored session, or a new tab page
    // if there was nothing to restore. Adding one here as well is what left a tab stranded in
    // the strip on every launch, since the session is saved back out afterwards. Only step in
    // if there is genuinely nothing, which is the case when CEF failed to start.
    browser_view.update(cx, |view, cx| {
        if !view.has_tabs() {
            view.add_tab(cx);
        }
    });

    let item = BrowserPaneItem::new(&browser_view, cx.entity().downgrade(), cx);
    workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);

    // Attach the toolbar directly rather than waiting on `Event::ItemAdded`, so the omnibox
    // and navigation buttons are present the moment the tab opens.
    let pane = workspace.active_pane().clone();
    attach_browser_toolbar_to_pane(&pane, &browser_view, window, cx);
}

pub fn init(cx: &mut App) {
    match CefInstance::initialize(cx) {
        Ok(_) => {
            // Ensure CEF is shut down before the process exits. Without this,
            // exit() triggers CEF's static CefShutdownChecker destructor which
            // asserts that CefShutdown() was already called.
            //
            // CefInstance::shutdown() handles everything: it takes all browser
            // handles from the global registry, force-closes them, drops the
            // Rust refs (so CEF's BrowserContext ref counts reach zero), pumps
            // the message loop, then calls cef::shutdown().
            std::mem::forget(cx.on_app_quit(|_| async {
                CefInstance::shutdown();
            }));
        }
        Err(e) => {
            log::error!(
                "[browser] init() Failed to initialize CEF: {}. Browser mode will show placeholder.",
                e
            );
        }
    }

    cx.observe_new(
        |workspace: &mut workspace::Workspace,
         window: Option<&mut Window>,
         cx: &mut gpui::Context<workspace::Workspace>| {
            workspace.register_action(
                |workspace, _: &browser_view::OpenBrowserPane, window, cx| {
                    open_browser_tab(workspace, window, cx);
                },
            );

            let Some(window) = window else {
                return;
            };

            let workspace_handle = cx.entity();
            cx.subscribe_in(&workspace_handle, window, {
                move |workspace, _, event, window, cx| match event {
                    workspace::Event::ItemAdded { item } => {
                        let Some(pane_item) = item.downcast::<BrowserPaneItem>() else {
                            return;
                        };
                        let browser_view = pane_item.read(cx).browser_view().clone();
                        let pane = workspace.active_pane().clone();
                        attach_browser_toolbar_to_pane(&pane, &browser_view, window, cx);
                    }
                    _ => {}
                }
            })
            .detach();
        },
    )
    .detach();
}

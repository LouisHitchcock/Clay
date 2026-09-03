use crate::BrowserView;
use crate::history::BrowserHistory;
use crate::omnibox::{Omnibox, OmniboxEvent};
use crate::tab::{BrowserTab, TabEvent};
use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, Render, Subscription,
    WeakEntity, Window,
};
use ui::{IconName, Tooltip, h_flex, prelude::*};
use workspace::{
    ItemHandle, ToolbarItemEvent, ToolbarItemLocation, ToolbarItemView,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserToolbarStyle {
    TitleBar,
    Pane,
}

pub struct BrowserToolbar {
    browser_view: WeakEntity<BrowserView>,
    tab: Option<Entity<BrowserTab>>,
    omnibox: Entity<Omnibox>,
    style: BrowserToolbarStyle,
    tab_subscription: Option<Subscription>,
    browser_view_subscription: Option<Subscription>,
    _omnibox_subscription: Subscription,
}

impl BrowserToolbar {
    pub fn new(
        browser_view: Entity<BrowserView>,
        history: Entity<BrowserHistory>,
        browser_focus_handle: FocusHandle,
        active_tab: Option<Entity<BrowserTab>>,
        style: BrowserToolbarStyle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let omnibox = cx.new(|cx| Omnibox::new(history, browser_focus_handle, window, cx));

        let omnibox_subscription = cx.subscribe(&omnibox, {
            move |_this, _omnibox, event: &OmniboxEvent, cx| match event {
                OmniboxEvent::Navigate(url) => {
                    let url = url.clone();
                    if let Some(tab) = _this.tab.clone() {
                        tab.update(cx, |tab, cx| {
                            tab.navigate(&url, cx);
                            tab.set_focus(true);
                        });
                    }
                }
            }
        });

        let mut this = Self {
            browser_view: browser_view.downgrade(),
            tab: active_tab,
            omnibox,
            style,
            tab_subscription: None,
            browser_view_subscription: None,
            _omnibox_subscription: omnibox_subscription,
        };
        this.bind_browser_view(browser_view, window, cx);
        this
    }

    /// Follow `browser_view`: its active tab now, and whichever tab becomes active later.
    ///
    /// A pane holds at most one toolbar item of each type, so a single `BrowserToolbar` has to
    /// serve every browser tab in the pane. Binding once at construction left it showing the
    /// state of whichever browser happened to be open first — which is how the omnibox came to
    /// be missing, since that first tab was a new tab page and the omnibox is hidden there.
    fn bind_browser_view(
        &mut self,
        browser_view: Entity<BrowserView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // `browser_view` is already set by the constructor, so an id comparison alone would
        // skip installing the observation on the very first call.
        let is_new_view = self.browser_view_subscription.is_none()
            || self.browser_view.entity_id() != browser_view.entity_id();
        if is_new_view {
            self.browser_view = browser_view.downgrade();
            self.browser_view_subscription = Some(cx.observe_in(
                &browser_view,
                window,
                |this, browser_view, window, cx| {
                    let tab = browser_view.read(cx).active_tab().cloned();
                    this.bind_active_tab_if_changed(tab, window, cx);
                },
            ));
        }

        let tab = browser_view.read(cx).active_tab().cloned();
        self.bind_active_tab_if_changed(tab, window, cx);
    }

    fn bind_active_tab_if_changed(
        &mut self,
        tab: Option<Entity<BrowserTab>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // `tab` is populated by the constructor before anything is subscribed, so an id
        // comparison alone would skip the initial bind and leave the omnibox with no
        // subscription — which is how it came to stay empty as pages loaded.
        let bound = self.tab_subscription.is_some();
        let current = self.tab.as_ref().map(Entity::entity_id);
        if bound && current == tab.as_ref().map(Entity::entity_id) {
            // Same tab, but something about it may have changed — the omnibox appears once a
            // new tab page navigates, for one.
            cx.notify();
            return;
        }
        self.bind_active_tab(tab, window, cx);
    }

    pub fn focus_omnibox(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.omnibox.update(cx, |omnibox, cx| {
            omnibox.focus_and_select_all(window, cx);
        });
    }

    fn bind_active_tab(
        &mut self,
        tab: Option<Entity<BrowserTab>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.tab_subscription = None;
        self.tab = tab;

        if let Some(tab) = self.tab.clone() {
            self.tab_subscription = Some(cx.subscribe_in(&tab, window, {
                let omnibox = self.omnibox.clone();
                move |_this, _tab, event, window, cx| match event {
                    TabEvent::AddressChanged(url) => {
                        let url = url.clone();
                        omnibox.update(cx, |omnibox, cx| {
                            omnibox.set_url(&url, window, cx);
                        });
                    }
                    TabEvent::LoadingStateChanged | TabEvent::TitleChanged => {
                        cx.notify();
                    }
                    _ => {}
                }
            }));

            let url = tab.read(cx).url().to_string();
            self.omnibox.update(cx, |omnibox, cx| {
                omnibox.set_url(&url, window, cx);
            });
        }

        cx.notify();
    }

    fn toggle_download_center(
        &mut self,
        _: &gpui::ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(browser_view) = self.browser_view.upgrade() {
            browser_view.update(cx, |browser_view, cx| {
                browser_view.toggle_download_center(cx);
            });
        }
    }

    fn go_back(&mut self, _: &gpui::ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.tab.clone() {
            tab.update(cx, |tab, _| {
                tab.go_back();
            });
        }
    }

    fn go_forward(&mut self, _: &gpui::ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.tab.clone() {
            tab.update(cx, |tab, _| {
                tab.go_forward();
            });
        }
    }

    fn reload(&mut self, _: &gpui::ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.tab.clone() {
            tab.update(cx, |tab, _| {
                tab.reload();
            });
        }
    }

    fn stop(&mut self, _: &gpui::ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.tab.clone() {
            tab.update(cx, |tab, _| {
                tab.stop();
            });
        }
    }
}

impl EventEmitter<ToolbarItemEvent> for BrowserToolbar {}

impl Focusable for BrowserToolbar {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.omnibox.focus_handle(cx)
    }
}

impl ToolbarItemView for BrowserToolbar {
    fn set_active_pane_item(
        &mut self,
        active_pane_item: Option<&dyn ItemHandle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> ToolbarItemLocation {
        let Some(item) =
            active_pane_item.and_then(|item| item.downcast::<crate::BrowserPaneItem>())
        else {
            return ToolbarItemLocation::Hidden;
        };

        // This is the only hook that fires when the pane brings a different browser tab to
        // the front, so it is where the toolbar has to change which browser it reflects.
        let browser_view = item.read(cx).browser_view().clone();
        self.bind_browser_view(browser_view, window, cx);
        ToolbarItemLocation::PrimaryLeft
    }
}

impl Render for BrowserToolbar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_new_tab_page = self
            .tab
            .as_ref()
            .is_some_and(|tab| tab.read(cx).is_new_tab_page());
        let can_go_back = self
            .tab
            .as_ref()
            .is_some_and(|tab| tab.read(cx).can_go_back());
        let can_go_forward = self
            .tab
            .as_ref()
            .is_some_and(|tab| tab.read(cx).can_go_forward());
        let is_loading = self
            .tab
            .as_ref()
            .is_some_and(|tab| tab.read(cx).is_loading());
        let show_navigation_buttons = !is_new_tab_page;
        let show_omnibox = !is_new_tab_page;
        let show_downloads_button = true;

        h_flex()
            .w_full()
            .min_w_0()
            .when(self.style == BrowserToolbarStyle::TitleBar, |this| {
                this.max_w(px(680.)).h_full().px_2()
            })
            .when(self.style == BrowserToolbarStyle::Pane, |this| {
                this.min_h_8()
            })
            .items_center()
            .gap_1()
            .key_context("BrowserToolbar")
            .when(show_navigation_buttons, |this| {
                this.child(
                    IconButton::new("back", IconName::ChevronLeft)
                        .disabled(!can_go_back)
                        .tooltip(Tooltip::text("Go Back"))
                        .on_click(cx.listener(Self::go_back)),
                )
                .child(
                    IconButton::new("forward", IconName::ChevronRight)
                        .disabled(!can_go_forward)
                        .tooltip(Tooltip::text("Go Forward"))
                        .on_click(cx.listener(Self::go_forward)),
                )
                .child(if is_loading {
                    IconButton::new("stop", IconName::Stop)
                        .on_click(cx.listener(Self::stop))
                        .tooltip(Tooltip::text("Stop"))
                } else {
                    IconButton::new("reload", IconName::RotateCw)
                        .on_click(cx.listener(Self::reload))
                        .tooltip(Tooltip::text("Reload"))
                })
            })
            .when(show_omnibox, |this| this.child(self.omnibox.clone()))
            .when(show_downloads_button, |this| {
                this.child(
                    IconButton::new("downloads", IconName::Download)
                        .on_click(cx.listener(Self::toggle_download_center))
                        .tooltip(Tooltip::text("Downloads")),
                )
            })
    }
}

use gpui::{
    AnyElement, App, Context, ElementId, IntoElement, Pixels, Point, SharedString, Styled,
    WeakEntity, Window, prelude::*,
};
#[cfg(not(target_os = "macos"))]
use gpui::{MouseButton, ParentElement, div, img, px, rems};
use ui::prelude::*;
use ui::{Icon, IconName, IconSize};

use super::BrowserView;

type HoverCallback = Box<dyn Fn(&(), &mut Window, &mut App) + 'static>;

/// Stands in for Glass's AppKit tracking view: a transparent overlay that reports when the
/// pointer enters and leaves it.
///
/// GPUI reports hover as a single boolean rather than as two events, so the separate enter
/// and exit callbacks are held here and dispatched from one `on_hover`. Keeping Glass's
/// two-callback shape means the call sites did not have to be restructured.
struct HoverTracker {
    element: gpui::Stateful<gpui::Div>,
    on_enter: Option<HoverCallback>,
    on_exit: Option<HoverCallback>,
}

fn hover_tracker(id: impl Into<ElementId>) -> HoverTracker {
    HoverTracker {
        element: div().id(id),
        on_enter: None,
        on_exit: None,
    }
}

impl HoverTracker {
    fn on_mouse_enter(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_enter = Some(Box::new(handler));
        self
    }

    fn on_mouse_exit(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_exit = Some(Box::new(handler));
        self
    }
}

// Callers style and position the tracker themselves, exactly as they did with Glass's
// version, so styling is delegated to the wrapped div.
impl Styled for HoverTracker {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.element.style()
    }
}

impl IntoElement for HoverTracker {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        let HoverTracker {
            element,
            on_enter,
            on_exit,
        } = self;
        element
            .on_hover(move |hovered, window, cx| {
                let handler = if *hovered {
                    on_enter.as_ref()
                } else {
                    on_exit.as_ref()
                };
                if let Some(handler) = handler {
                    handler(&(), window, cx);
                }
            })
            .into_any_element()
    }
}

#[cfg(not(target_os = "macos"))]
const SIDEBAR_WIDTH_PX: f32 = 200.0;

// TODO: right-click tab menu is not ported yet. Glass used an AppKit popup whose single
// callback dispatches on an item index; `ui::ContextMenu` takes a handler per entry and needs
// menu state plus a dismiss subscription on `BrowserView`, so it is a restructure rather than a
// translation. Stubbed so the browser can be exercised; the menu offered pin/unpin, close,
// close others and bookmark.
fn show_tab_context_menu(
    view: WeakEntity<BrowserView>,
    index: usize,
    is_pinned: bool,
    position: Point<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    let _ = (view, index, is_pinned, position, window, cx);
}

#[cfg(not(target_os = "macos"))]
fn render_tab_favicon(id: SharedString, favicon_url: Option<&str>, _cx: &App) -> gpui::AnyElement {
    if let Some(url) = favicon_url {
        img(url.to_string())
            .size(px(14.))
            .rounded_sm()
            .flex_shrink_0()
            .into_any_element()
    } else {
        Icon::new(IconName::Public)
            .size(IconSize::Small)
            .into_any_element()
    }
}

impl BrowserView {
    #[cfg(not(target_os = "macos"))]
    pub(super) fn render_tab_strip(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let active_index = self.active_tab_index;
        let view = cx.entity().downgrade();

        let pinned_count = self.tabs.iter().filter(|t| t.read(cx).is_pinned()).count();

        h_flex()
            .w_full()
            .h(px(34.))
            .px_1()
            .gap_1()
            .items_center()
            .flex_shrink_0()
            // Pinned tabs dock
            .when(pinned_count > 0, |this| {
                this.child(
                    h_flex()
                        .items_center()
                        .gap_1()
                        .px_1()
                        .h(px(28.))
                        .rounded(px(8.0))
                        .bg(theme.colors().text.opacity(0.06))
                        .border_1()
                        .border_color(theme.colors().border.opacity(0.4))
                        .children(self.tabs.iter().enumerate().take(pinned_count).map(
                            |(index, tab)| {
                                let tab_data = tab.read(cx);
                                let favicon_url = tab_data.favicon_url();
                                let is_active = index == active_index;
                                let is_hovered = self.hovered_top_tab_index == Some(index);
                                let selected_bg = theme.colors().text.opacity(0.14);
                                let hover_bg = theme.colors().text.opacity(0.09);

                                let favicon_element = render_tab_favicon(
                                    SharedString::from(format!("browser-tab-favicon-{index}")),
                                    favicon_url,
                                    cx,
                                );

                                let hover_view = view.clone();
                                let context_view = view.clone();
                                div()
                                    .id(("browser-tab-inner", index))
                                    .relative()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .h(px(22.))
                                    .w(px(30.))
                                    .flex_shrink_0()
                                    .rounded(
                                        px(4.0),
                                    )
                                    .cursor_pointer()
                                    .when(is_active, |this| this.bg(selected_bg))
                                    .when(is_hovered && !is_active, |this| this.bg(hover_bg))
                                    .when(!is_active, |this| {
                                        this.hover(move |style| style.bg(hover_bg))
                                    })
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.switch_to_tab(index, window, cx);
                                    }))
                                    .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                                        show_tab_context_menu(
                                            context_view.clone(),
                                            index,
                                            true,
                                            event.position,
                                            window,
                                            cx,
                                        );
                                    })
                                    .child(favicon_element)
                                    .child(
                                        hover_tracker(format!("browser-tab-track-{index}"))
                                            .on_mouse_enter(move |_, _window, cx| {
                                                hover_view
                                                    .update(cx, |this, cx| {
                                                        if this.hovered_top_tab_index != Some(index)
                                                        {
                                                            this.hovered_top_tab_index =
                                                                Some(index);
                                                            cx.notify();
                                                        }
                                                    })
                                                    .ok();
                                            })
                                            .on_mouse_exit({
                                                let hover_view = view.clone();
                                                move |_, _window, cx| {
                                                    hover_view
                                                        .update(cx, |this, cx| {
                                                            if this.hovered_top_tab_index
                                                                == Some(index)
                                                            {
                                                                this.hovered_top_tab_index = None;
                                                                this.hovered_top_tab_close_index =
                                                                    None;
                                                                cx.notify();
                                                            }
                                                        })
                                                        .ok();
                                                }
                                            })
                                            .absolute()
                                            .top_0()
                                            .left_0()
                                            .size_full(),
                                    )
                                    .into_any_element()
                            },
                        )),
                )
            })
            // Unpinned tabs
            .children(
                self.tabs
                    .iter()
                    .enumerate()
                    .skip(pinned_count)
                    .map(|(index, tab)| {
                        let tab_data = tab.read(cx);
                        let title = tab_data.title().to_string();
                        let favicon_url = tab_data.favicon_url();
                        let is_pinned = tab_data.is_pinned();
                        let is_active = index == active_index;
                        let is_hovered = self.hovered_top_tab_index == Some(index);
                        let is_close_hovered = self.hovered_top_tab_close_index == Some(index);
                        let selected_bg = theme.colors().text.opacity(0.14);
                        let hover_bg = theme.colors().text.opacity(0.09);

                        let favicon_element = render_tab_favicon(
                            SharedString::from(format!("browser-tab-favicon-{index}")),
                            favicon_url,
                            cx,
                        );

                        let display_title = if title.len() > 24 {
                            let truncated = match title.char_indices().nth(21) {
                                Some((byte_index, _)) => &title[..byte_index],
                                None => &title,
                            };
                            format!("{truncated}...")
                        } else {
                            title
                        };

                        let hover_view = view.clone();
                        let context_view = view.clone();
                        div()
                            .id(("browser-tab-inner", index))
                            .relative()
                            .flex()
                            .items_center()
                            .h(px(24.))
                            .px_2()
                            .gap_1()
                            .min_w(px(92.))
                            .max_w(px(220.))
                            .rounded(px(8.0))
                            .cursor_pointer()
                            .when(is_active, |this| this.bg(selected_bg))
                            .when(is_hovered && !is_active, |this| this.bg(hover_bg))
                            .when(!is_active, |this| {
                                this.hover(move |style| style.bg(hover_bg))
                            })
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.switch_to_tab(index, window, cx);
                            }))
                            .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                                show_tab_context_menu(
                                    context_view.clone(),
                                    index,
                                    is_pinned,
                                    event.position,
                                    window,
                                    cx,
                                );
                            })
                            .child(favicon_element)
                            .child(
                                div()
                                    .flex_1()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .text_size(rems(0.75))
                                    .text_color(if is_active {
                                        theme.colors().text
                                    } else {
                                        theme.colors().text_muted
                                    })
                                    .child(display_title),
                            )
                            .when(is_hovered, |this| {
                                let close_hover_view = view.clone();
                                this.child(
                                    div()
                                        .id(SharedString::from(format!("close-tab-{index}")))
                                        .relative()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .w(px(16.))
                                        .h(px(16.))
                                        .rounded(
                                            px(4.0),
                                        )
                                        .cursor_pointer()
                                        .when(is_close_hovered, |this| this.bg(hover_bg))
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.close_tab_at(index, window, cx);
                                        }))
                                        .child(
                                            Icon::new(IconName::Close)
                                                .size(IconSize::XSmall),
                                        )
                                        .child(
                                            hover_tracker(format!(
                                                "close-tab-track-{index}"
                                            ))
                                            .on_mouse_enter(move |_, _window, cx| {
                                                close_hover_view
                                                    .update(cx, |this, cx| {
                                                        if this.hovered_top_tab_close_index
                                                            != Some(index)
                                                        {
                                                            this.hovered_top_tab_close_index =
                                                                Some(index);
                                                            cx.notify();
                                                        }
                                                    })
                                                    .ok();
                                            })
                                            .on_mouse_exit({
                                                let close_hover_view = view.clone();
                                                move |_, _window, cx| {
                                                    close_hover_view
                                                        .update(cx, |this, cx| {
                                                            if this.hovered_top_tab_close_index
                                                                == Some(index)
                                                            {
                                                                this.hovered_top_tab_close_index =
                                                                    None;
                                                                cx.notify();
                                                            }
                                                        })
                                                        .ok();
                                                }
                                            })
                                            .absolute()
                                            .top_0()
                                            .left_0()
                                            .size_full(),
                                        ),
                                )
                            })
                            .child(
                                hover_tracker(format!("browser-tab-track-{index}"))
                                    .on_mouse_enter(move |_, _window, cx| {
                                        hover_view
                                            .update(cx, |this, cx| {
                                                if this.hovered_top_tab_index != Some(index) {
                                                    this.hovered_top_tab_index = Some(index);
                                                    cx.notify();
                                                }
                                            })
                                            .ok();
                                    })
                                    .on_mouse_exit({
                                        let hover_view = view.clone();
                                        move |_, _window, cx| {
                                            hover_view
                                                .update(cx, |this, cx| {
                                                    if this.hovered_top_tab_index == Some(index) {
                                                        this.hovered_top_tab_index = None;
                                                        this.hovered_top_tab_close_index = None;
                                                        cx.notify();
                                                    }
                                                })
                                                .ok();
                                        }
                                    })
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .size_full(),
                            )
                            .into_any_element()
                    }),
            )
            .child(
                div()
                    .id("new-tab-button")
                    .relative()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(20.))
                    .h(px(20.))
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .when(self.hovered_top_new_tab_button, |this| {
                        this.bg(theme.colors().text.opacity(0.09))
                    })
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.add_tab(cx);
                        this.update_toolbar_active_tab(window, cx);
                        cx.notify();
                    }))
                    .child(
                        Icon::new(IconName::Plus).size(IconSize::XSmall),
                    )
                    .child(
                        hover_tracker("new-tab-button-track")
                            .on_mouse_enter({
                                let view = view.clone();
                                move |_, _window, cx| {
                                    view.update(cx, |this, cx| {
                                        if !this.hovered_top_new_tab_button {
                                            this.hovered_top_new_tab_button = true;
                                            cx.notify();
                                        }
                                    })
                                    .ok();
                                }
                            })
                            .on_mouse_exit({
                                let view = view.clone();
                                move |_, _window, cx| {
                                    view.update(cx, |this, cx| {
                                        if this.hovered_top_new_tab_button {
                                            this.hovered_top_new_tab_button = false;
                                            cx.notify();
                                        }
                                    })
                                    .ok();
                                }
                            })
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full(),
                    ),
            )
    }
}

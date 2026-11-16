//! A person's profile picture at full size, opened from the profile panel.
//! The picture is fetched once at `FETCH_PIXELS` through the same bounded
//! avatar decoder as the small avatars, and released when closed.
use std::sync::Arc;

use gpui_kit::base::TestSupportExt;
use gpui_kit::{
    component::{
        button::{Button, ButtonVariants},
        *,
    },
    *,
};

use crate::app::MatrixApp;
use crate::theme::scaled_px as px;

/// Physical pixels: sharp at the 400 pt display size on Retina.
const FETCH_PIXELS: u32 = 800;
/// Display size in points; smaller windows shrink it to fit.
const DISPLAY: f32 = 400.;

pub(crate) enum AvatarLoad {
    Loading,
    Ready(Arc<RenderImage>),
    Failed,
}

pub(crate) struct AvatarViewer {
    pub mxc: String,
    pub name: String,
    pub load: AvatarLoad,
    pub focus: FocusHandle,
}

impl MatrixApp {
    pub(crate) fn open_avatar_viewer(
        &mut self,
        mxc: String,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !mxc.starts_with("mxc://") {
            return;
        }
        self.close_avatar_viewer(window, cx);
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        self.avatar_viewer = Some(AvatarViewer {
            mxc: mxc.clone(),
            name,
            load: AvatarLoad::Loading,
            focus,
        });
        cx.notify();
        let Some(backend) = self.backend.clone() else {
            if let Some(viewer) = &mut self.avatar_viewer {
                viewer.load = AvatarLoad::Failed;
            }
            return;
        };
        cx.spawn(async move |this, cx| {
            let key = mxc.clone();
            let result = cx
                .background_spawn(async move { backend.avatar(key, FETCH_PIXELS) })
                .await;
            let _ = this.update(cx, |this, cx| {
                // A newer picture (or none) replaced this one meanwhile.
                let Some(viewer) = this.avatar_viewer.as_mut().filter(|v| v.mxc == mxc) else {
                    return;
                };
                viewer.load = result
                    .ok()
                    .and_then(|png| {
                        Image::from_bytes(ImageFormat::Png, png)
                            .to_image_data(cx.svg_renderer())
                            .ok()
                    })
                    .map_or(AvatarLoad::Failed, AvatarLoad::Ready);
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn close_avatar_viewer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(viewer) = self.avatar_viewer.take() else {
            return;
        };
        if let AvatarLoad::Ready(image) = viewer.load {
            window.drop_image(image).ok();
        }
        self.focus.focus(window, cx);
        cx.notify();
    }

    pub(crate) fn avatar_viewer_view(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let viewer = self.avatar_viewer.as_ref().expect("avatar viewer");
        let muted = cx.theme().muted_foreground;
        let viewport = window.viewport_size();
        // Leave room for the toolbar and a margin on every side.
        let side = DISPLAY
            .min(f32::from(viewport.width) - 48.)
            .min(f32::from(viewport.height) - 120.)
            .max(64.);
        let picture = match &viewer.load {
            AvatarLoad::Ready(image) => div()
                .id("avatar-viewer-image")
                .role(Role::Image)
                .aria_label(format!("Profile picture of {}", viewer.name))
                .size(px(side))
                .rounded(px(12.))
                .overflow_hidden()
                .child(img(image.clone()).size_full())
                .into_any_element(),
            AvatarLoad::Loading => div()
                .text_color(muted)
                .child("Loading picture…")
                .into_any_element(),
            AvatarLoad::Failed => div()
                .text_color(muted)
                .child("The picture couldn't be loaded.")
                .into_any_element(),
        };
        v_flex()
            .id("avatar-viewer")
            .role(Role::Dialog)
            .aria_label(format!("Profile picture of {}", viewer.name))
            .test_support()
            .absolute()
            .inset_0()
            .size_full()
            .occlude()
            .bg(cx.theme().background)
            .track_focus(&viewer.focus)
            .key_context("AvatarViewer")
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    this.close_avatar_viewer(window, cx);
                    cx.stop_propagation();
                }
            }))
            .capture_any_mouse_down(cx.listener(|this, event: &MouseDownEvent, window, cx| {
                if let MouseButton::Navigate(NavigationDirection::Back) = event.button {
                    this.close_avatar_viewer(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                self.toolbar("avatar-viewer-toolbar", cx)
                    .pl(px(if cfg!(target_os = "macos") { 96. } else { 16. }))
                    .pr(px(12.))
                    .gap(px(10.))
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(viewer.name.clone()),
                    )
                    .child(
                        Button::new("avatar-viewer-close")
                            .ghost()
                            .w(px(28.))
                            .h(px(28.))
                            .icon(assets::IconName::X)
                            .accessibility_label("Close picture")
                            .tooltip("Close  Escape")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.close_avatar_viewer(window, cx)
                            })),
                    ),
            )
            .child(
                div()
                    .id("avatar-viewer-canvas")
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    // A click anywhere closes, like the image viewer.
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, window, cx| this.close_avatar_viewer(window, cx)),
                    )
                    .child(picture),
            )
            .into_any_element()
    }
}

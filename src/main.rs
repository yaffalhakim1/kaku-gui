// Native GUI entry point for Kaku.
// This file only opens the application window. Everything else lives in src/app.rs.

mod app;
mod theme;

use gpui::*;
use gpui_platform::application;
use crate::app::KakuApp;

fn main() {
    application().run(|cx: &mut App| {
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(900.0), px(640.0)),
                    cx,
                ))),
                window_min_size: Some(size(px(600.0), px(400.0))),
                ..Default::default()
            },
            |window, cx| {
                let app = KakuApp::new(window, cx);
                let focus = app.focus_handle(cx).clone();
                window.focus(&focus, cx);
                app
            },
        )
        .expect("failed to open main window");
    });
}

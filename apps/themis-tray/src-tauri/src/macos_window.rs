//! macOS overlay transparency and mini-mode circular clipping.

use tauri::WebviewWindow;

#[cfg(target_os = "macos")]
pub fn apply_overlay_transparency(window: &WebviewWindow) {
    let _ = window.set_shadow(false);
    let _ = window.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));

    let _ = window.with_webview(|webview| {
        use cocoa::base::{id, nil, NO};
        use cocoa::foundation::NSString;
        use objc::{class, msg_send, sel, sel_impl};

        unsafe {
            let wk: id = webview.inner() as id;
            let key = NSString::alloc(nil).init_str("drawsBackground");
            let no: id = msg_send![class!(NSNumber), numberWithBool: NO];
            let _: () = msg_send![wk, setValue: no forKey: key];
        }
    });

    if let Ok(ns_window) = window.ns_window() {
        use cocoa::appkit::NSColor;
        use cocoa::base::{id, nil, NO};
        use objc::{msg_send, sel, sel_impl};

        unsafe {
            let ns_window = ns_window as id;
            let clear = NSColor::clearColor(nil);
            let _: () = msg_send![ns_window, setOpaque: NO];
            let _: () = msg_send![ns_window, setBackgroundColor: clear];
            let _: () = msg_send![ns_window, setHasShadow: NO];
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn apply_overlay_transparency(_window: &WebviewWindow) {}

#[cfg(target_os = "macos")]
pub fn set_mini_circular_clip(window: &WebviewWindow, active: bool) {
    let Ok(ns_window) = window.ns_window() else {
        return;
    };
    use cocoa::base::{id, nil, NO, YES};
    use objc::{msg_send, sel, sel_impl};

    unsafe {
        let ns_window = ns_window as id;
        let content_view: id = msg_send![ns_window, contentView];
        let _: () = msg_send![content_view, setWantsLayer: YES];
        let layer: id = msg_send![content_view, layer];
        if layer.is_null() {
            return;
        }

        if active {
            let bounds: cocoa::foundation::NSRect = msg_send![content_view, bounds];
            let side = bounds.size.width.min(bounds.size.height);
            let radius = side / 2.0;
            let _: () = msg_send![layer, setCornerRadius: radius];
            let _: () = msg_send![layer, setMasksToBounds: YES];
            let _: () = msg_send![layer, setMask: nil];
        } else {
            let _: () = msg_send![layer, setCornerRadius: 0.0f64];
            let _: () = msg_send![layer, setMasksToBounds: NO];
            let _: () = msg_send![layer, setMask: nil];
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_mini_circular_clip(_window: &WebviewWindow, _active: bool) {}

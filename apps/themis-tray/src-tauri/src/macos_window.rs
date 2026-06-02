//! macOS overlay transparency, mini-mode circular clipping, and floater elevation.

use tauri::WebviewWindow;

#[cfg(target_os = "macos")]
const NS_NORMAL_WINDOW_LEVEL: i64 = 0;
#[cfg(target_os = "macos")]
const NS_MAIN_MENU_WINDOW_LEVEL: i64 = 24;

#[cfg(target_os = "macos")]
const COLLECTION_CAN_JOIN_ALL_SPACES: u64 = 1 << 0;
#[cfg(target_os = "macos")]
const COLLECTION_MOVE_TO_ACTIVE_SPACE: u64 = 1 << 1;
#[cfg(target_os = "macos")]
const COLLECTION_MANAGED: u64 = 1 << 2;
#[cfg(target_os = "macos")]
const COLLECTION_STATIONARY: u64 = 1 << 4;
#[cfg(target_os = "macos")]
const COLLECTION_IGNORES_CYCLE: u64 = 1 << 6;
#[cfg(target_os = "macos")]
const COLLECTION_FULLSCREEN_PRIMARY: u64 = 1 << 7;
#[cfg(target_os = "macos")]
const COLLECTION_FULLSCREEN_AUXILIARY: u64 = 1 << 8;
#[cfg(target_os = "macos")]
const COLLECTION_FULLSCREEN_NONE: u64 = 1 << 9;
#[cfg(target_os = "macos")]
const COLLECTION_TRANSIENT: u64 = 1 << 3;
#[cfg(target_os = "macos")]
const COLLECTION_PRIMARY: u64 = 1 << 16;
#[cfg(target_os = "macos")]
const COLLECTION_AUXILIARY: u64 = 1 << 17;
#[cfg(target_os = "macos")]
const COLLECTION_CAN_JOIN_ALL_APPLICATIONS: u64 = 1 << 18;
#[cfg(target_os = "macos")]
const STYLE_MASK_NONACTIVATING_PANEL: u64 = 1 << 7;

#[cfg(target_os = "macos")]
fn mini_floater_collection_behavior(current: u64) -> u64 {
    let mut behavior = current;
    behavior &= !(
        COLLECTION_MOVE_TO_ACTIVE_SPACE
            | COLLECTION_MANAGED
            | COLLECTION_STATIONARY
            | COLLECTION_FULLSCREEN_PRIMARY
            | COLLECTION_FULLSCREEN_NONE
            | COLLECTION_PRIMARY
            | COLLECTION_AUXILIARY
    );
    behavior |= COLLECTION_CAN_JOIN_ALL_SPACES
        | COLLECTION_FULLSCREEN_AUXILIARY
        | COLLECTION_CAN_JOIN_ALL_APPLICATIONS
        | COLLECTION_IGNORES_CYCLE
        | COLLECTION_TRANSIENT;
    behavior
}

#[cfg(target_os = "macos")]
fn apply_macos_floater_objc(window: &WebviewWindow, elevated: bool) -> Result<(), String> {
    let ns_window = window.ns_window().map_err(|e| e.to_string())?;
    use cocoa::base::{id, nil, NO, YES};
    use objc::{msg_send, sel, sel_impl};

    unsafe {
        let ns_window = ns_window as id;
        if elevated {
            let current: u64 = msg_send![ns_window, collectionBehavior];
            let behavior = mini_floater_collection_behavior(current);
            let mask: u64 = msg_send![ns_window, styleMask];
            let _: () = msg_send![ns_window, setStyleMask: mask | STYLE_MASK_NONACTIVATING_PANEL];
            let _: () = msg_send![ns_window, setLevel: NS_MAIN_MENU_WINDOW_LEVEL];
            let _: () = msg_send![ns_window, setCollectionBehavior: behavior];
            let _: () = msg_send![ns_window, setHidesOnDeactivate: NO];
            let _: () = msg_send![ns_window, setHasShadow: YES];
            let _: () = msg_send![ns_window, orderFrontRegardless];
        } else {
            let _: () = msg_send![ns_window, setLevel: NS_NORMAL_WINDOW_LEVEL];
            let _: () = msg_send![ns_window, setCollectionBehavior: COLLECTION_CAN_JOIN_ALL_SPACES];
            let _: () = msg_send![ns_window, setHasShadow: NO];
        }
    }
    Ok(())
}

/// Raise the mini floater so it stays visible on every Space, including fullscreen apps.
#[cfg(target_os = "macos")]
pub fn set_macos_mini_floater_elevated(
    window: &WebviewWindow,
    elevated: bool,
) -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    window
        .run_on_main_thread({
            let window = window.clone();
            move || {
                let result = (|| {
                    if elevated {
                        window
                            .set_visible_on_all_workspaces(true)
                            .map_err(|e| e.to_string())?;
                        apply_macos_floater_objc(&window, true)?;
                        window.show().map_err(|e| e.to_string())?;
                    } else {
                        apply_macos_floater_objc(&window, false)?;
                        window
                            .set_visible_on_all_workspaces(false)
                            .map_err(|e| e.to_string())?;
                    }
                    Ok::<(), String>(())
                })();
                let _ = tx.send(result);
            }
        })
        .map_err(|e| e.to_string())?;
    rx.recv()
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn set_macos_mini_floater_elevated(
    window: &WebviewWindow,
    elevated: bool,
) -> Result<(), String> {
    if elevated {
        window
            .set_visible_on_all_workspaces(true)
            .map_err(|e| e.to_string())?;
        window
            .set_always_on_top(true)
            .map_err(|e| e.to_string())
    } else {
        window
            .set_visible_on_all_workspaces(false)
            .map_err(|e| e.to_string())?;
        window
            .set_always_on_top(false)
            .map_err(|e| e.to_string())
    }
}

#[cfg(target_os = "macos")]
pub fn apply_overlay_topmost(
    window: &WebviewWindow,
    mini_mode: bool,
    always_on_top: bool,
) -> Result<(), String> {
    if mini_mode {
        Ok(())
    } else {
        window
            .set_always_on_top(always_on_top)
            .map_err(|e| e.to_string())
    }
}

#[cfg(not(target_os = "macos"))]
pub fn apply_overlay_topmost(
    window: &WebviewWindow,
    mini_mode: bool,
    always_on_top: bool,
) -> Result<(), String> {
    if mini_mode {
        Ok(())
    } else {
        window
            .set_always_on_top(always_on_top)
            .map_err(|e| e.to_string())
    }
}

#[cfg(target_os = "macos")]
pub fn apply_overlay_transparency(window: &WebviewWindow) {
    let _ = window.set_shadow(false);
    let _ = window.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));

    let _ = window.run_on_main_thread({
        let window = window.clone();
        move || {
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
    });
}

#[cfg(not(target_os = "macos"))]
pub fn apply_overlay_transparency(window: &WebviewWindow) {
    #[cfg(windows)]
    crate::windows_window::apply_overlay_transparency(window);
}

#[cfg(target_os = "macos")]
pub fn set_mini_circular_clip(window: &WebviewWindow, active: bool) {
    let Ok(()) = window.run_on_main_thread({
        let window = window.clone();
        move || {
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
    }) else {
        return;
    };
}

#[cfg(not(target_os = "macos"))]
pub fn set_mini_circular_clip(window: &WebviewWindow, active: bool) {
    #[cfg(windows)]
    crate::windows_window::set_mini_circular_clip(window, active);
}

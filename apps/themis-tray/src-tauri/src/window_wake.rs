//! Bring the overlay capture window to the front and keep it on top.

use tauri::WebviewWindow;

pub fn wake_overlay_window(window: &WebviewWindow) -> Result<(), String> {
    if window.is_minimized().map_err(|e| e.to_string())? {
        window.unminimize().map_err(|e| e.to_string())?;
    }
    window.show().map_err(|e| e.to_string())?;
    window.set_always_on_top(true).map_err(|e| e.to_string())?;
    platform_bring_to_front(window)?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn platform_bring_to_front(window: &WebviewWindow) -> Result<(), String> {
    use cocoa::base::{id, nil, YES};
    use objc::{class, msg_send, sel, sel_impl};

    unsafe {
        let app: id = msg_send![class!(NSApplication), sharedApplication];
        let _: () = msg_send![app, activateIgnoringOtherApps: YES];
    }

    if let Ok(ns_window) = window.ns_window() {
        use objc::{msg_send, sel, sel_impl};
        unsafe {
            let ns_window = ns_window as id;
            let _: () = msg_send![ns_window, makeKeyAndOrderFront: nil];
            let _: () = msg_send![ns_window, orderFrontRegardless];
        }
    }
    Ok(())
}

#[cfg(windows)]
fn platform_bring_to_front(window: &WebviewWindow) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW};

    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    unsafe {
        let hwnd = HWND(hwnd.0 as _);
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
    Ok(())
}

#[cfg(not(any(target_os = "macos", windows)))]
fn platform_bring_to_front(_window: &WebviewWindow) -> Result<(), String> {
    Ok(())
}

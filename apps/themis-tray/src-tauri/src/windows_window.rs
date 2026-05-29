//! Windows overlay transparency and mini floater circular clip.

use tauri::WebviewWindow;
use windows_core::Interface;

pub fn apply_overlay_transparency(window: &WebviewWindow) {
    let _ = window.set_shadow(false);
    let _ = window.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));

    let w = window.clone();
    let _ = w.clone().run_on_main_thread(move || {
        let _ = w.with_webview(|webview| {
            use webview2_com::Microsoft::Web::WebView2::Win32::{
                ICoreWebView2Controller2, COREWEBVIEW2_COLOR,
            };

            unsafe {
                let controller = webview.controller();
                let Ok(controller2) = controller.cast::<ICoreWebView2Controller2>() else {
                    return;
                };
                let transparent = COREWEBVIEW2_COLOR {
                    A: 0,
                    R: 0,
                    G: 0,
                    B: 0,
                };
                let _ = controller2.SetDefaultBackgroundColor(transparent);
            }
        });
    });
}

pub fn set_mini_circular_clip(window: &WebviewWindow, active: bool) {
    let window = window.clone();
    let _ = window.clone().run_on_main_thread(move || {
        apply_circular_region(&window, active);
    });
}

fn apply_circular_region(window: &WebviewWindow, active: bool) {
    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    let hwnd = windows::Win32::Foundation::HWND(hwnd.0);

    unsafe {
        use windows::Win32::Foundation::RECT;
        use windows::Win32::Graphics::Gdi::{CreateEllipticRgn, SetWindowRgn, HRGN};
        use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

        if !active {
            let _ = SetWindowRgn(hwnd, HRGN::default(), true);
            return;
        }

        let mut rect = RECT::default();
        if GetClientRect(hwnd, &mut rect).is_err() {
            return;
        }
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        if w <= 0 || h <= 0 {
            return;
        }
        let rgn = CreateEllipticRgn(0, 0, w, h);
        if !rgn.is_invalid() {
            let _ = SetWindowRgn(hwnd, rgn, true);
        }
    }
}

//! Native macOS NSPanel floater (Helium / Doubao-style) — visible on all Spaces incl. fullscreen.

use cocoa::appkit::{NSBackingStoreType, NSColor};
use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSPoint, NSRect, NSSize, NSString};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use std::cell::Cell;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter, Manager};

const PANEL_SIZE: f64 = 52.0;
const NS_SCREEN_SAVER_WINDOW_LEVEL: i64 = 1000;
const STYLE_BORDERLESS: u64 = 0;
const STYLE_NONACTIVATING_PANEL: u64 = 1 << 7;
const COLLECTION_CAN_JOIN_ALL_SPACES: u64 = 1 << 0;
const COLLECTION_TRANSIENT: u64 = 1 << 3;
const COLLECTION_IGNORES_CYCLE: u64 = 1 << 6;
const COLLECTION_FULLSCREEN_AUXILIARY: u64 = 1 << 8;
const COLLECTION_CAN_JOIN_ALL_APPLICATIONS: u64 = 1 << 18;

thread_local! {
    static DRAG_START: Cell<Option<(f64, f64)>> = Cell::new(None);
    static DID_DRAG: Cell<bool> = Cell::new(false);
    static PANEL: Cell<Option<id>> = Cell::new(None);
    static SPACE_OBSERVER: Cell<Option<id>> = Cell::new(None);
}

static PANEL_APP: Mutex<Option<AppHandle>> = Mutex::new(None);

static FLOATER_VIEW_CLASS: OnceLock<&'static Class> = OnceLock::new();
static FLOATER_PANEL_CLASS: OnceLock<&'static Class> = OnceLock::new();
static SPACE_OBSERVER_CLASS: OnceLock<&'static Class> = OnceLock::new();

pub fn install_panel_app(app: AppHandle) {
    if let Ok(mut guard) = PANEL_APP.lock() {
        *guard = Some(app);
    }
}

pub fn set_accessory_activation_policy() {
    unsafe {
        let app: id = msg_send![class!(NSApplication), sharedApplication];
        let _: i64 = msg_send![app, setActivationPolicy: 1_i64];
    }
}

fn floater_panel_class() -> &'static Class {
    FLOATER_PANEL_CLASS.get_or_init(|| {
        let mut decl = ClassDecl::new("ThemisFloaterPanel", class!(NSPanel))
            .expect("register ThemisFloaterPanel");
        unsafe {
            decl.add_method(
                sel!(isFloatingPanel),
                is_floating_panel_yes as extern "C" fn(&Object, Sel) -> cocoa::base::BOOL,
            );
        }
        decl.register()
    })
}

fn floater_view_class() -> &'static Class {
    FLOATER_VIEW_CLASS.get_or_init(|| {
        let mut decl = ClassDecl::new("ThemisFloaterView", class!(NSView))
            .expect("register ThemisFloaterView");
        unsafe {
            decl.add_method(
                sel!(isFlipped),
                is_flipped_yes as extern "C" fn(&Object, Sel) -> cocoa::base::BOOL,
            );
            decl.add_method(
                sel!(mouseDown:),
                mouse_down as extern "C" fn(&Object, Sel, id),
            );
            decl.add_method(
                sel!(mouseDragged:),
                mouse_dragged as extern "C" fn(&Object, Sel, id),
            );
            decl.add_method(
                sel!(mouseUp:),
                mouse_up as extern "C" fn(&Object, Sel, id),
            );
            decl.add_method(
                sel!(drawRect:),
                draw_rect as extern "C" fn(&Object, Sel, NSRect),
            );
        }
        decl.register()
    })
}

fn space_observer_class() -> &'static Class {
    SPACE_OBSERVER_CLASS.get_or_init(|| {
        let mut decl = ClassDecl::new("ThemisSpaceObserver", class!(NSObject))
            .expect("register ThemisSpaceObserver");
        unsafe {
            decl.add_method(
                sel!(spaceChanged:),
                space_changed as extern "C" fn(&Object, Sel, id),
            );
            decl.add_method(
                sel!(appActivated:),
                app_activated as extern "C" fn(&Object, Sel, id),
            );
        }
        decl.register()
    })
}

extern "C" fn is_floating_panel_yes(_: &Object, _: Sel) -> cocoa::base::BOOL {
    YES
}

extern "C" fn is_flipped_yes(_: &Object, _: Sel) -> cocoa::base::BOOL {
    YES
}

extern "C" fn mouse_down(_: &Object, _: Sel, event: id) {
    unsafe {
        let loc: NSPoint = msg_send![event, locationInWindow];
        DRAG_START.set(Some((loc.x, loc.y)));
        DID_DRAG.set(false);
    }
}

extern "C" fn mouse_dragged(this: &Object, _: Sel, event: id) {
    unsafe {
        if let Some((sx, sy)) = DRAG_START.get() {
            let loc: NSPoint = msg_send![event, locationInWindow];
            let dx = loc.x - sx;
            let dy = loc.y - sy;
            if (dx * dx + dy * dy).sqrt() > 4.0 {
                DID_DRAG.set(true);
                let window: id = msg_send![this, window];
                let _: () = msg_send![window, performWindowDragWithEvent: event];
            }
        }
    }
}

extern "C" fn mouse_up(_: &Object, _: Sel, _: id) {
    if !DID_DRAG.get() {
        panel_clicked();
    }
    DRAG_START.set(None);
    DID_DRAG.set(false);
}

extern "C" fn draw_rect(_: &Object, _: Sel, _: NSRect) {
    unsafe {
        let bounds = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(PANEL_SIZE, PANEL_SIZE));
        let oval: id = msg_send![class!(NSBezierPath), bezierPathWithOvalInRect: bounds];
        let fill: id = NSColor::colorWithCalibratedRed_green_blue_alpha_(nil, 0.10, 0.28, 0.50, 0.96);
        let _: () = msg_send![fill, setFill];
        let _: () = msg_send![oval, fill];
        let stroke: id = NSColor::colorWithCalibratedRed_green_blue_alpha_(nil, 0.55, 0.82, 1.0, 0.75);
        let _: () = msg_send![stroke, setStroke];
        let _: () = msg_send![oval, setLineWidth: 1.5f64];
        let _: () = msg_send![oval, stroke];

        let title = NSString::alloc(nil).init_str("⚖");
        let font: id = msg_send![class!(NSFont), systemFontOfSize: 24.0f64];
        let font_key = NSString::alloc(nil).init_str("NSFont");
        let attrs: id =
            msg_send![class!(NSDictionary), dictionaryWithObject: font forKey: font_key];
        let size: NSSize = msg_send![title, sizeWithAttributes: attrs];
        let point = NSPoint::new(
            (PANEL_SIZE - size.width) / 2.0,
            (PANEL_SIZE - size.height) / 2.0,
        );
        let _: () = msg_send![title, drawAtPoint: point withAttributes: attrs];
    }
}

extern "C" fn space_changed(_: &Object, _: Sel, _: id) {
    unsafe {
        bring_panel_to_front();
    }
}

extern "C" fn app_activated(_: &Object, _: Sel, _: id) {
    unsafe {
        bring_panel_to_front();
    }
}

fn panel_clicked() {
    let app = match PANEL_APP.lock() {
        Ok(guard) => guard.clone(),
        Err(_) => None,
    };
    let Some(app) = app else { return };
    let _ = app.emit("mini-panel-clicked", ());
}

fn panel_collection_behavior() -> u64 {
    COLLECTION_CAN_JOIN_ALL_SPACES
        | COLLECTION_FULLSCREEN_AUXILIARY
        | COLLECTION_CAN_JOIN_ALL_APPLICATIONS
        | COLLECTION_IGNORES_CYCLE
        | COLLECTION_TRANSIENT
}

fn top_left_to_ns_rect(x: f64, y_top: f64) -> NSRect {
    unsafe {
        let screen: id = msg_send![class!(NSScreen), mainScreen];
        let screen_frame: NSRect = msg_send![screen, frame];
        let y = screen_frame.size.height - y_top - PANEL_SIZE;
        NSRect::new(NSPoint::new(x, y), NSSize::new(PANEL_SIZE, PANEL_SIZE))
    }
}

unsafe fn apply_panel_behavior(panel: id) {
    let behavior = panel_collection_behavior();
    let _: () = msg_send![panel, setLevel: NS_SCREEN_SAVER_WINDOW_LEVEL];
    let _: () = msg_send![panel, setCollectionBehavior: behavior];
    let _: () = msg_send![panel, setFloatingPanel: YES];
    let _: () = msg_send![panel, setWorksWhenModal: YES];
    let _: () = msg_send![panel, setBecomesKeyOnlyIfNeeded: YES];
    let _: () = msg_send![panel, setHidesOnDeactivate: NO];
    let _: () = msg_send![panel, setMovableByWindowBackground: NO];
    let _: () = msg_send![panel, setOpaque: NO];
    let clear = NSColor::clearColor(nil);
    let _: () = msg_send![panel, setBackgroundColor: clear];
    let _: () = msg_send![panel, setHasShadow: YES];
    let _: () = msg_send![panel, setIgnoresMouseEvents: NO];
    let _: () = msg_send![panel, setReleasedWhenClosed: NO];
    let _: () = msg_send![panel, setDisplaysWhenScreenProfileChanges: YES];
}

unsafe fn bring_panel_to_front() {
    if let Some(panel) = PANEL.get() {
        apply_panel_behavior(panel);
        let _: () = msg_send![panel, setIsVisible: YES];
        let _: () = msg_send![panel, orderFrontRegardless];
    }
}

unsafe fn add_observer(
    center: id,
    observer: id,
    selector: Sel,
    name: &str,
    object: id,
) {
    let name = NSString::alloc(nil).init_str(name);
    let _: () = msg_send![center, addObserver: observer selector:selector name:name object:object];
}

unsafe fn ensure_space_observer() {
    if SPACE_OBSERVER.get().is_some() {
        return;
    }
    let observer_cls = space_observer_class();
    let observer: id = msg_send![observer_cls, new];
    let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
    let center: id = msg_send![workspace, notificationCenter];
    add_observer(
        center,
        observer,
        sel!(spaceChanged:),
        "NSWorkspaceActiveSpaceDidChangeNotification",
        workspace,
    );
    add_observer(
        center,
        observer,
        sel!(appActivated:),
        "NSWorkspaceDidActivateApplicationNotification",
        workspace,
    );
    let nc: id = msg_send![class!(NSNotificationCenter), defaultCenter];
    add_observer(
        nc,
        observer,
        sel!(spaceChanged:),
        "NSApplicationDidChangeScreenParametersNotification",
        nil,
    );
    SPACE_OBSERVER.set(Some(observer));
}

unsafe fn create_panel(frame: NSRect) -> id {
    let panel_cls = floater_panel_class();
    let panel: id = msg_send![panel_cls, alloc];
    let style = STYLE_BORDERLESS | STYLE_NONACTIVATING_PANEL;
    let panel: id = msg_send![
        panel,
        initWithContentRect: frame
        styleMask: style
        backing: NSBackingStoreType::NSBackingStoreBuffered
        defer: NO
    ];
    apply_panel_behavior(panel);

    let view_cls = floater_view_class();
    let view: id = msg_send![view_cls, alloc];
    let view: id = msg_send![view, initWithFrame: frame];
    let _: () = msg_send![panel, setContentView: view];
    panel
}

unsafe fn show_panel_at(x: f64, y_top: f64) {
    let frame = top_left_to_ns_rect(x, y_top);
    let panel = if let Some(existing) = PANEL.get() {
        let _: () = msg_send![existing, setFrame: frame display: YES animate: NO];
        existing
    } else {
        let panel = create_panel(frame);
        PANEL.set(Some(panel));
        panel
    };
    ensure_space_observer();
    bring_panel_to_front();
}

unsafe fn hide_panel() {
    if let Some(panel) = PANEL.take() {
        let _: () = msg_send![panel, orderOut: nil];
        let _: () = msg_send![panel, release];
    }
}

unsafe fn is_on_main_thread() -> bool {
    let is_main: cocoa::base::BOOL = msg_send![class!(NSThread), isMainThread];
    is_main == YES
}

fn with_main_thread<F>(f: F) -> Result<(), String>
where
    F: FnOnce() + Send + 'static,
{
    if unsafe { is_on_main_thread() } {
        f();
        return Ok(());
    }
    let app = PANEL_APP
        .lock()
        .map_err(|_| "panel app lock poisoned".to_string())?
        .clone()
        .ok_or_else(|| "panel app not installed".to_string())?;
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        f();
        let _ = tx.send(());
    })
    .map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn show_mini_panel(x: f64, y_top: f64) -> Result<(), String> {
    with_main_thread(move || unsafe {
        show_panel_at(x, y_top);
    })
}

pub fn hide_mini_panel() -> Result<(), String> {
    with_main_thread(|| unsafe {
        hide_panel();
    })
}

pub fn refresh_mini_panel() -> Result<(), String> {
    with_main_thread(|| unsafe {
        bring_panel_to_front();
    })
}

pub fn is_mini_panel_visible() -> bool {
    PANEL.get().map_or(false, |panel| unsafe {
        let visible: bool = msg_send![panel, isVisible];
        visible
    })
}

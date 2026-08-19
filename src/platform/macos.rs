//! macOS "Open With" support.
//!
//! When a file is opened through Finder — the "Open With" menu, or a
//! double-click on an associated type — macOS does not pass the path in
//! `argv`. A bundled app receives an `'aevt'`/`'odoc'` Apple Event instead,
//! and an app that only reads `std::env::args()` silently ignores it. That is
//! why this used to be a no-op returning an empty `Vec`: the file
//! associations in `assets/macos/info_plist_ext.xml` put the app in the "Open
//! With" menu, but picking it there did nothing.
//!
//! # Why not an `NSApplicationDelegate`
//!
//! The obvious implementation is `application:openURLs:` on the app delegate.
//! It cannot be used here. winit 0.30 installs its own delegate inside
//! `EventLoop::new` and then asserts on it:
//!
//! ```text
//! // winit-0.30.13/src/platform_impl/macos/app_state.rs
//! let delegate = unsafe { app.delegate() }.expect("a delegate was not configured...");
//! if delegate.is_kind_of::<Self>() { ... } else {
//!     panic!("tried to get a delegate that was not the one Winit has registered")
//! }
//! ```
//!
//! `ApplicationDelegate::get` runs on every window creation, so replacing the
//! delegate — before or after eframe starts — panics. (winit's own
//! `platform::macos` docs suggest registering one and say winit never will;
//! that describes a later release than the 0.30.13 this depends on.)
//!
//! So the event is taken one level lower, from `NSAppleEventManager`, which
//! dispatches to any object without going near `NSApp`'s delegate. The timing
//! of that registration turns out to matter as much as the mechanism — see
//! [`init_app_delegate`].
//!
//! # Flow
//!
//! Paths land in a queue rather than being opened directly, because the
//! handler fires on the main thread from inside AppKit with no access to the
//! app state. [`get_open_file_paths`] drains that queue, and
//! `Skrivr::handle_instance_paths` polls it every frame alongside the paths
//! forwarded by secondary instances — the same code that already handles a
//! double-click while the app is running.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Four-char codes from `AppleEvents.h` / `AEDataModel.h`.
mod codes {
    /// `'aevt'` — `kCoreEventClass`.
    pub const CORE_EVENT_CLASS: u32 = 0x6165_7674;
    /// `'odoc'` — `kAEOpenDocuments`.
    pub const OPEN_DOCUMENTS: u32 = 0x6f64_6f63;
    /// `'----'` — `keyDirectObject`, where the document list lives.
    pub const DIRECT_OBJECT: u32 = 0x2d2d_2d2d;
    /// `'furl'` — `typeFileURL`. Finder may send aliases or bookmarks
    /// instead, so each item is coerced to this before being read.
    pub const FILE_URL: u32 = 0x6675_726c;
}

fn pending_paths() -> &'static Mutex<Vec<PathBuf>> {
    static PENDING: OnceLock<Mutex<Vec<PathBuf>>> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(Vec::new()))
}

fn repaint_ctx() -> &'static Mutex<Option<eframe::egui::Context>> {
    static CTX: OnceLock<Mutex<Option<eframe::egui::Context>>> = OnceLock::new();
    CTX.get_or_init(|| Mutex::new(None))
}

/// Give the Apple Event handler a way to wake the UI.
///
/// Without this an event that arrives while the app is idle sits in the queue
/// until something else causes a repaint.
pub fn set_repaint_ctx(ctx: &eframe::egui::Context) {
    if let Ok(mut slot) = repaint_ctx().lock() {
        if slot.is_none() {
            *slot = Some(ctx.clone());
        }
    }
}

/// Take the paths delivered by "Open With" since the last call.
pub fn get_open_file_paths() -> Vec<PathBuf> {
    match pending_paths().lock() {
        Ok(mut queue) => std::mem::take(&mut *queue),
        Err(_) => Vec::new(),
    }
}

fn queue_paths(paths: Vec<PathBuf>) {
    if paths.is_empty() {
        return;
    }
    log::info!("Apple Event delivered {} path(s)", paths.len());
    if let Ok(mut queue) = pending_paths().lock() {
        queue.extend(paths);
    }
    if let Ok(slot) = repaint_ctx().lock() {
        if let Some(ctx) = slot.as_ref() {
            ctx.request_repaint();
        }
    }
}

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{define_class, msg_send, sel, AllocAnyThread};
use objc2_foundation::{
    NSAppleEventDescriptor, NSAppleEventManager, NSNotificationCenter, NSObjectProtocol, NSString,
};

define_class!(
    // SAFETY:
    // - `NSObject` has no subclassing requirements.
    // - `OpenDocumentsHandler` holds no state and does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "SkrivrOpenDocumentsHandler"]
    struct OpenDocumentsHandler;

    unsafe impl NSObjectProtocol for OpenDocumentsHandler {}

    impl OpenDocumentsHandler {
        #[unsafe(method(handleAppleEvent:withReplyEvent:))]
        fn handle_open_documents(
            &self,
            event: &NSAppleEventDescriptor,
            _reply: &NSAppleEventDescriptor,
        ) {
            queue_paths(paths_from_event(event));
        }

        #[unsafe(method(applicationWillFinishLaunching:))]
        fn application_will_finish_launching(&self, _notification: &NSObject) {
            self.install_handler();
        }
    }
);

/// Pull the document list out of an `'odoc'` event.
///
/// The list is one-indexed, and a bad item is skipped rather than aborting the
/// batch: dragging five files onto the icon should open the four that resolve
/// even if one does not.
fn paths_from_event(event: &NSAppleEventDescriptor) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // `paramDescriptorForKeyword:` and `coerceToDescriptorType:` are gated
    // behind `objc2-core-services` in the generated bindings, purely because
    // their arguments are typed as the `AEKeyword`/`DescType` aliases. Sent by
    // hand to keep that dependency out of the tree — the codes are plain
    // `u32`s either way.
    let direct: Option<Retained<NSAppleEventDescriptor>> =
        unsafe { msg_send![event, paramDescriptorForKeyword: codes::DIRECT_OBJECT] };
    let Some(direct) = direct else {
        log::warn!("'odoc' event carried no direct object");
        return paths;
    };

    for index in 1..=direct.numberOfItems() {
        let Some(item) = direct.descriptorAtIndex(index) else {
            continue;
        };
        let url_descriptor: Option<Retained<NSAppleEventDescriptor>> =
            unsafe { msg_send![&*item, coerceToDescriptorType: codes::FILE_URL] };
        let Some(url) = url_descriptor.and_then(|descriptor| descriptor.fileURLValue()) else {
            continue;
        };
        if let Some(path) = url.path() {
            paths.push(PathBuf::from(path.to_string()));
        }
    }

    paths
}

impl OpenDocumentsHandler {
    /// Claim `'aevt'`/`'odoc'` for this object.
    fn install_handler(&self) {
        let manager = NSAppleEventManager::sharedAppleEventManager();
        unsafe {
            let _: () = msg_send![
                &*manager,
                setEventHandler: self,
                andSelector: sel!(handleAppleEvent:withReplyEvent:),
                forEventClass: codes::CORE_EVENT_CLASS,
                andEventID: codes::OPEN_DOCUMENTS,
            ];
        }
    }
}

/// Arrange for the `'odoc'` handler to be installed.
///
/// Note the indirection: this observes a notification rather than registering
/// the event handler outright. Registering here does not survive. AppKit
/// claims `'aevt'`/`'odoc'` for itself during `-finishLaunching`, last
/// registration wins, and AppKit's copy forwards to `application:openFiles:`
/// on the app delegate — which is winit's delegate, which does not implement
/// it. Anything AppKit takes is therefore dropped on the floor, and Finder
/// puts up "Kotori Skrivr cannot open files in the Markdown Document format".
/// Registering before AppKit reliably reproduces exactly that.
///
/// `applicationWillFinishLaunching:` is Apple's documented place to claim
/// Apple Events, and it runs after AppKit has installed its own — so that is
/// where the handler actually goes. The notification is observable without
/// owning the delegate, which is what makes this work alongside winit.
///
/// Called from `main` before eframe starts, because the notification is posted
/// during eframe's startup and an observer added later would miss it.
///
/// `NSAppleEventManager` and `NSNotificationCenter` both hold their observers
/// weakly, so the object is leaked on purpose: it has to outlive every event,
/// which means the life of the process.
pub fn init_app_delegate() {
    let handler = OpenDocumentsHandler::alloc();
    let handler: Retained<OpenDocumentsHandler> = unsafe { msg_send![handler, init] };

    // The AppKit notification names are literally their symbol names, so this
    // needs no dependency on objc2-app-kit.
    unsafe {
        NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
            &handler,
            sel!(applicationWillFinishLaunching:),
            Some(&NSString::from_str("NSApplicationWillFinishLaunchingNotification")),
            None,
        );
    }

    std::mem::forget(handler);
    log::debug!("scheduled 'odoc' Apple Event handler registration");
}

use arboard::Clipboard;
use drasyl_sdn::rest_api;
use drasyl_sdn::rest_api::Status;
use std::sync::{Arc, Mutex};
use tokio::time::{self, Duration};
use tracing::warn;
use tray_icon::{
    TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};
use winit::{application::ApplicationHandler, event_loop::EventLoop};

#[allow(clippy::large_enum_variant)]
enum UserEvent {
    TrayIconEvent(TrayIconEvent),
    MenuEvent(MenuEvent),
    Status(Result<Status, String>),
}

#[derive(Default)]
struct DrasylUiInner {
    tray_icon: Option<TrayIcon>,
    address_item: Option<MenuItem>,
    status: Option<Result<Status, String>>,
}

#[derive(Default)]
struct DrasylUi {
    inner: Arc<Mutex<DrasylUiInner>>,
}

impl DrasylUi {
    fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    fn new_tray_icon(&mut self) -> TrayIcon {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/tray-icon.png");
        let icon = load_icon(std::path::Path::new(path));

        TrayIconBuilder::new()
            .with_menu(Box::new(Self::new_tray_menu(self.inner.clone())))
            .with_tooltip("drasyl")
            .with_icon(icon)
            .with_icon_as_template(true)
            .build()
            .unwrap()
    }

    fn new_tray_menu(inner: Arc<Mutex<DrasylUiInner>>) -> Menu {
        let menu = Menu::new();

        // address
        let item = MenuItem::new(
            "Waiting for drasyl service to become available…",
            false,
            None,
        );
        if let Err(e) = menu.append(&item) {
            panic!("{e:?}");
        }
        inner.lock().expect("Mutex poisoned").address_item = Some(item);

        // separator
        if let Err(e) = menu.append(&PredefinedMenuItem::separator()) {
            panic!("{e:?}");
        }

        // quit
        if let Err(e) = menu.append(&PredefinedMenuItem::quit(Some("Quit drasyl UI"))) {
            panic!("{e:?}");
        }

        menu
    }
}

impl ApplicationHandler<UserEvent> for DrasylUi {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        _event: winit::event::WindowEvent,
    ) {
    }

    fn new_events(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        // We create the icon once the event loop is actually running
        // to prevent issues like https://github.com/tauri-apps/tray-icon/issues/90
        if winit::event::StartCause::Init == cause {
            #[cfg(not(target_os = "linux"))]
            {
                self.inner.lock().expect("Mutex poisoned").tray_icon = Some(self.new_tray_icon());
            }

            // We have to request a redraw here to have the icon actually show up.
            // Winit only exposes a redraw method on the Window so we use core-foundation directly.
            #[cfg(target_os = "macos")]
            {
                use objc2_core_foundation::CFRunLoop;

                let rl = CFRunLoop::main().unwrap();
                rl.wake_up();
            }
        }
    }

    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::MenuEvent(menu_event) => {
                let guard = self.inner.lock().expect("Mutex poisoned");
                if let Some(menu) = guard.address_item.as_ref() {
                    if menu_event.id == menu.id() {
                        if let Some(Ok(status)) = guard.status.as_ref() {
                            if let Ok(mut clipboard) = Clipboard::new() {
                                let _ = clipboard.set_text(status.opts.id.pk.to_string());
                            }
                        }
                    }
                }
            }
            UserEvent::Status(result) => {
                let mut guard = self.inner.lock().expect("Mutex poisoned");
                if let Some(menu) = guard.address_item.as_mut() {
                    match &result {
                        Ok(status) => {
                            let pk = status.opts.id.pk;
                            menu.set_text(format!("Public key: {}", pk));
                            menu.set_enabled(true);
                        }
                        Err(e) => {
                            menu.set_text(e);
                            menu.set_enabled(false);
                        }
                    }
                }

                guard.status = Some(result);
            }
            _ => {}
        }
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();

    // set a tray event handler that forwards the event and wakes up the event loop
    let proxy = event_loop.create_proxy();
    TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::TrayIconEvent(event));
    }));
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::MenuEvent(event));
    }));

    // Starte Tokio Runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    // Starte Background-Job
    let proxy = event_loop.create_proxy();
    rt.spawn(async move {
        let mut interval = time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;

            let client = rest_api::RestApiClient::new();
            match client.status().await {
                Ok(status) => {
                    let _ = proxy.send_event(UserEvent::Status(Ok(status)));
                }
                Err(e) => {
                    warn!("Failed to retrieve status: {}", e);
                    let _ = proxy.send_event(UserEvent::Status(Err(e.to_string())));
                }
            }
        }
    });

    let mut app = DrasylUi::new();

    // Since winit doesn't use gtk on Linux, and we need gtk for
    // the tray icon to show up, we need to spawn a thread
    // where we initialize gtk and create the tray_icon
    #[cfg(target_os = "linux")]
    {
        let inner = app.inner.clone();
        std::thread::spawn(|| {
            gtk::init().unwrap();

            let _tray_icon = DrasylUI::new_tray_icon(inner);

            gtk::main();
        });
    }

    if let Err(err) = event_loop.run_app(&mut app) {
        println!("Error: {:?}", err);
    }
}

fn load_icon(path: &std::path::Path) -> tray_icon::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}

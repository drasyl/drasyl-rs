use tray_icon::menu::MenuId;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder, TrayIconEvent,
};
use winit::{
    application::ApplicationHandler
    ,
    event_loop::EventLoop,
};
use tokio::time::{self, Duration};
use drasyl_sdn::rest_api::load_auth_token;

#[derive(Debug)]
enum UserEvent {
    TrayIconEvent(TrayIconEvent),
    MenuEvent(MenuEvent),
}

struct DrasylUI {
    tray_icon: Option<TrayIcon>,
    quit_id: Option<MenuId>,
}

impl DrasylUI {
    fn new() -> DrasylUI {
        DrasylUI {
            tray_icon: None,
            quit_id: None,
        }
    }

    fn new_tray_icon(&mut self) -> TrayIcon {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/dark-icon.png");
        let icon = load_icon(std::path::Path::new(path));

        TrayIconBuilder::new()
            .with_menu(Box::new(self.new_tray_menu()))
            .with_tooltip("drasyl")
            .with_icon(icon)
            .build()
            .unwrap()
    }

    fn new_tray_menu(&mut self) -> Menu {
        let menu = Menu::new();

        // address
        let item = MenuItem::new("My address: 643e446204d90ac73d00bc51ed29243d1628b062dc1d62dca2b5fc70f45b1de1", true, None);
        if let Err(e) = menu.append(&item) {
            panic!("{e:?}");
        }

        // separator
        if let Err(e) = menu.append(&PredefinedMenuItem::separator()) {
            panic!("{e:?}");
        }

        // quit
        if let Err(e) = menu.append(&PredefinedMenuItem::quit(Some("Quit drasyl UI"))) {
            panic!("{e:?}");
        }
        // self.quit_id = Some(item.id().clone());

        menu
    }
}

impl ApplicationHandler<UserEvent> for DrasylUI {
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
                self.tray_icon = Some(self.new_tray_icon());
            }

            // We have to request a redraw here to have the icon actually show up.
            // Winit only exposes a redraw method on the Window so we use core-foundation directly.
            #[cfg(target_os = "macos")]
            unsafe {
                use objc2_core_foundation::{CFRunLoopGetMain, CFRunLoopWakeUp};

                let rl = CFRunLoopGetMain().unwrap();
                CFRunLoopWakeUp(&rl);
            }
        }
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: UserEvent) {
        println!("{event:?}");
        match event {
            UserEvent::MenuEvent(menu_event) => {
                if Some(menu_event.id) == self.quit_id {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    // Starte Tokio Runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    // Starte Background-Job
    rt.spawn(async {
        let mut interval = time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            println!("Background task running...");
        }
    });

    let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();

    // set a tray event handler that forwards the event and wakes up the event loop
    let proxy = event_loop.create_proxy();
    TrayIconEvent::set_event_handler(Some(move |event| {
        proxy.send_event(UserEvent::TrayIconEvent(event));
    }));
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        proxy.send_event(UserEvent::MenuEvent(event));
    }));

    let mut app = DrasylUI::new();

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

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};
mod vulkan_app;
use vulkan_app::VulkanApp;

#[cfg(target_os = "android")]
use winit::platform::android::{activity::AndroidApp, EventLoopBuilderExtAndroid};

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Trace),
    );
    let event_loop = EventLoop::builder().with_android_app(app).build().unwrap();
    _main(event_loop);
}

#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .parse_default_env()
        .init();
    let event_loop = EventLoop::new().unwrap();
    _main(event_loop);
}

fn _main(event_loop: EventLoop<()>) {
    event_loop.set_control_flow(ControlFlow::Poll);

    log::warn!("Vulkano start main loop!");
    event_loop.run_app(&mut Application::default()).unwrap();
}

#[derive(Default)]
struct Application {
    vulkan_app: Option<VulkanApp>,
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        log::debug!("Resumed");
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        self.vulkan_app = Some(VulkanApp::new(window.clone()));
    }

    fn suspended(&mut self, _: &winit::event_loop::ActiveEventLoop) {
        log::debug!("Suspended");
        self.vulkan_app = None;
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                log::debug!("WindowEvent::CloseRequested");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                log::trace!("WindowEvent::RedrawRequested");
                if let Some(vulkan_app) = self.vulkan_app.as_mut() {
                    vulkan_app.draw_frame();
                }
            }
            WindowEvent::Resized(_) => {
                log::debug!("WindowEvent::Resized");
                if let Some(vulkan_app) = self.vulkan_app.as_mut() {
                    vulkan_app.notify_window_resized();
                    vulkan_app.window().request_redraw();
                }
            }
            _ => (),
        }
    }
}

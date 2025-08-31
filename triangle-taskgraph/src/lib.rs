mod render_context;
mod vulkan_context;

use crate::{render_context::RenderContext, vulkan_context::VulkanContext};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::WindowId,
};

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
    log::warn!("Vulkano start main loop!");
    let context = VulkanContext::new(&event_loop);
    event_loop
        .run_app(&mut Application {
            context,
            renderer: None,
        })
        .unwrap();
}

struct Application {
    context: VulkanContext,
    renderer: Option<RenderContext>,
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::debug!("Resumed");
        self.renderer = Some(RenderContext::new(event_loop, &self.context));
    }

    fn suspended(&mut self, _: &ActiveEventLoop) {
        log::debug!("Suspended");
        self.renderer = None;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                log::debug!("WindowEvent::CloseRequested");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                log::trace!("WindowEvent::RedrawRequested");
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.draw_frame();
                }
            }
            WindowEvent::Resized(_) => {
                log::debug!("WindowEvent::Resized");
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.notify_window_resized();
                    renderer.window().request_redraw();
                }
            }
            _ => (),
        }
    }
}

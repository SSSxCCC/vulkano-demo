mod vulkan_app;

use std::sync::Arc;
use vulkan_app::VulkanApp;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::WindowBuilder,
};

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Trace));
    use winit::platform::android::EventLoopBuilderExtAndroid;
    let event_loop = EventLoopBuilder::new().with_android_app(app).build();
    _main(event_loop);
}

#[cfg(not(target_os = "android"))]
fn main() {
    env_logger::builder().filter_level(log::LevelFilter::Trace).parse_default_env().init();
    let event_loop = EventLoopBuilder::new().build();
    _main(event_loop);
}

fn _main(event_loop: EventLoop<()>) {
    let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());
    let mut vulkan_app = None;

    log::warn!("Vulkano start main loop!");
    event_loop.run(move |event: Event<'_, ()>, _, control_flow| match event {
        Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
            log::info!("WindowEvent::CloseRequested");
            *control_flow = ControlFlow::Exit;
        }
        Event::Resumed => {
            log::info!("Resumed");
            vulkan_app = Some(VulkanApp::new(window.clone()));
        }
        Event::Suspended => {
            log::info!("Suspended");
            vulkan_app = None;
        }
        Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
            log::info!("WindowEvent::Resized");
            if let Some(vulkan_app) = vulkan_app.as_mut() { vulkan_app.notify_window_resized() }
        }
        Event::RedrawRequested(_) => {
            log::info!("RedrawRequested");
            if let Some(vulkan_app) = vulkan_app.as_mut() { vulkan_app.draw_frame() }
        }
        Event::MainEventsCleared => {
            log::info!("MainEventsCleared");
            if let Some(vulkan_app) = vulkan_app.as_mut() { vulkan_app.draw_frame() }
        }
        _ => (),
    });
}

use vulkano_util::{
    context::{VulkanoConfig, VulkanoContext},
    window::{VulkanoWindows, WindowDescriptor},
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
};
mod triangle_renderer;
use triangle_renderer::TriangleRenderer;

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Trace),
    );
    use winit::platform::android::EventLoopBuilderExtAndroid;
    let event_loop = EventLoopBuilder::new().with_android_app(app).build();
    _main(event_loop);
}

#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .parse_default_env()
        .init();
    let event_loop = EventLoopBuilder::new().build();
    _main(event_loop);
}

fn _main(event_loop: EventLoop<()>) {
    let mut config = VulkanoConfig::default();
    config.device_features.vulkan_memory_model = true;
    let context = VulkanoContext::new(config);
    let mut windows = VulkanoWindows::default();

    log::warn!("Vulkano start main loop!");
    event_loop.run(
        move |event: Event<'_, ()>, event_loop, control_flow| match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                log::debug!("WindowEvent::CloseRequested");
                *control_flow = ControlFlow::Exit;
            }
            Event::Resumed => {
                log::debug!("Resumed");
                windows.create_window(&event_loop, &context, &WindowDescriptor::default(), |_| {});
            }
            Event::Suspended => {
                log::debug!("Suspended");
                windows.remove_renderer(windows.primary_window_id().unwrap());
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                log::debug!("WindowEvent::Resized");
                if let Some(renderer) = windows.get_primary_renderer_mut() {
                    renderer.resize()
                }
            }
            Event::RedrawRequested(_) => {
                log::trace!("RedrawRequested");
                if let Some(renderer) = windows.get_primary_renderer_mut() {
                    let before_future = renderer.acquire().unwrap();

                    let after_future = TriangleRenderer::draw(before_future, &context, renderer);

                    renderer.present(after_future, true);
                }
            }
            Event::MainEventsCleared => {
                log::trace!("MainEventsCleared");
                if let Some(renderer) = windows.get_primary_renderer() {
                    renderer.window().request_redraw()
                }
            }
            _ => (),
        },
    );
}

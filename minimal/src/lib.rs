use vulkano_util::{window::{VulkanoWindows, WindowDescriptor}, context::VulkanoContext};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
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
    let context = VulkanoContext::default();
    let mut windows = VulkanoWindows::default();

    log::warn!("Vulkano start main loop!");
    event_loop.run(move |event: Event<'_, ()>, event_loop, control_flow| match event {
        Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
            log::info!("WindowEvent::CloseRequested");
            *control_flow = ControlFlow::Exit;
        }
        Event::Resumed => {
            log::info!("Resumed");
            windows.create_window(&event_loop, &context, &WindowDescriptor::default(), |_|{});
        }
        Event::Suspended => {
            log::info!("Suspended");
            windows.remove_renderer(windows.primary_window_id().unwrap());
        }
        Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
            log::info!("WindowEvent::Resized");
            if let Some(renderer) = windows.get_primary_renderer_mut() { renderer.resize() }
        }
        Event::RedrawRequested(_) => {
            log::info!("RedrawRequested");
            if let Some(renderer) = windows.get_primary_renderer_mut() {
                let before_future = renderer.acquire().unwrap();
                renderer.present(before_future, true);
            }
        }
        Event::MainEventsCleared => {
            log::info!("MainEventsCleared");
            if let Some(renderer) = windows.get_primary_renderer() { renderer.window().request_redraw() }
        }
        _ => (),
    });
}

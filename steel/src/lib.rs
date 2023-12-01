use vulkano_util::{window::{VulkanoWindows, WindowDescriptor}, context::VulkanoContext};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
};

mod steel;

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
#[allow(dead_code)]
fn main() {
    env_logger::builder().filter_level(log::LevelFilter::Trace).parse_default_env().init();
    let event_loop = EventLoopBuilder::new().build();
    _main(event_loop);
}

fn _main(event_loop: EventLoop<()>) {
    let context = VulkanoContext::default();
    let mut windows = VulkanoWindows::default();
    let mut engine = steel::create();
    engine.init();

    log::warn!("Vulkano start main loop!");
    event_loop.run(move |event, event_loop, control_flow| match event {
        Event::Resumed => {
            log::info!("Event::Resumed");
            windows.create_window(&event_loop, &context, &WindowDescriptor::default(), |_|{});
        }
        Event::Suspended => {
            log::info!("Event::Suspended");
            windows.remove_renderer(windows.primary_window_id().unwrap());
        }
        Event::WindowEvent { event , .. } => match event {
            WindowEvent::CloseRequested => {
                log::info!("WindowEvent::CloseRequested");
                *control_flow = ControlFlow::Exit;
            }
            WindowEvent::Resized(_) => {
                log::info!("WindowEvent::Resized");
                if let Some(renderer) = windows.get_primary_renderer_mut() { renderer.resize() }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                log::info!("WindowEvent::ScaleFactorChanged");
                if let Some(renderer) = windows.get_primary_renderer_mut() { renderer.resize() }
            }
            _ => ()
        }
        Event::RedrawRequested(_) => {
            log::info!("Event::RedrawRequested");
            if let Some(renderer) = windows.get_primary_renderer_mut() {
                let before_future = renderer.acquire().unwrap();

                engine.update();
                let after_future = engine.draw(before_future, &context, renderer);

                renderer.present(after_future, true);
            }
        }
        Event::MainEventsCleared => {
            log::info!("Event::MainEventsCleared");
            if let Some(renderer) = windows.get_primary_renderer() { renderer.window().request_redraw() }
        }
        _ => (),
    });
}

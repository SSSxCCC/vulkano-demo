use glam::Vec2;
use egui_winit_vulkano::{Gui, GuiConfig};
use vulkano::image::{StorageImage, ImageUsage};
use vulkano_util::{window::{VulkanoWindows, WindowDescriptor}, context::VulkanoContext};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
};

mod steel;
use crate::steel::DrawInfo;

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
    let mut gui = None;
    let mut demo_windows = egui_demo_lib::DemoWindows::default();
    let mut scene_image = None;
    let mut scene_texture_id = None;
    let mut scene_size = Vec2::ZERO;
    let mut engine = steel::create();
    engine.init();

    log::warn!("Vulkano start main loop!");
    event_loop.run(move |event, event_loop, control_flow| match event {
        Event::Resumed => {
            log::info!("Event::Resumed");
            windows.create_window(&event_loop, &context,
                &WindowDescriptor::default(), |_|{});
            let renderer = windows.get_primary_renderer().unwrap();
            gui = Some(Gui::new(&event_loop, renderer.surface(),
                renderer.graphics_queue(),
                renderer.swapchain_format(),
                GuiConfig { is_overlay: false, ..Default::default() }));
        }
        Event::Suspended => {
            log::info!("Event::Suspended");
            scene_texture_id = None;
            scene_image = None;
            gui = None;
            windows.remove_renderer(windows.primary_window_id().unwrap());
        }
        Event::WindowEvent { event , .. } => {
            if let Some(gui) = gui.as_mut() {
                let _pass_events_to_game = !gui.update(&event);
            }
            match event {
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
        }
        Event::RedrawRequested(_) => {
            log::info!("Event::RedrawRequested");
            if let Some(renderer) = windows.get_primary_renderer_mut() {
                let gui = gui.as_mut().unwrap();
                gui.immediate_ui(|gui| {
                    let ctx = gui.context();
                    demo_windows.ui(&ctx);
                    egui::Window::new("Scene").resizable(true).show(&ctx, |ui| {
                        let available_size = ui.available_size();
                        if scene_image.is_none() || scene_size.x != available_size.x || scene_size.y != available_size.y {
                            (scene_size.x, scene_size.y) = (available_size.x, available_size.y);
                            scene_image = Some(StorageImage::general_purpose_image_view(
                                context.memory_allocator(),
                                context.graphics_queue().clone(),
                                [scene_size.x as u32, scene_size.y as u32],
                                renderer.swapchain_format(),
                                ImageUsage::SAMPLED | ImageUsage::COLOR_ATTACHMENT,
                            ).unwrap());
                            if let Some(scene_texture_id) = scene_texture_id {
                                gui.unregister_user_image(scene_texture_id);
                            }
                            scene_texture_id = Some(gui.register_user_image_view(
                                scene_image.as_ref().unwrap().clone(), Default::default()));
                            log::info!("Created scene image, scene_size={scene_size}");
                        }
                        ui.image(scene_texture_id.unwrap(), available_size);
                    });
                });

                let gpu_future = renderer.acquire().unwrap();

                engine.update();
                let gpu_future = engine.draw(DrawInfo {
                    before_future: gpu_future, context: &context, renderer: &renderer,
                    image: scene_image.as_ref().unwrap().clone(), window_size: scene_size,
                });

                let gpu_future = gui.draw_on_image(gpu_future, renderer.swapchain_image_view());

                renderer.present(gpu_future, true);
            }
        }
        Event::MainEventsCleared => {
            log::info!("Event::MainEventsCleared");
            if let Some(renderer) = windows.get_primary_renderer() { renderer.window().request_redraw() }
        }
        _ => (),
    });
}

use egui_winit_vulkano::{Gui, GuiConfig};
use glam::Vec2;
use std::sync::Arc;
use vulkano::{
    image::{view::ImageView, Image, ImageCreateInfo, ImageUsage},
    memory::allocator::AllocationCreateInfo,
    sync::GpuFuture,
};
use vulkano_util::{
    context::VulkanoContext,
    window::{VulkanoWindows, WindowDescriptor},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
};
mod steel;
use crate::steel::DrawInfo;

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
    event_loop.run_app(&mut Application::new()).unwrap();
}

struct Application {
    context: VulkanoContext,
    windows: VulkanoWindows,
    gui: Option<Gui>,
    demo_windows: egui_demo_lib::DemoWindows,
    scene_image: Option<Arc<ImageView>>,
    scene_texture_id: Option<egui::TextureId>,
    scene_size: Vec2,
    engine: Box<dyn steel::Engine>,
}

impl Application {
    fn new() -> Self {
        let mut engine = steel::create();
        engine.init();
        Self {
            context: VulkanoContext::default(),
            windows: VulkanoWindows::default(),
            gui: None,
            demo_windows: egui_demo_lib::DemoWindows::default(),
            scene_image: None,
            scene_texture_id: None,
            scene_size: Vec2::ZERO,
            engine,
        }
    }
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        log::debug!("Resumed");
        self.windows.create_window(
            &event_loop,
            &self.context,
            &WindowDescriptor::default(),
            |_| {},
        );
        let renderer = self.windows.get_primary_renderer().unwrap();
        self.gui = Some(Gui::new(
            &event_loop,
            renderer.surface(),
            renderer.graphics_queue(),
            renderer.swapchain_format(),
            GuiConfig {
                is_overlay: false,
                ..Default::default()
            },
        ));
    }

    fn suspended(&mut self, _: &winit::event_loop::ActiveEventLoop) {
        log::debug!("Suspended");
        self.scene_texture_id = None;
        self.scene_image = None;
        self.gui = None;
        self.windows
            .remove_renderer(self.windows.primary_window_id().unwrap());
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if let Some(gui) = self.gui.as_mut() {
            let _pass_events_to_game = !gui.update(&event);
        }
        match event {
            WindowEvent::CloseRequested => {
                log::debug!("WindowEvent::CloseRequested");
                event_loop.exit();
            }
            WindowEvent::Resized(_) => {
                log::debug!("WindowEvent::Resized");
                if let Some(renderer) = self.windows.get_primary_renderer_mut() {
                    renderer.resize();
                    renderer.window().request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                log::debug!("WindowEvent::ScaleFactorChanged");
                if let Some(renderer) = self.windows.get_primary_renderer_mut() {
                    renderer.resize();
                    renderer.window().request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                log::trace!("WindowEvent::RedrawRequested");
                if let Some(renderer) = self.windows.get_primary_renderer_mut() {
                    let gui = self.gui.as_mut().unwrap();
                    gui.immediate_ui(|gui| {
                        let ctx = gui.context();
                        self.demo_windows.ui(&ctx);
                        egui::Window::new("Scene Window")
                            .resizable(true)
                            .show(&ctx, |ui| {
                                let available_size = ui.available_size();
                                if self.scene_image.is_none()
                                    || self.scene_size.x != available_size.x
                                    || self.scene_size.y != available_size.y
                                {
                                    (self.scene_size.x, self.scene_size.y) =
                                        (available_size.x, available_size.y);
                                    let image = Image::new(
                                        self.context.memory_allocator().clone(),
                                        ImageCreateInfo {
                                            format: renderer.swapchain_format(),
                                            extent: [
                                                self.scene_size.x as u32,
                                                self.scene_size.y as u32,
                                                1,
                                            ],
                                            usage: ImageUsage::SAMPLED
                                                | ImageUsage::COLOR_ATTACHMENT,
                                            ..Default::default()
                                        },
                                        AllocationCreateInfo::default(),
                                    )
                                    .unwrap();
                                    self.scene_image = Some(ImageView::new_default(image).unwrap());
                                    if let Some(scene_texture_id) = self.scene_texture_id {
                                        gui.unregister_user_image(scene_texture_id);
                                    }
                                    self.scene_texture_id = Some(gui.register_user_image_view(
                                        self.scene_image.as_ref().unwrap().clone(),
                                        Default::default(),
                                    ));
                                    log::info!(
                                        "Created scene image, scene_size={}",
                                        self.scene_size
                                    );
                                }
                                ui.image(egui::ImageSource::Texture(
                                    egui::load::SizedTexture::new(
                                        *self.scene_texture_id.as_ref().unwrap(),
                                        available_size,
                                    ),
                                ));
                            });
                    });

                    let gpu_future = renderer.acquire(None, |_| {}).unwrap();

                    self.engine.update();

                    let draw_future = self.engine.draw(DrawInfo {
                        before_future: vulkano::sync::now(self.context.device().clone()).boxed(),
                        context: &self.context,
                        renderer: &renderer,
                        image: self.scene_image.as_ref().unwrap().clone(),
                        window_size: self.scene_size,
                    });

                    let gpu_future = gui.draw_on_image(
                        gpu_future.join(draw_future),
                        renderer.swapchain_image_view(),
                    );

                    renderer.present(gpu_future, true);

                    renderer.window().request_redraw();
                }
            }
            _ => (),
        }
    }
}

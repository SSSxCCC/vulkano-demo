// Copyright (c) 2017 The vulkano developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

//! This is the source code of the "Windowing" chapter at http://vulkano.rs.
//!
//! It is not commented, as the explanations can be found in the guide itself.

use std::sync::Arc;

use platform::Platform;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

mod vulkan_app;
#[cfg_attr(target_os = "windows", path = "platform_pc.rs")]
#[cfg_attr(target_os = "android", path = "platform_android.rs")]
mod platform;
use vulkan_app::VulkanApp;

fn main() {
    let event_loop = EventLoop::new();
    let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());

    let platform = Arc::new(Platform::new(window));
    let mut vulkan_app = VulkanApp::new(platform);

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
            *control_flow = ControlFlow::Exit;
        }
        Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
            vulkan_app.notify_window_resized();
        }
        Event::MainEventsCleared => {
            vulkan_app.draw_frame();
        }
        _ => (),
    });
}
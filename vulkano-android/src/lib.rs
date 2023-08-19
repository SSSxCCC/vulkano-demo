use android_activity::{AndroidApp, InputStatus, MainEvent, PollEvent};

#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_min_level(log::Level::Info));

    loop {
        app.poll_events(Some(std::time::Duration::from_millis(500)) /* timeout */, |event| {
            match event {
                PollEvent::Wake => { log::info!("Early wake up"); },
                PollEvent::Timeout => { log::info!("Hello, World!"); },
                PollEvent::Main(main_event) => {
                    log::info!("Main event: {:?}", main_event);
                    match main_event {
                        MainEvent::Destroy => { return; }
                        _ => {}
                    }
                },
                _ => {}
            }

            app.input_events(|event| {
                log::info!("Input Event: {event:?}");
                InputStatus::Unhandled
            });
        });
    }
}
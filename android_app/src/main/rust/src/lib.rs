#[cfg(target_os = "android")]
use android_activity::AndroidApp;
#[cfg(target_os = "android")]
use client::HttpApp;

#[cfg(target_os = "android")]
#[export_name = "android_main"]
fn main(app: AndroidApp) {
    use android_logger::Config;
    use log::LevelFilter;
    // Log to android output
    android_logger::init_once(Config::default().with_max_level(LevelFilter::Info));

    let options = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };
    eframe::run_native(
        "sweat selector client",
        options,
        Box::new(|cc| Ok(Box::new(HttpApp::new(cc)))),
    )
        .unwrap()
}


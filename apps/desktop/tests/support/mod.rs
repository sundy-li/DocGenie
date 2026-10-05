use makepad_test::{Selector, TestApp};
use std::time::{Duration, Instant};

#[allow(dead_code)]
pub fn open_settings(app: &TestApp) {
    app.press_key_with_modifiers(
        makepad_test::KeyCode::Comma,
        makepad_test::KeyModifiers {
            logo: true,
            ..Default::default()
        },
    );
    app.locator(Selector::id("preferences_overlay"))
        .wait_visible();
}

/// Poll rendered ink, not just widget state: font resources/rasterization are
/// asynchronous. A flat grey screenshot or empty label cannot satisfy this.
pub fn wait_for_ink(app: &TestApp, id: &str) -> std::path::PathBuf {
    app.locator(Selector::id(id)).wait_visible();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let path = app.screenshot();
        let pixels = image::open(&path).unwrap().to_rgb8();
        let widgets = app.widget_snapshot();
        let target = widgets.iter().find(|w| w.id == id).expect("target widget");
        let root = widgets
            .iter()
            .find(|w| w.id == "main_window")
            .expect("window widget");
        let scale = pixels.width() as f64 / root.width as f64;
        let x = (target.x as f64 * scale).max(0.0) as u32;
        let y = (target.y as f64 * scale).max(0.0) as u32;
        let width = (target.width as f64 * scale) as u32;
        let height = (target.height as f64 * scale) as u32;
        let mut ink = 0;
        let mut light = 0;
        for py in y..(y + height).min(pixels.height()) {
            for px in x..(x + width).min(pixels.width()) {
                let rgb = pixels.get_pixel(px, py).0;
                if rgb.iter().all(|c| *c < 150) {
                    ink += 1;
                }
                if rgb.iter().all(|c| *c > 190) {
                    light += 1;
                }
            }
        }
        if ink > 30 && light > 30 {
            return path;
        }
        assert!(
            Instant::now() < deadline,
            "rendered text missing: {id}, ink={ink}, light={light}, screenshot={}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[allow(dead_code)]
pub mod library;

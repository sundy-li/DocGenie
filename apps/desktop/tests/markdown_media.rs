mod support;
use makepad_test::Selector;

#[test]
fn local_image_and_blue_link_render_in_both_modes() {
    let root = support::library::root("markdown_media");
    let _ = std::fs::remove_dir_all(&root);
    let library = project_store::library::Library::open(root.clone()).unwrap();
    let pixels = [30_u8, 190, 80, 255].repeat(80 * 40);
    let mut png = Vec::new();
    use image::ImageEncoder;
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&pixels, 80, 40, image::ExtendedColorType::Rgba8)
        .unwrap();
    let rel = library.store_image(&png).unwrap();
    let mut config = support::library::config("markdown_media", &root);
    config.startup_timeout = std::time::Duration::from_secs(30);
    makepad_test::run_with_config(config, |app| {
        support::wait_for_ink(&app, "revision_label");
        support::library::fill_document(
            &app,
            format!("# 媒体\n\n[fdsafd](http://www.bing.com)\n\n![图片]({rel})\n"),
        );
        app.locator(Selector::id("title_input")).click();
        for reading in [false, true] {
            if reading {
                app.locator(Selector::id("mode_edit")).click();
            }
            app.locator(Selector::all().text_exact("fdsafd"))
                .wait_visible();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                let pixels = image::open(app.screenshot()).unwrap().to_rgb8();
                let green = pixels
                    .pixels()
                    .filter(|p| p[0] < 70 && p[1] > 170 && p[2] < 120)
                    .count();
                let blue = pixels
                    .pixels()
                    .filter(|p| p[0] < 80 && p[1] > 70 && p[1] < 140 && p[2] > 190)
                    .count();
                if green > 500 && blue > 30 {
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "missing media ink: green={green}, blue={blue}"
                );
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
        app.locator(Selector::id("save_label"))
            .wait_text("已自动保存到本地");
    })
    .unwrap();
    let _ = std::fs::remove_dir_all(root);
}

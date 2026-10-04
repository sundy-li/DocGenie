mod support;
use makepad_test::{Selector, TestConfig, run_with_config};

#[test]
fn stock_render_control() {
    let mut config = TestConfig::current_package(
        env!("CARGO_MANIFEST_DIR"),
        env!("CARGO_PKG_NAME"),
        "stock_render_control",
    )
    .unwrap();
    config.bin_name = Some("render-probe".into());
    config.visible = true;
    config.startup_pause = std::time::Duration::from_millis(1500);
    run_with_config(config, |app| {
        app.locator(Selector::id("probe_label"))
            .wait_text("Render control / 原生文字");
        println!(
            "render control screenshot: {}",
            support::wait_for_ink(&app, "probe_label").display()
        );
    })
    .unwrap();
}

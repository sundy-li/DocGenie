//! Minimal native rendering control used to isolate V001.
pub use makepad_widgets;
use makepad_widgets::*;
app_main!(App);
script_mod! {
    use mod.prelude.widgets.*
    startup() do #(App::script_component(vm)){
        ui: Root{
            main_window := Window{
                window.inner_size: vec2(640, 400)
                body +: {flow: Down padding: 24 spacing: 16
                    probe_label := Label{text: "Render control / 原生文字"}
                    probe_input := TextInput{width: Fill text: "Chinese 中文 / ABC 123"}
                    probe_button := Button{text: "Native button"}
                }
            }
        }
    }
}
#[derive(Script, ScriptHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
}
impl AppMain for App {
    fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
        makepad_widgets::theme_mod(vm);
        script_eval!(vm, { mod.theme = mod.themes.light });
        makepad_widgets::widgets_mod(vm);
        self::script_mod(vm)
    }
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}

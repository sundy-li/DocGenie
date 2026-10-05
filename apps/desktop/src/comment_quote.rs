//! Compact clickable quote. Raw semantic text is retained; no source input.
use makepad_widgets::*;
script_mod! {
    use mod.prelude.widgets_internal.*
    use mod.widgets.*
    mod.widgets.CommentQuote = #(CommentQuote::register_widget(vm)){
        width: Fill height: Fit
        quote_jump := ButtonFlatter{width: Fill height: Fit padding: Inset{left: 10 right: 8 top: 8 bottom: 8} align: Align{x: 0.0 y: 0.0} grab_key_focus: false text: "选区原文"
            draw_text +: {color: #x858585 color_hover: #x687381 text_style +: {font_size: 12} max_lines: 1 text_overflow: TextOverflow.Ellipsis}
            draw_bg +: {color: #xf5f5f5 color_hover: #xf0f0f0 border_size: 0.0}
        }
    }
}
#[derive(Script, ScriptHook, Widget)]
pub struct CommentQuote {
    #[deref]
    view: View,
    #[rust]
    text: String,
}
impl Widget for CommentQuote {
    fn text(&self) -> String {
        self.text.clone()
    }
    fn set_text(&mut self, cx: &mut Cx, text: &str) {
        self.text = text.to_owned();
        self.view.button(cx, ids!(quote_jump)).set_text(
            cx,
            if text.is_empty() {
                "选区原文".into()
            } else {
                crate::comment_meta::excerpt(text, 46)
            }
            .as_str(),
        );
        self.redraw(cx);
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }
}

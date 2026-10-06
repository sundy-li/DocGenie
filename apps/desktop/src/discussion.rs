//! All messages of one thread, laid out in the outer card's scroll flow.
use document_core::{Message, Speaker};
use makepad_widgets::*;
script_mod! {
    use mod.prelude.widgets.*
    mod.widgets.Discussion = #(Discussion::register_widget(vm)){
        width: Fill height: Fit flow: Down spacing: 18
        message_template: View{width: Fill height: Fit flow: Right spacing: 10
            RectView{width: 28 height: 28 align: Align{x: 0.5 y: 0.5}
                draw_bg +: {color: #x7c3aed border_radius: 14.0 border_size: 0.0}
                avatar := Label{text: "你" draw_text +: {color: #xffffff text_style +: {font_size: 11}}}
            }
            View{width: Fill height: Fit flow: Down spacing: 6
                View{width: Fill height: Fit flow: Right spacing: 6 align: Align{y: 0.5}
                    author := Label{text: "你" draw_text +: {color: #x303740 text_style: theme.font_bold{font_size: 12}}}
                    message_time := Label{text: "" draw_text +: {color: #x868b93 text_style +: {font_size: 10}}}
                }
                message_body := Markdown{width: Fill height: Fit font_size: 14 font_color: #x303740 selectable: true
                    splash_block := View{width: Fill height: Fit splash_view := TextInput{width: Fill height: Fit is_multiline: true is_read_only: true empty_text: ""}}
                }
            }
        }
    }
}
#[derive(Script, ScriptHook, Widget)]
pub struct Discussion {
    #[deref]
    view: View,
    #[live]
    message_template: ScriptObjectRef,
    #[rust]
    messages: Vec<Message>,
    #[rust]
    transcript: String,
    #[rust]
    thread: Option<usize>,
}
impl Discussion {
    pub fn update(
        &mut self,
        cx: &mut Cx,
        thread: Option<usize>,
        messages: &[Message],
        transcript: &str,
    ) {
        if self.thread == thread && self.messages == messages && self.transcript == transcript {
            return;
        }
        if self.thread != thread {
            self.view.children.clear();
        }
        self.thread = thread;
        self.messages = messages.to_vec();
        self.transcript = transcript.to_owned();
        while self.view.children.len() > messages.len() {
            self.view.children.pop();
        }
        for (index, message) in messages.iter().enumerate() {
            if index >= self.view.children.len() {
                let template = self.message_template.clone();
                let widget =
                    cx.with_vm(|vm| WidgetRef::script_from_value(vm, template.as_object().into()));
                self.view.children.push((LiveId(index as u64 + 1), widget));
            }
            let row = &self.view.children[index].1;
            row.label(cx, ids!(author))
                .set_text(cx, crate::comment_meta::author(message));
            row.label(cx, ids!(message_time))
                .set_text(cx, &crate::comment_meta::label(message));
            row.label(cx, ids!(avatar)).set_text(
                cx,
                if message.speaker == Speaker::User {
                    "你"
                } else {
                    "AI"
                },
            );
            row.markdown(cx, ids!(message_body))
                .set_text(cx, &message.text);
        }
        cx.widget_tree_mark_dirty(self.view.widget_uid());
        self.redraw(cx);
    }
}
impl Widget for Discussion {
    fn text(&self) -> String {
        self.transcript.clone()
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }
}

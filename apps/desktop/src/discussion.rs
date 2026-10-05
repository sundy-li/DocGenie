//! Native message cards, separate from the comment composer.
use document_core::{Message, Speaker};
use makepad_widgets::*;
script_mod! {
    use mod.prelude.widgets.*
    let MessageCard = SolidView{width: Fill height: Fit flow: Down padding: 14 spacing: 8
                show_bg: true draw_bg.color: #xffffff
                View{width: Fill height: Fit flow: Right spacing: 8 align: Align{y: 0.5}
                    SolidView{width: 24 height: 24 align: Align{x: 0.5 y: 0.5} draw_bg.color: #x7c3aed
                        avatar := Label{text: "你" draw_text +: {color: #xffffff text_style +: {font_size: 12}}}
                    }
                    author := Label{text: "你" draw_text +: {color: #x303740 text_style: theme.font_bold{font_size: 13}}}
                    message_time := Label{text: "" draw_text +: {color: #x9a9a9a text_style +: {font_size: 11}}}
                }
                message_body := Markdown{width: Fill height: Fit font_size: 14 font_color: #x303740 selectable: true
                    splash_block := View{width: Fill height: Fit
                        splash_view := TextInput{width: Fill height: Fit is_multiline: true is_read_only: true empty_text: ""}
                    }
                }
                SolidView{width: Fill height: 1 draw_bg.color: #xe8eaed}
            }
    mod.widgets.Discussion = #(Discussion::register_widget(vm)){
        width: Fill height: Fill
        list := PortalList{width: Fill height: Fill flow: Down
            Message := MessageCard{}
            Agent := MessageCard{draw_bg +: {color: #xeff6fb}}
            Empty := View{width: Fill height: 1}
        }
    }
}
#[derive(Script, ScriptHook, Widget)]
pub struct Discussion {
    #[deref]
    view: View,
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
        let new_thread = self.thread != thread;
        let added = self.messages.len() < messages.len();
        self.thread = thread;
        self.messages = messages.to_vec();
        self.transcript = transcript.to_owned();
        let list = self.view.portal_list(cx, ids!(list));
        if new_thread {
            list.set_first_id_and_scroll(0, 0.0);
        } else if added {
            // Chat convention: a new message scrolls into view instead of
            // staying below the fold of the (short) transcript viewport.
            list.scroll_to_end(cx);
        }
        self.redraw(cx);
    }
}
impl Widget for Discussion {
    fn text(&self) -> String {
        self.transcript.clone()
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        while let Some(step) = self.view.draw_walk(cx, scope, walk).step() {
            if let Some(mut list) = step.as_portal_list().borrow_mut() {
                list.set_item_range(cx, 0, self.messages.len());
                while let Some(id) = list.next_visible_item(cx) {
                    let message = self.messages.get(id);
                    let row = list.item(
                        cx,
                        id,
                        if message.is_some_and(|m| m.speaker == Speaker::Agent) {
                            id!(Agent)
                        } else if message.is_some() {
                            id!(Message)
                        } else {
                            id!(Empty)
                        },
                    );
                    if let Some(message) = message {
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
                    row.draw_all_unscoped(cx);
                }
            }
        }
        DrawStep::done()
    }
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }
}

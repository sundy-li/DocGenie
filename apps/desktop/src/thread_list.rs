//! Complete native thread cards. Mutations are dispatched to App / Workbench.
use crate::agent_progress::Progress;
use document_core::{Message, Workbench};
use makepad_widgets::*;
use std::collections::{HashMap, HashSet};

script_mod! {
    use mod.prelude.widgets.*
    let CardIcon = ButtonFlatterIcon{width: 30 height: 30 padding: 0 align: Align{x: 0.5 y: 0.5} grab_key_focus: false
        icon_walk: Walk{width: 17 height: 17}
        draw_icon +: {color: #x6b7280 color_hover: #x2563eb}
        draw_bg +: {color: #x00000000 color_hover: #xeef2f7 color_down: #xe2e8f0 border_size: 0.0 border_radius: 5.0}
    }
    mod.widgets.ThreadList = #(ThreadList::register_widget(vm)){
        width: Fill height: Fill
        list := PortalList{width: Fill height: Fill flow: Down keep_invisible: true selectable: false
            Thread := View{width: Fill height: Fit flow: Down padding: Inset{bottom: 14}
                card := RectView{width: Fill height: Fit flow: Down
                    draw_bg +: {color: #xffffff border_radius: 8.0 border_size: 1.0 border_color: #xdfe2e6}
                    active_band := SolidView{width: Fill height: 5 draw_bg.color: #xffc107}
                    View{width: Fill height: Fit flow: Down padding: 14 spacing: 16
                        View{width: Fill height: Fit flow: Right spacing: 4 align: Align{y: 0.0}
                            SolidView{width: 2 height: 20 draw_bg.color: #xb9bec6}
                            quote := mod.widgets.CommentQuote{width: Fill quote_jump +: {padding: 4 draw_bg +: {color: #xffffff color_hover: #xf6f7f9}}}
                            processed_badge := Label{visible: false text: "已处理" draw_text +: {color: #x237345 text_style +: {font_size: 11}}}
                            thread_resolve_toggle := CardIcon{draw_icon.svg: crate_resource("self:resources/icons/check-circle.svg")}
                        }
                        thread_state := Label{visible: false text: "" draw_text +: {color: #x8a6a20 text_style +: {font_size: 11}}}
                        transcript := mod.widgets.Discussion{}
                        ai_panel := RectView{visible: false width: Fill height: Fit flow: Down padding: 9 spacing: 6
                            draw_bg +: {color: #xf3edff border_radius: 6.0 border_size: 0.0}
                            ai_working := View{visible: false width: Fill height: 24 flow: Right spacing: 8 align: Align{y: 0.5}
                                ai_spinner := LoadingSpinner{width: 18 height: 18 draw_bg +: {color: #x7040bd stroke_width: 2.0 rotation_speed: 1.0}}
                                Label{text: "Agent 正在处理…" draw_text +: {color: #x7040bd text_style +: {font_size: 12}}}
                            }
                            View{width: Fill height: Fit flow: Right spacing: 6 align: Align{y: 0.5}
                                ai_progress := Label{width: Fill text: "" draw_text +: {color: #x7040bd text_style +: {font_size: 12}}}
                                view_change := CardIcon{visible: false draw_icon +: {color: #x7040bd svg: crate_resource("self:resources/icons/chevron-down.svg")}}
                            }
                        }
                        change_view := mod.widgets.ChangeView{visible: false}
                        reply_box := View{width: Fill height: Fit flow: Down spacing: 6
                            comment_input := TextInput{width: Fill height: 42 is_multiline: true empty_text: "回复…" blink_speed: 0.5
                                draw_bg +: {color: #xffffff color_empty: #xffffff color_hover: #xffffff color_focus: #xffffff border_color: #xd5d9de border_color_focus: #x2563eb border_radius: 6.0}
                                draw_cursor +: {color: #x2563eb}
                            }
                            View{width: Fill height: Fit flow: Right spacing: 6 align: Align{x: 1.0 y: 0.5}
                                comment_send := CardIcon{draw_icon +: {color: #x2563eb svg: crate_resource("self:resources/icons/send.svg")}}
                                thread_rebind := ButtonFlatter{visible: false height: 28 text: "重新绑定选区" grab_key_focus: false}
                            }
                        }
                    }
                }
            }
            Empty := View{width: Fill height: 1}
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Summary {
    id: usize,
    quote: String,
    messages: Vec<Message>,
    stale: bool,
    resolved: bool,
    processed: bool,
    change: Option<document_core::CommentChange>,
}
fn summaries(workbench: &Workbench, resolved: bool) -> Vec<Summary> {
    (0..workbench.thread_count())
        .filter_map(|id| {
            let thread = workbench.thread(id)?;
            (thread.resolved() == resolved).then(|| Summary {
                id,
                quote: thread.original().into(),
                messages: thread.messages().to_vec(),
                stale: thread.revision() != workbench.revision(),
                resolved: thread.resolved(),
                processed: thread.processed(),
                change: thread.last_change().cloned(),
            })
        })
        .collect()
}
#[derive(Script, ScriptHook, Widget)]
pub struct ThreadList {
    #[deref]
    view: View,
    #[rust]
    rows: Vec<Summary>,
    #[rust]
    active: Option<usize>,
    #[rust]
    resolved: bool,
    #[rust]
    document: String,
    #[rust]
    drafts: HashMap<usize, String>,
    #[rust]
    bound: HashMap<usize, usize>,
    #[rust]
    progress: HashMap<usize, Progress>,
    #[rust]
    changes_open: HashSet<usize>,
    #[rust]
    icon_states: HashMap<usize, (bool, bool)>,
}
impl ThreadList {
    pub fn card(&self, cx: &Cx, id: usize) -> WidgetRef {
        let index = self.rows.iter().position(|r| r.id == id);
        self.view
            .portal_list(cx, ids!(list))
            .borrow()
            .and_then(|list| {
                index.and_then(|index| list.items().get(&index).map(|entry| entry.widget.clone()))
            })
            .unwrap_or_default()
    }
    pub fn toggle_change(&mut self, cx: &mut Cx, id: usize) {
        if !self.changes_open.remove(&id) {
            self.changes_open.insert(id);
        }
        self.redraw(cx);
    }
    pub fn hint_at(&self, cx: &Cx, point: DVec2) -> Option<(&'static str, Rect)> {
        for summary in &self.rows {
            let row = self.card(cx, summary.id);
            for (id, text) in [
                (
                    ids!(view_change),
                    if self.changes_open.contains(&summary.id) {
                        "收起本次修改对比"
                    } else {
                        "展开本次修改对比"
                    },
                ),
                (
                    ids!(thread_resolve_toggle),
                    if summary.resolved {
                        "重新打开评论"
                    } else {
                        "解决评论"
                    },
                ),
                (ids!(comment_send), "发送回复"),
                (ids!(quote_jump), "定位到原文"),
            ] {
                let button = row.button(cx, id);
                if button.visible() && button.point_hits_area(cx, point) {
                    return Some((text, button.area().rect(cx)));
                }
            }
        }
        None
    }
    pub fn reply_focused(&self, cx: &Cx) -> bool {
        self.cards(cx).iter().any(|(_, row)| {
            row.text_input(cx, ids!(comment_input))
                .borrow()
                .is_some_and(|i| i.key_focus(cx))
        })
    }
    pub fn cards(&self, cx: &Cx) -> Vec<(usize, WidgetRef)> {
        self.rows
            .iter()
            .map(|r| (r.id, self.card(cx, r.id)))
            .collect()
    }
    pub fn set_progress(&mut self, progress: &HashMap<usize, Progress>) {
        self.progress = progress.clone();
    }
    pub fn update(
        &mut self,
        cx: &mut Cx,
        workbench: &Workbench,
        resolved: bool,
        active: Option<usize>,
        document: &str,
        drafts: &HashMap<Option<usize>, String>,
    ) {
        let rows = summaries(workbench, resolved);
        if self.document != document {
            self.document = document.into();
            self.bound.clear();
            self.changes_open.clear();
            self.icon_states.clear();
            // A document has its own append-only thread IDs. Drop every old
            // native reply input, not just the cards currently in the filter.
            let list = self.view.portal_list(cx, ids!(list));
            if let Some(mut list) = list.borrow_mut() {
                for (_, entry) in list.items().iter() {
                    entry
                        .widget
                        .text_input(cx, ids!(comment_input))
                        .set_text(cx, "");
                }
                list.set_first_id_and_scroll(0, 0.0);
            }
        }
        let active_changed = self.active != active || self.resolved != resolved;
        self.drafts = drafts
            .iter()
            .filter_map(|(id, text)| id.map(|id| (id, text.clone())))
            .collect();
        self.rows = rows;
        self.active = active;
        self.resolved = resolved;
        // Instantiate by stable thread ID here so App can scope actions and
        // draft access before the next draw. Only one outer scroll viewport.
        let list = self.view.portal_list(cx, ids!(list));
        if let Some(mut list) = list.borrow_mut() {
            for index in 0..self.rows.len() {
                let row = list.item(cx, index, id!(Thread));
                let input = row.text_input(cx, ids!(comment_input));
                let draft = self
                    .drafts
                    .get(&self.rows[index].id)
                    .map_or("", String::as_str);
                if input.text() != draft {
                    input.set_text(cx, draft);
                }
                self.bound.insert(index, self.rows[index].id);
            }
            if active_changed
                && let Some(index) = self.rows.iter().position(|r| Some(r.id) == active)
            {
                list.set_first_id_and_scroll(index, 0.0);
            }
        }
        self.redraw(cx);
    }
}
impl Widget for ThreadList {
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        while let Some(step) = self.view.draw_walk(cx, scope, walk).step() {
            if let Some(mut list) = step.as_portal_list().borrow_mut() {
                list.set_item_range(cx, 0, self.rows.len());
                while let Some(index) = list.next_visible_item(cx) {
                    let summary = self.rows.get(index);
                    let row = list.item(
                        cx,
                        index,
                        if summary.is_some() {
                            id!(Thread)
                        } else {
                            id!(Empty)
                        },
                    );
                    if let Some(summary) = summary {
                        row.widget(cx, ids!(quote)).set_text(cx, &summary.quote);
                        let input = row.text_input(cx, ids!(comment_input));
                        let draft = self.drafts.get(&summary.id).map_or("", String::as_str);
                        if self.bound.get(&index) != Some(&summary.id) {
                            input.set_text(cx, draft);
                            self.bound.insert(index, summary.id);
                        }
                        let progress = self
                            .progress
                            .get(&summary.id)
                            .cloned()
                            .filter(|p| *p != Progress::Completed || summary.processed)
                            .or_else(|| summary.processed.then_some(Progress::Completed));
                        let working = progress.as_ref().is_some_and(|p| p.busy());
                        row.view(cx, ids!(ai_working)).set_visible(cx, working);
                        row.label(cx, ids!(processed_badge))
                            .set_visible(cx, summary.processed);
                        let has_diff = summary.change.as_ref().is_some_and(|c| c.before != c.after);
                        row.view(cx, ids!(ai_panel))
                            .set_visible(cx, progress.is_some());
                        row.label(cx, ids!(ai_progress))
                            .set_text(cx, &progress.map_or(String::new(), |p| p.label()));
                        row.button(cx, ids!(view_change)).set_visible(cx, has_diff);
                        row.widget(cx, ids!(change_view))
                            .set_visible(cx, has_diff && self.changes_open.contains(&summary.id));
                        if has_diff
                            && let Some(change) = &summary.change
                            && let Some(mut view) = row
                                .widget(cx, ids!(change_view))
                                .borrow_mut::<crate::change_view::ChangeView>()
                        {
                            view.update(cx, change, summary.stale);
                        }
                        row.view(cx, ids!(active_band))
                            .set_visible(cx, self.active == Some(summary.id));
                        let enabled = !row
                            .text_input(cx, ids!(comment_input))
                            .text()
                            .trim()
                            .is_empty();
                        row.button(cx, ids!(comment_send)).set_enabled(cx, enabled);
                        row.button(cx, ids!(thread_rebind))
                            .set_visible(cx, summary.stale && self.active == Some(summary.id));
                        row.view(cx, ids!(reply_box))
                            .set_visible(cx, self.active == Some(summary.id) && !summary.resolved);
                        row.label(cx, ids!(thread_state))
                            .set_visible(cx, summary.stale);
                        row.label(cx, ids!(thread_state))
                            .set_text(cx, "原段落已变化 · 请重新绑定");
                        let icon_state =
                            (summary.resolved, self.changes_open.contains(&summary.id));
                        if self.icon_states.get(&index) != Some(&icon_state) {
                            let mut resolve = row.button(cx, ids!(thread_resolve_toggle));
                            if summary.resolved {
                                script_apply_eval!(cx,resolve,{draw_icon.svg: crate_resource("self:resources/icons/reopen.svg")});
                            } else {
                                script_apply_eval!(cx,resolve,{draw_icon.svg: crate_resource("self:resources/icons/check-circle.svg")});
                            }
                            let mut diff = row.button(cx, ids!(view_change));
                            if self.changes_open.contains(&summary.id) {
                                script_apply_eval!(cx,diff,{draw_icon.svg: crate_resource("self:resources/icons/chevron-up.svg")});
                            } else {
                                script_apply_eval!(cx,diff,{draw_icon.svg: crate_resource("self:resources/icons/chevron-down.svg")});
                            }
                            self.icon_states.insert(index, icon_state);
                        }
                        if let Some(mut messages) = row
                            .widget(cx, ids!(transcript))
                            .borrow_mut::<crate::discussion::Discussion>()
                        {
                            let transcript = summary
                                .messages
                                .iter()
                                .map(|m| {
                                    format!(
                                        "{}\n{}",
                                        if m.speaker == document_core::Speaker::User {
                                            "你"
                                        } else {
                                            "Agent"
                                        },
                                        m.text
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join("\n\n");
                            messages.update(cx, Some(summary.id), &summary.messages, &transcript);
                        }
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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filters_preserve_ids_and_all_messages() {
        let mut w = Workbench::new("相同文字\n\n相同文字").unwrap();
        let a = w.comment(0..12, "第一条").unwrap();
        let b = w.comment(14..26, "第二条").unwrap();
        w.reply(a, "回复").unwrap();
        w.resolve_thread(a).unwrap();
        assert_eq!(summaries(&w, false)[0].id, b);
        assert_eq!(summaries(&w, true)[0].messages.len(), 2);
        w.edit("完全变化").unwrap();
        assert!(summaries(&w, false)[0].stale);
    }
}

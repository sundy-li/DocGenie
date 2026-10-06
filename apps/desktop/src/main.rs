mod agent;
mod agent_progress;
mod autosave;
mod ui;
mod ui_hint;
use autosave::Autosave;
mod change_view;
mod clipboard;
mod comment_meta;
mod comment_quote;
mod context_menu;
mod discussion;
mod edit_projection;
mod fonts;
mod live_editor;
mod markdown;
mod markdown_parse;
mod preferences;
mod reading;
mod styled_input;
mod styled_layout;
mod tab_menu;
mod table_menu;
mod tabs;
mod thread_list;
mod typography;
mod workspace;
use document_core::WorkbenchSnapshot;
use document_core::{CommentRequest, Workbench};
pub use makepad_widgets;
use makepad_widgets::*;
use preferences::Preferences;
use project_store::library::{Document, Library, Loaded, Node, UiState};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    ops::Range,
    sync::{Mutex, OnceLock},
};

app_main!(App);
#[derive(Clone)]
struct TreeRow {
    rel: String,
    name: String,
    is_dir: bool,
    depth: usize,
    open: bool,
    active: bool,
}
static TREE_ROWS: OnceLock<Mutex<Vec<TreeRow>>> = OnceLock::new();
fn tree_rows() -> &'static Mutex<Vec<TreeRow>> {
    TREE_ROWS.get_or_init(|| Mutex::new(Vec::new()))
}
const MAX_TABS: usize = 6;
const SWITCH_SLOTS: usize = 8;
const FONT_ROW_SLOTS: usize = 8;
fn font_row_id(i: usize) -> LiveId {
    [
        id!(font_row0),
        id!(font_row1),
        id!(font_row2),
        id!(font_row3),
        id!(font_row4),
        id!(font_row5),
        id!(font_row6),
        id!(font_row7),
    ][i]
}
fn tab_id(i: usize) -> LiveId {
    [
        id!(tab0),
        id!(tab1),
        id!(tab2),
        id!(tab3),
        id!(tab4),
        id!(tab5),
    ][i]
}
fn switch_id(i: usize) -> LiveId {
    [
        id!(sw0),
        id!(sw1),
        id!(sw2),
        id!(sw3),
        id!(sw4),
        id!(sw5),
        id!(sw6),
        id!(sw7),
    ][i]
}

#[derive(Clone)]
struct OutlineEntry {
    title: String,
    range: Range<usize>,
}
static OUTLINE: OnceLock<Mutex<Vec<OutlineEntry>>> = OnceLock::new();
fn outline() -> &'static Mutex<Vec<OutlineEntry>> {
    OUTLINE.get_or_init(|| Mutex::new(Vec::new()))
}

#[derive(Script, ScriptHook, Widget)]
struct Outline {
    #[deref]
    view: View,
    #[live]
    document_mode: bool,
}
impl Widget for Outline {
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        while let Some(step) = self.view.draw_walk(cx, scope, walk).step() {
            if let Some(mut list) = step.as_portal_list().borrow_mut() {
                if self.document_mode {
                    let rows = tree_rows().lock().unwrap().clone();
                    list.set_item_range(cx, 0, rows.len());
                    while let Some(id) = list.next_visible_item(cx) {
                        let item = list.item(
                            cx,
                            id,
                            if id < rows.len() {
                                id!(Row)
                            } else {
                                id!(Empty)
                            },
                        );
                        if let Some(row) = rows.get(id) {
                            let mut indent = item.view(cx, ids!(indent));
                            let width = 12.0 * row.depth as f64;
                            script_apply_eval!(cx, indent, { width: #(width) });
                            item.button(cx, ids!(fold_open))
                                .set_visible(cx, row.is_dir && row.open);
                            item.button(cx, ids!(fold_closed))
                                .set_visible(cx, row.is_dir && !row.open);
                            item.view(cx, ids!(file_gap)).set_visible(cx, !row.is_dir);
                            item.button(cx, ids!(heading)).set_text(cx, &row.name);
                            let mut bg = item.clone();
                            let color =
                                Vec4f::from_u32(if row.active { 0xe9edf3ff } else { 0x00000000 });
                            script_apply_eval!(cx, bg, { draw_bg +: { color: #(color) } });
                        }
                        item.draw_all_unscoped(cx);
                    }
                } else {
                    let headings = outline().lock().unwrap().clone();
                    list.set_item_range(cx, 0, headings.len());
                    while let Some(id) = list.next_visible_item(cx) {
                        let item = list.item(
                            cx,
                            id,
                            if id < headings.len() {
                                id!(Row)
                            } else {
                                id!(Empty)
                            },
                        );
                        if let Some(entry) = headings.get(id) {
                            item.button(cx, ids!(heading)).set_text(cx, &entry.title);
                        }
                        item.draw_all_unscoped(cx);
                    }
                }
            }
        }
        DrawStep::done()
    }
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }
}

enum LibraryCommand {
    Initialize(Option<String>),
    CreateNote(String),
    Open(Document),
    Save {
        document: Document,
        state: WorkbenchSnapshot,
        baseline: String,
    },
}
enum Dialog {
    Rename(String),
    Delete(String),
}
enum LibraryReply {
    Loaded(Result<Loaded, project_store::StoreError>),
    Saved {
        state: WorkbenchSnapshot,
        result: Result<(), project_store::StoreError>,
    },
}
enum AgentEvent {
    Progress {
        epoch: u64,
        project_id: String,
        thread: usize,
        round: u64,
        revision: u64,
        progress: agent_progress::Progress,
    },
    Finished(AgentReply),
}
struct AgentReply {
    epoch: u64,
    project_id: String,
    request: CommentRequest,
    result: Result<agent::Edit, String>,
}

#[derive(Script, ScriptHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[rust]
    workbench: Workbench,
    #[rust]
    library: Option<Library>,
    #[rust]
    document: Option<Document>,
    #[rust]
    persisted: Option<WorkbenchSnapshot>,
    #[rust]
    baseline: String,
    #[rust]
    file_rx: Option<std::sync::mpsc::Receiver<LibraryReply>>,
    #[rust]
    pending_switch: Option<LibraryCommand>,
    #[rust]
    save_timer: Timer,
    #[rust]
    autosave: Autosave,
    #[rust]
    save_failed: bool,
    #[rust]
    agent_rx: Option<std::sync::mpsc::Receiver<AgentEvent>>,
    #[rust]
    agent_states: HashMap<usize, agent_progress::Progress>,
    #[rust]
    agent_running: Option<usize>,
    #[rust]
    agent_queue: VecDeque<usize>,
    #[rust]
    closing: bool,
    #[rust]
    quitting: bool,
    #[rust]
    pending_tabs: Option<Vec<String>>,
    #[rust]
    tab_menu_target: Option<String>,
    #[rust]
    tab_menu_pointer: bool,
    #[rust]
    tab_menu_opening: bool,
    #[rust]
    table_target: Option<(Range<usize>, usize, usize, u64)>,
    #[rust]
    table_pointer: bool,
    #[rust]
    table_opening: bool,
    #[rust]
    hint_target: Option<(String, DVec2)>,
    #[rust]
    hint_timer: Timer,
    #[rust]
    selected: Option<Range<usize>>,
    #[rust]
    thread: Option<usize>,
    #[rust]
    rebind_target: Option<usize>,
    #[rust]
    auto: bool,
    #[rust]
    epoch: u64,
    #[rust]
    preferences: Preferences,
    #[rust]
    preferences_visible: bool,
    /// Which Preferences page the left nav shows: 0 外观, 1 编辑器, 2 Agent.
    #[rust]
    preferences_page: usize,
    /// Set once the background installed-font scan has finished and the
    /// configured code font has been applied (Preferences re-applies it too).
    #[rust]
    fonts_ready: bool,
    #[rust]
    document_selection: bool,
    #[rust]
    editing: bool,
    #[rust]
    popup_selection: Option<(Range<usize>, u64)>,
    #[rust]
    popup_position: Option<DVec2>,
    #[rust]
    popup_pointer: bool,
    #[rust]
    composer_pointer: bool,
    #[rust]
    composer_ask_ai: bool,
    #[rust]
    nodes: Vec<Node>,
    #[rust]
    expanded: HashSet<String>,
    #[rust]
    selected_node: Option<String>,
    #[rust]
    tabs: Vec<String>,
    #[rust]
    right_outline: bool,
    #[rust]
    filter_resolved: bool,
    #[rust]
    reply_drafts: HashMap<Option<usize>, String>,
    #[rust]
    dialog: Option<Dialog>,
    #[rust]
    switcher_hits: Vec<String>,
    #[rust]
    switcher_visible: bool,
    #[rust]
    window_width: f64,
    #[rust]
    navigation_collapsed: bool,
    #[rust]
    comments_collapsed: bool,
}
impl App {
    fn hide_table_menu(&mut self, cx: &mut Cx) {
        self.table_target = None;
        self.table_pointer = false;
        self.table_opening = false;
        if let Some(mut menu) = self
            .ui
            .widget(cx, ids!(table_menu))
            .borrow_mut::<table_menu::TableMenu>()
        {
            menu.hide(cx);
        }
    }
    fn clear_hint(&mut self, cx: &mut Cx) {
        cx.stop_timer(self.hint_timer);
        self.hint_timer = Timer::default();
        self.hint_target = None;
        if let Some(mut hint) = self
            .ui
            .widget(cx, ids!(ui_hint))
            .borrow_mut::<ui_hint::UiHint>()
        {
            hint.hide(cx);
        }
    }
    fn hover_hint(&mut self, cx: &mut Cx, point: DVec2) {
        let card_hint = self
            .ui
            .widget(cx, ids!(threads))
            .borrow::<thread_list::ThreadList>()
            .and_then(|list| list.hint_at(cx, point));
        let mut found = card_hint;
        if found.is_none() {
            for (id, text) in [
                (ids!(toggle_navigation), "收起文件侧栏"),
                (ids!(expand_navigation), "展开文件侧栏"),
                (ids!(toggle_comments), "收起大纲与评论"),
                (ids!(expand_comments), "展开大纲与评论"),
                (ids!(undo_button), "撤销正文修改 · ⌘Z"),
                (ids!(retry_save), "重试保存"),
                (ids!(agent_cancel), "停止 Agent"),
                (ids!(new_folder), "新建文件夹"),
                (ids!(rename_node), "重命名所选文件"),
                (ids!(delete_node), "删除所选文件"),
                (ids!(thread_prev), "上一条评论"),
                (ids!(thread_next), "下一条评论"),
                (ids!(comment_new), "新建评论"),
                (ids!(mode_edit), "切换到阅读模式"),
                (ids!(mode_read), "切换到编辑模式"),
            ] {
                let button = self.ui.button(cx, id);
                if button.visible() && button.point_hits_area(cx, point) {
                    found = Some((text, button.area().rect(cx)));
                    break;
                }
            }
        }
        let target =
            found.map(|(text, rect)| (text.to_owned(), rect.pos + dvec2(0.0, rect.size.y + 6.0)));
        if self.hint_target == target {
            return;
        }
        self.clear_hint(cx);
        self.hint_target = target;
        if self.hint_target.is_some() {
            self.hint_timer = cx.start_timeout(0.4);
        }
    }
    fn set_sidebar_visibility(&mut self, cx: &mut Cx, navigation: bool, comments: bool) {
        self.navigation_collapsed = !navigation;
        self.comments_collapsed = !comments;
        self.layout_sidebars(cx);
    }

    fn layout_sidebars(&self, cx: &mut Cx) {
        let width = if self.window_width > 0.0 {
            self.window_width
        } else {
            1400.0
        };
        let (navigation, comments) =
            workspace::sidebars(width, !self.navigation_collapsed, !self.comments_collapsed);
        self.ui
            .view(cx, ids!(navigation_panel))
            .set_visible(cx, navigation);
        self.ui
            .view(cx, ids!(comments_panel))
            .set_visible(cx, comments);
        self.ui
            .view(cx, ids!(navigation_rail))
            .set_visible(cx, !navigation);
        self.ui
            .view(cx, ids!(comments_rail))
            .set_visible(cx, !comments);
        for (id, shown) in [
            (ids!(toggle_navigation), navigation),
            (ids!(toggle_comments), comments),
        ] {
            let mut button = self.ui.button(cx, id);
            let color = Vec4f::from_u32(if shown { 0xe9edf3ff } else { 0x00000000 });
            script_apply_eval!(cx,button,{draw_bg +: {color: #(color)}});
        }
        // Relayout invalidates the menu's old screen position, not its selection.
        if let Some(mut menu) = self
            .ui
            .widget(cx, ids!(comment_menu))
            .borrow_mut::<context_menu::CommentMenu>()
        {
            menu.hide(cx);
        }
    }

    fn snapshot(&self) -> WorkbenchSnapshot {
        self.workbench.snapshot()
    }
    fn status(&self, cx: &mut Cx, text: &str) {
        self.ui.label(cx, ids!(status_label)).set_text(cx, text);
    }
    fn sync_status(&self, cx: &mut Cx) {
        self.ui
            .button(cx, ids!(retry_save))
            .set_visible(cx, self.save_failed);
        self.ui
            .button(cx, ids!(agent_cancel))
            .set_visible(cx, self.agent_rx.is_some());
        self.ui.label(cx, ids!(agent_hint)).set_text(
            cx,
            if self.auto {
                "Agent 默认开启 · 会外发选区和评论"
            } else {
                "Agent 默认关闭 · 问 AI 可单条授权"
            },
        );
        self.ui
            .label(cx, ids!(revision_label))
            .set_text(cx, &format!("Revision {}", self.workbench.revision()));
        self.ui.label(cx, ids!(save_label)).set_text(
            cx,
            if self.save_failed {
                "自动保存失败 · 保留内存内容"
            } else if self.file_rx.is_some() {
                "正在保存 / 加载…"
            } else if self.persisted.as_ref() == Some(&self.snapshot()) {
                "已自动保存到本地"
            } else {
                "等待自动保存…"
            },
        );
    }
    /// Replace (or remove) the first `# heading` line in the workbench
    /// text with `new_title`. Empty titles strip the line entirely so the
    /// next sync shows the placeholder. Driven by the title_input TextInput.
    fn apply_title_change(&mut self, cx: &mut Cx, new_title: &str) {
        // The top document title is a special plain field: no displayed
        // Markdown prefix. Only serialization adds the H1 marker.
        let trimmed = new_title.trim();
        let mut text = self.workbench.text().to_owned();
        let found = reading::title_block(&text);
        match (found, trimmed.is_empty()) {
            (Some(range), false) => {
                text.replace_range(range, &format!("# {trimmed}"));
            }
            (Some(range), true) => {
                // Strip the heading line and its trailing newline (if any)
                // so the body starts cleanly.
                let mut end = range.end;
                if text.as_bytes().get(end) == Some(&b'\n') {
                    end += 1;
                }
                text.replace_range(range.start..end, "");
            }
            (None, false) => {
                // Clearing a title leaves the original blank separator in the
                // body. Reuse it when retyping instead of adding another one.
                let separator = if text.starts_with('\n') { "\n" } else { "\n\n" };
                text.insert_str(0, &format!("# {trimmed}{separator}"));
            }
            (None, true) => {}
        }
        // Push through the domain's trusted set_text so undo / revision stay
        // consistent without going through the proposal flow.
        match self.workbench.set_text(text) {
            Ok(()) => {
                self.selected = None;
                self.sync_document(cx);
                self.show_thread(cx);
                self.status(cx, "标题已更新");
            }
            Err(e) => self.status(cx, &e.to_string()),
        }
    }
    /// Unresolved, current-revision comment ranges paint the live editor's
    /// rows yellow; reading mode computes the same set inside Reading::update.
    fn sync_editor_highlights(&self, cx: &mut Cx) {
        let ranges: Vec<Range<usize>> = (0..self.workbench.thread_count())
            .filter_map(|id| {
                let thread = self.workbench.thread(id)?;
                (!thread.resolved()
                    && !thread.processed()
                    && thread.revision() == self.workbench.revision())
                .then(|| thread.range())
            })
            .collect();
        if let Some(mut live) = self
            .ui
            .widget(cx, ids!(live_editor))
            .borrow_mut::<live_editor::LiveEditor>()
        {
            live.set_highlights(cx, ranges);
        }
    }
    fn sync_document(&self, cx: &mut Cx) {
        // The title is shown only in title_input; it is the first
        // single-line `# heading` block. The placeholder shows when absent.
        let target = match reading::title_block(self.workbench.text()) {
            Some(range) => self.workbench.text()[range][2..].trim().to_owned(),
            None => String::new(),
        };
        let title_widget = self.ui.text_input(cx, ids!(title_input));
        let focused = title_widget.borrow().is_some_and(|t| t.key_focus(cx));
        let current = title_widget.text();
        // During typing, normalization of edge whitespace must not rewrite
        // the focused buffer, interrupt IME or reset the native undo stack.
        if current != target && !(focused && current.trim() == target) {
            title_widget.set_text(cx, &target);
        }

        if let Some(mut live) = self
            .ui
            .widget(cx, ids!(live_editor))
            .borrow_mut::<live_editor::LiveEditor>()
        {
            live.update(cx, self.workbench.text());
        }
        self.sync_reading(cx);
        let mut entries = Vec::new();
        let mut offset = 0;
        let mut fenced = false;
        for line in self.workbench.text().split_inclusive('\n') {
            if line.trim_start().starts_with("```") {
                fenced = !fenced;
            }
            if !fenced {
                let level = line.chars().take_while(|c| *c == '#').count();
                if (1..=6).contains(&level) && line.as_bytes().get(level) == Some(&b' ') {
                    entries.push(OutlineEntry {
                        title: format!(
                            "{}{}",
                            "  ".repeat(level.saturating_sub(1)),
                            line[level..].trim()
                        ),
                        range: offset..offset + line.trim_end().len(),
                    });
                }
            }
            offset += line.len();
        }
        *outline().lock().unwrap() = entries;
        self.ui.widget(cx, ids!(navigation)).redraw(cx);
        self.sync_editor_highlights(cx);
        self.sync_status(cx);
    }
    fn mode(&mut self, cx: &mut Cx, edit: bool) {
        self.hide_table_menu(cx);
        self.popup_selection = None;
        self.hide_composer(cx);
        if let Some(mut menu) = self
            .ui
            .widget(cx, ids!(comment_menu))
            .borrow_mut::<context_menu::CommentMenu>()
        {
            menu.hide(cx);
        }
        if let Some(mut live) = self
            .ui
            .widget(cx, ids!(live_editor))
            .borrow_mut::<live_editor::LiveEditor>()
        {
            live.deactivate(cx);
        }
        self.editing = edit;
        self.ui.view(cx, ids!(live_panel)).set_visible(cx, edit);
        self.ui.view(cx, ids!(preview_panel)).set_visible(cx, !edit);
        // The icon shows the current mode; clicking it switches to the other.
        self.ui.button(cx, ids!(mode_edit)).set_visible(cx, edit);
        self.ui.button(cx, ids!(mode_read)).set_visible(cx, !edit);
    }
    fn readonly(&self, cx: &mut Cx, value: bool) {
        if let Some(mut live) = self
            .ui
            .widget(cx, ids!(live_editor))
            .borrow_mut::<live_editor::LiveEditor>()
        {
            live.set_readonly(cx, value);
        }
        // The title edits the outgoing document through the workbench; without
        // this, a title typed while an async document switch is in flight is
        // silently discarded when the Loaded reply replaces the workbench.
        self.ui
            .text_input(cx, ids!(title_input))
            .set_is_read_only(cx, value);
        if let Some(list) = self
            .ui
            .widget(cx, ids!(threads))
            .borrow::<thread_list::ThreadList>()
        {
            for (_, card) in list.cards(cx) {
                card.text_input(cx, ids!(comment_input))
                    .set_is_read_only(cx, value);
            }
        }
    }
    fn sync_reading(&self, cx: &mut Cx) {
        if let Some(mut reading) = self
            .ui
            .widget(cx, ids!(preview))
            .borrow_mut::<reading::Reading>()
        {
            reading.update(cx, &self.workbench);
        }
    }
    fn show_preferences(&mut self, cx: &mut Cx) {
        self.preferences_visible = true;
        self.ui
            .view(cx, ids!(preferences_overlay))
            .set_visible(cx, true);
        self.set_preferences_page(cx, self.preferences_page);
        self.sync_preferences_form(cx);
        // Refresh the "current code font" line (the scan may have finished
        // while the overlay was closed, or still be running).
        self.apply_code_font_pref(cx);
    }
    fn hide_preferences(&mut self, cx: &mut Cx) {
        self.preferences_visible = false;
        self.ui
            .view(cx, ids!(preferences_overlay))
            .set_visible(cx, false);
    }
    fn toggle_preferences(&mut self, cx: &mut Cx) {
        if self.preferences_visible {
            self.hide_preferences(cx);
        } else {
            self.show_preferences(cx);
        }
    }
    /// Obsidian-style left nav: one visible page, the active entry tinted.
    fn set_preferences_page(&mut self, cx: &mut Cx, page: usize) {
        self.preferences_page = page;
        for (index, nav, panel) in [
            (0, id!(pref_nav_appearance), id!(pref_page_appearance)),
            (1, id!(pref_nav_editor), id!(pref_page_editor)),
            (2, id!(pref_nav_agent), id!(pref_page_agent)),
        ] {
            let active = index == page;
            self.ui.view(cx, &[panel]).set_visible(cx, active);
            let mut button = self.ui.button(cx, &[nav]);
            if active {
                script_apply_eval!(cx, button, {
                    draw_text: {color: #x2162c2 color_hover: #x2162c2}
                });
            } else {
                script_apply_eval!(cx, button, {
                    draw_text: {color: #x303740 color_hover: #x2162c2}
                });
            }
            button.redraw(cx);
        }
    }
    /// Push the live `Preferences` into the form widgets. The panel edits
    /// `self.preferences` directly (instant apply), so the widgets simply
    /// mirror it whenever the overlay opens or a change lands.
    fn sync_preferences_form(&self, cx: &mut Cx) {
        let config = agent::Config::probe();
        self.ui
            .label(cx, ids!(pref_model_endpoint_label))
            .set_text(cx, &format!("Endpoint: {}", config.endpoint));
        self.ui
            .label(cx, ids!(pref_model_name_label))
            .set_text(cx, &format!("Model: {}", config.model));
        self.ui.label(cx, ids!(pref_model_key_label)).set_text(
            cx,
            if config.key_present {
                "API Key: 已配置 (来自环境变量)"
            } else {
                "API Key: 未配置 (AGENT_DOCS_API_KEY 或 MINIMAX_API_KEY)"
            },
        );
        self.ui.check_box(cx, ids!(pref_default_auto)).set_active(
            cx,
            self.preferences.default_auto_modify,
            Animate::No,
        );
        self.ui.check_box(cx, ids!(pref_auto_save)).set_active(
            cx,
            self.preferences.auto_save_enabled,
            Animate::No,
        );
        self.ui
            .text_input(cx, ids!(pref_font_size))
            .set_text(cx, &self.preferences.editor_font_size.to_string());
        self.sync_font_rows(cx);
    }
    /// Mirror the code-font priority list into the fixed row slots and the
    /// add-font dropdown (installed families not already listed).
    fn sync_font_rows(&self, cx: &mut Cx) {
        let fonts = &self.preferences.editor_code_fonts;
        for i in 0..FONT_ROW_SLOTS {
            let row = self.ui.view(cx, &[font_row_id(i)]);
            let Some(name) = fonts.get(i) else {
                row.set_visible(cx, false);
                continue;
            };
            row.set_visible(cx, true);
            row.label(cx, ids!(font_name)).set_text(cx, name);
            row.button(cx, ids!(font_up)).set_visible(cx, i > 0);
            row.button(cx, ids!(font_down))
                .set_visible(cx, i + 1 < fonts.len());
        }
        let mut labels = vec!["添加字体…".to_owned()];
        labels.extend(
            fonts::menu_names()
                .into_iter()
                .filter(|name| !fonts.contains(name)),
        );
        let dropdown = self.ui.drop_down(cx, ids!(pref_font_add));
        dropdown.set_labels(cx, labels);
        dropdown.set_selected_item(cx, 0);
    }
    /// Persist the current `Preferences`; failures surface in the status bar.
    fn save_preferences(&mut self, cx: &mut Cx) {
        match preferences::save(&self.preferences) {
            Ok(path) => self.status(cx, &format!("Preferences 已保存: {}", path.display())),
            Err(e) => self.status(cx, &format!("Preferences 保存失败: {e}")),
        }
    }
    /// Apply the whole `Preferences` snapshot to runtime state. Called at
    /// startup and after every panel edit, so changes take effect live.
    fn apply_preferences(&mut self, cx: &mut Cx) {
        self.auto = self.preferences.default_auto_modify;
        let body = self.preferences.editor_font_size as f64;
        if let Some(mut live) = self
            .ui
            .widget(cx, ids!(live_editor))
            .borrow_mut::<live_editor::LiveEditor>()
        {
            live.set_body_size(cx, body);
        }
        if let Some(mut reading) = self
            .ui
            .widget(cx, ids!(preview))
            .borrow_mut::<reading::Reading>()
        {
            reading.set_body_size(cx, body);
        }
        self.apply_code_font_pref(cx);
    }
    /// Apply the configured code-font priority list to the theme's code
    /// font, then force the editors to re-layout with the new face.
    fn apply_code_font_pref(&mut self, cx: &mut Cx) {
        let applied = fonts::apply_code_font(cx, &self.preferences.editor_code_fonts);
        if let Some(mut live) = self
            .ui
            .widget(cx, ids!(live_editor))
            .borrow_mut::<live_editor::LiveEditor>()
        {
            live.restyle(cx);
        }
        if let Some(mut reading) = self
            .ui
            .widget(cx, ids!(preview))
            .borrow_mut::<reading::Reading>()
        {
            reading.redraw(cx);
        }
        if self.preferences_visible {
            let text = match &applied {
                Some(name) => format!("当前代码字体: {name}"),
                None if fonts::ready() => "当前代码字体: 内置 Liberation Mono".to_owned(),
                None => "正在扫描已安装字体…".to_owned(),
            };
            self.ui
                .label(cx, ids!(pref_font_status))
                .set_text(cx, &text);
        }
        cx.redraw_all();
    }
    /// Bring the active thread's paragraph into view in the current editor
    /// mode. Called from prev/next/Open so the user never needs a separate
    /// "locate" button. No-op when the thread is stale or already on screen.
    fn reveal_active_thread(&mut self, cx: &mut Cx) {
        let Some(id) = self.thread else { return };
        let Some(thread) = self.workbench.thread(id).cloned() else {
            return;
        };
        if thread.revision() != self.workbench.revision() {
            self.status(cx, "原段落已变化，请选中当前段落后重新绑定。");
            return;
        }
        let range = thread.range();
        if !self.editing {
            if let Some(mut r) = self
                .ui
                .widget(cx, ids!(preview))
                .borrow_mut::<reading::Reading>()
            {
                r.reveal(cx, range);
            }
            return;
        }
        if let Some(mut live) = self
            .ui
            .widget(cx, ids!(live_editor))
            .borrow_mut::<live_editor::LiveEditor>()
        {
            live.activate_at(cx, range.start);
        }
    }
    fn show_thread(&mut self, cx: &mut Cx) {
        self.ui
            .view(cx, ids!(new_comment_panel))
            .set_visible(cx, self.thread.is_none());
        let open_count = (0..self.workbench.thread_count())
            .filter(|id| self.workbench.thread(*id).is_some_and(|t| !t.resolved()))
            .count();
        let resolved_count = self.workbench.thread_count() - open_count;
        self.ui
            .button(cx, ids!(filter_open))
            .set_text(cx, &format!("未解决 {open_count}"));
        self.ui
            .button(cx, ids!(filter_resolved))
            .set_text(cx, &format!("已解决 {resolved_count}"));
        let empty = if self.filter_resolved {
            resolved_count == 0
        } else {
            open_count == 0
        };
        self.ui.widget(cx, ids!(threads)).set_visible(cx, !empty);
        self.ui
            .label(cx, ids!(threads_empty))
            .set_visible(cx, empty);
        self.ui.label(cx, ids!(threads_empty)).set_text(
            cx,
            if self.filter_resolved {
                "还没有已解决的评论。"
            } else {
                "没有未解决评论。选中文本后添加评论。"
            },
        );
        if let Some(mut list) = self
            .ui
            .widget(cx, ids!(threads))
            .borrow_mut::<thread_list::ThreadList>()
        {
            list.set_progress(&self.agent_states);
            list.update(
                cx,
                &self.workbench,
                self.filter_resolved,
                self.thread,
                self.document.as_ref().map_or("", |d| d.rel.as_str()),
                &self.reply_drafts,
            );
        }
        for (id, active) in [
            (ids!(filter_open), !self.filter_resolved),
            (ids!(filter_resolved), self.filter_resolved),
        ] {
            let mut button = self.ui.button(cx, id);
            let color = Vec4f::from_u32(if active { 0xe9edf3ff } else { 0x00000000 });
            script_apply_eval!(cx, button, {draw_bg +: {color: #(color)}});
        }
        self.sync_reading(cx);
        self.sync_editor_highlights(cx);
        // Prev/next are always visible (even with a single thread) because
        // they also serve as the auto-reveal trigger for the active thread.
        self.ui.button(cx, ids!(thread_prev)).set_visible(cx, true);
        self.ui.button(cx, ids!(thread_next)).set_visible(cx, true);
        if self.thread.is_none() {
            self.ui
                .button(cx, ids!(thread_rebind))
                .set_visible(cx, self.rebind_target.is_some() && self.selected.is_some());
        }
        self.ui
            .label(cx, ids!(comments_label))
            .set_text(cx, &format!("评论 ({})", self.workbench.thread_count()));
        if self.thread.is_none() {
            self.ui.widget(cx, ids!(quote)).set_text(
                cx,
                self.selected
                    .as_ref()
                    .and_then(|r| self.workbench.text().get(r.clone()))
                    .unwrap_or(""),
            );
        }
    }
    fn capture_selection(&mut self, cx: &mut Cx, position: DVec2) -> Result<(), String> {
        let range = if self.editing {
            self.ui
                .widget(cx, ids!(live_editor))
                .borrow::<live_editor::LiveEditor>()
                .and_then(|e| e.selection(cx))
                .ok_or("请先选中当前行的文字")?
        } else {
            self.ui
                .widget(cx, ids!(preview))
                .borrow::<reading::Reading>()
                .and_then(|r| r.selected_block(cx, position))
                .ok_or("请先选中文档段落中的文字")?
        };
        self.rebind_target = self.thread;
        self.selected = Some(range);
        self.thread = None;
        self.show_thread(cx);
        Ok(())
    }
    fn undo_document(&mut self, cx: &mut Cx) {
        if self.document.is_none()
            || self.pending_switch.is_some()
            || self.pending_tabs.is_some()
            || self.closing
            || self.save_failed
        {
            return;
        }
        let cursor = self
            .ui
            .widget(cx, ids!(live_editor))
            .borrow::<live_editor::LiveEditor>()
            .filter(|e| e.focused(cx))
            .and_then(|e| e.cursor_offset(cx));
        match self.workbench.undo() {
            Ok(()) => {
                self.selected = None;
                self.popup_selection = None;
                self.hide_composer(cx);
                self.sync_document(cx);
                if let Some(cursor) = cursor {
                    let mut offset = cursor.min(self.workbench.text().len());
                    while !self.workbench.text().is_char_boundary(offset) {
                        offset -= 1;
                    }
                    if let Some(mut live) = self
                        .ui
                        .widget(cx, ids!(live_editor))
                        .borrow_mut::<live_editor::LiveEditor>()
                    {
                        live.activate_at(cx, offset);
                    }
                }
                self.show_thread(cx);
                self.queue_save(cx);
                self.status(cx, "已撤销；重叠评论锚点过期，不会自动覆盖。");
            }
            Err(e) => self.status(cx, &e.to_string()),
        }
    }
    fn stamp_message(&mut self, id: usize, author: &str) {
        let _ = self
            .workbench
            .stamp_last_message(id, author, comment_meta::timestamp());
    }
    fn run_agent(&mut self, cx: &mut Cx) {
        let Some(id) = self.thread else {
            self.status(cx, "请先发送评论。");
            return;
        };
        let allowed = self.auto || self.workbench.thread(id).is_some_and(|t| t.ask_ai());
        if !allowed {
            self.status(
                cx,
                "勾选「问 AI」或在 Preferences 开启默认自动修改，将外发当前段落与本条评论。",
            );
            return;
        }
        if self.agent_rx.is_some() {
            self.enqueue_agent(id);
            self.agent_states
                .insert(id, agent_progress::Progress::Queued);
            self.show_thread(cx);
            self.status(cx, "评论已排队，Agent 将处理最新回复。");
            return;
        }
        self.start_agent(cx, id);
    }
    fn enqueue_agent(&mut self, id: usize) {
        if !self.agent_queue.contains(&id) {
            self.agent_queue.push_back(id);
        }
    }
    fn drain_agent(&mut self, cx: &mut Cx) {
        if self.agent_rx.is_none() {
            while let Some(id) = self.agent_queue.pop_front() {
                let wanted = self.auto || self.workbench.thread(id).is_some_and(|t| t.ask_ai());
                if wanted
                    && self.workbench.comment_request(id).is_ok_and(|r| {
                        r.messages
                            .last()
                            .is_some_and(|m| m.speaker == document_core::Speaker::User)
                    })
                {
                    self.start_agent(cx, id);
                    break;
                } else {
                    self.agent_states
                        .insert(id, agent_progress::Progress::Ignored);
                    self.show_thread(cx);
                }
            }
        }
    }
    fn start_agent(&mut self, cx: &mut Cx, id: usize) {
        let request = match self.workbench.comment_request(id) {
            Ok(r) => r,
            Err(e) => {
                self.agent_states
                    .insert(id, agent_progress::Progress::Failed(e.to_string()));
                self.show_thread(cx);
                self.status(cx, &e.to_string());
                return;
            }
        };
        let config = match agent::Config::from_env() {
            Ok(c) => c,
            Err(e) => {
                self.agent_states
                    .insert(id, agent_progress::Progress::Failed(e.clone()));
                self.show_thread(cx);
                self.status(cx, &e);
                return;
            }
        };
        let epoch = self.epoch;
        let Some(document) = self.document.as_ref() else {
            return;
        };
        let project_id = document.id.to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        self.agent_rx = Some(rx);
        self.agent_running = Some(id);
        self.agent_states
            .insert(id, agent_progress::Progress::Preparing);
        self.show_thread(cx);
        self.status(cx, "Agent 正在读取选区与评论…");
        std::thread::spawn(move || {
            let result = agent::run_with_progress(config, &request, |progress| {
                let _ = tx.send(AgentEvent::Progress {
                    epoch,
                    project_id: project_id.clone(),
                    thread: id,
                    round: request.round,
                    revision: request.revision,
                    progress,
                });
                SignalToUI::set_ui_signal();
            });
            let _ = tx.send(AgentEvent::Finished(AgentReply {
                epoch,
                project_id,
                request,
                result,
            }));
            SignalToUI::set_ui_signal();
        });
    }
    fn poll_agent(&mut self, cx: &mut Cx) {
        let reply = loop {
            match self.agent_rx.as_ref().map(|rx| rx.try_recv()) {
                Some(Ok(AgentEvent::Progress {
                    epoch,
                    project_id,
                    thread,
                    round,
                    revision,
                    progress,
                })) => {
                    if epoch == self.epoch
                        && self
                            .document
                            .as_ref()
                            .is_some_and(|d| d.id.to_string() == project_id)
                        && self.workbench.thread(thread).is_some_and(|t| {
                            t.round() == round
                                && !t.resolved()
                                && self.workbench.revision() == revision
                        })
                    {
                        self.agent_states.insert(thread, progress);
                        self.show_thread(cx);
                    }
                }
                Some(Ok(AgentEvent::Finished(reply))) => break reply,
                Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                    self.agent_rx = None;
                    if let Some(id) = self.agent_running.take() {
                        self.agent_states.insert(
                            id,
                            agent_progress::Progress::Failed("任务异常，文档未修改".into()),
                        );
                        self.show_thread(cx);
                    }
                    self.status(cx, "Agent 任务异常，文档未修改");
                    self.drain_agent(cx);
                    return;
                }
                _ => return,
            }
        };
        self.agent_rx = None;
        self.agent_running = None;
        let wanted = self.auto
            || self
                .workbench
                .thread(reply.request.thread)
                .is_some_and(|t| t.ask_ai());
        if !wanted
            || reply.epoch != self.epoch
            || self
                .document
                .as_ref()
                .is_none_or(|doc| reply.project_id != doc.id.to_string())
        {
            if self
                .document
                .as_ref()
                .is_some_and(|d| d.id.to_string() == reply.project_id)
            {
                self.agent_states
                    .entry(reply.request.thread)
                    .and_modify(|p| {
                        if *p != agent_progress::Progress::Cancelled {
                            *p = agent_progress::Progress::Ignored;
                        }
                    });
                self.show_thread(cx);
            }
            self.status(cx, "结果已忽略，文档未修改");
            self.drain_agent(cx);
            return;
        }
        match reply.result {
            Ok(edit) => match self.workbench.apply_comment_result(
                &reply.request,
                &edit.replacement,
                &edit.explanation,
            ) {
                Ok(()) => {
                    self.agent_states
                        .insert(reply.request.thread, agent_progress::Progress::Completed);
                    self.stamp_message(reply.request.thread, "Agent");
                    self.selected = None;
                    self.sync_document(cx);
                    self.show_thread(cx);
                    self.status(
                        cx,
                        if edit.replacement == reply.request.original {
                            "Agent 已回复 · 正文未修改 · 评论已处理"
                        } else {
                            "已修改选区并回复 · 可撤销 · 自动保存"
                        },
                    );
                }
                Err(e) => {
                    self.agent_states
                        .insert(reply.request.thread, agent_progress::Progress::Ignored);
                    self.show_thread(cx);
                    self.status(cx, &e.to_string());
                    // Only a newer user reply on the same unchanged paragraph
                    // triggers a fresh round, never rebase a stale document.
                    if self
                        .workbench
                        .comment_request(reply.request.thread)
                        .is_ok_and(|r| r.round > reply.request.round)
                    {
                        self.enqueue_agent(reply.request.thread);
                    }
                }
            },
            Err(e) => {
                self.agent_states.insert(
                    reply.request.thread,
                    agent_progress::Progress::Failed(e.clone()),
                );
                self.show_thread(cx);
                self.status(cx, &e);
            }
        }
        self.drain_agent(cx);
    }
    fn refresh_documents(&mut self, cx: &mut Cx) {
        if let Some(library) = &self.library
            && let Ok(nodes) = library.tree()
        {
            self.nodes = nodes;
            let nodes = &self.nodes;
            self.tabs
                .retain(|rel| nodes.iter().any(|n| !n.is_dir && &n.rel == rel));
        }
        self.rebuild_rows(cx);
        self.draw_tabs(cx);
    }
    fn node_name(&self, rel: &str) -> String {
        self.nodes
            .iter()
            .find(|n| n.rel == rel)
            .map(|n| n.name.clone())
            .unwrap_or_else(|| rel.rsplit('/').next().unwrap_or(rel).to_owned())
    }
    fn rebuild_rows(&self, cx: &mut Cx) {
        let active = self.document.as_ref().map(|d| d.rel.as_str());
        let mut rows = Vec::new();
        let mut hidden_below: Option<usize> = None;
        for node in &self.nodes {
            if let Some(depth) = hidden_below {
                if node.depth > depth {
                    continue;
                }
                hidden_below = None;
            }
            let open = node.is_dir && self.expanded.contains(&node.rel);
            if node.is_dir && !open {
                hidden_below = Some(node.depth);
            }
            rows.push(TreeRow {
                rel: node.rel.clone(),
                name: node.name.clone(),
                is_dir: node.is_dir,
                depth: node.depth,
                open,
                active: self.selected_node.as_deref() == Some(&node.rel)
                    || (self.selected_node.is_none() && active == Some(&node.rel)),
            });
        }
        *tree_rows().lock().unwrap() = rows;
        self.ui.widget(cx, ids!(document_list)).redraw(cx);
    }
    fn draw_tabs(&self, cx: &mut Cx) {
        let active = self.document.as_ref().map(|d| d.rel.as_str());
        for i in 0..MAX_TABS {
            let mut tab = self.ui.view(cx, &[tab_id(i)]);
            let Some(rel) = self.tabs.get(i) else {
                tab.set_visible(cx, false);
                continue;
            };
            tab.set_visible(cx, true);
            let color = Vec4f::from_u32(if active == Some(rel.as_str()) {
                0xffffffff
            } else {
                0x00000000
            });
            script_apply_eval!(cx, tab, { draw_bg +: { color: #(color) } });
            tab.button(cx, ids!(title))
                .set_text(cx, &self.node_name(rel));
            tab.button(cx, ids!(close)).set_visible(cx, true);
        }
        self.ui.redraw(cx);
    }
    fn persist_ui(&self) {
        if let Some(library) = &self.library {
            let mut expanded: Vec<_> = self.expanded.iter().cloned().collect();
            expanded.sort();
            let _ = library.save_ui(&UiState {
                expanded,
                tabs: self.tabs.clone(),
                active: self.document.as_ref().map(|d| d.rel.clone()),
                right_tab: if self.right_outline {
                    "outline".into()
                } else {
                    "comments".into()
                },
            });
        }
    }
    fn set_right_tab(&mut self, cx: &mut Cx, outline: bool) {
        self.right_outline = outline;
        for (id, active) in [(ids!(tab_outline), outline), (ids!(tab_comments), !outline)] {
            let mut button = self.ui.button(cx, id);
            let color = Vec4f::from_u32(if active { 0xe9edf3ff } else { 0x00000000 });
            script_apply_eval!(cx, button, {draw_bg +: {color: #(color)}});
        }
        self.ui
            .view(cx, ids!(outline_body))
            .set_visible(cx, outline);
        self.ui
            .view(cx, ids!(comments_body))
            .set_visible(cx, !outline);
        self.persist_ui();
    }
    /// The folder new notes/folders are created in: the selection itself if it
    /// is a folder, otherwise the parent of the selected or active document.
    fn target_dir(&self) -> String {
        let rel = self
            .selected_node
            .clone()
            .or_else(|| self.document.as_ref().map(|d| d.rel.clone()))
            .unwrap_or_default();
        if self.nodes.iter().any(|n| n.is_dir && n.rel == rel) {
            rel
        } else {
            rel.rsplit_once('/')
                .map(|(d, _)| d.to_owned())
                .unwrap_or_default()
        }
    }
    fn open_rel(&mut self, cx: &mut Cx, rel: &str) {
        if self.document.as_ref().is_some_and(|d| d.rel == rel) {
            return;
        }
        let Some(library) = &self.library else {
            return;
        };
        match library.document(rel) {
            Ok(doc) => self.switch_document(cx, LibraryCommand::Open(doc)),
            Err(e) => self.status(cx, &e.to_string()),
        }
    }
    fn toggle_folder(&mut self, cx: &mut Cx, rel: &str) {
        if !self.expanded.remove(rel) {
            self.expanded.insert(rel.to_owned());
        }
        self.selected_node = Some(rel.to_owned());
        self.rebuild_rows(cx);
        self.persist_ui();
    }
    fn tree_idle(&mut self, cx: &mut Cx) -> bool {
        let idle = self.file_rx.is_none()
            && self.pending_switch.is_none()
            && self.persisted.as_ref() == Some(&self.snapshot());
        if !idle {
            self.status(cx, "正在保存，请稍后重试。");
        }
        idle
    }
    fn show_dialog(&mut self, cx: &mut Cx, dialog: Dialog) {
        let (title, name) = match &dialog {
            Dialog::Rename(rel) => ("重命名".to_owned(), Some(self.node_name(rel))),
            Dialog::Delete(rel) => (format!("删除「{}」？不可撤销。", self.node_name(rel)), None),
        };
        self.ui.label(cx, ids!(dialog_title)).set_text(cx, &title);
        self.ui.label(cx, ids!(dialog_status)).set_text(cx, "");
        let input = self.ui.text_input(cx, ids!(dialog_input));
        input.set_visible(cx, name.is_some());
        input.set_text(cx, name.as_deref().unwrap_or(""));
        self.dialog = Some(dialog);
        self.ui.view(cx, ids!(dialog_overlay)).set_visible(cx, true);
        if name.is_some() {
            input.set_key_focus(cx);
        }
    }
    fn hide_dialog(&mut self, cx: &mut Cx) {
        self.dialog = None;
        self.ui
            .view(cx, ids!(dialog_overlay))
            .set_visible(cx, false);
    }
    fn replace_prefix(rel: &str, old: &str, new: &str) -> Option<String> {
        if rel == old {
            Some(new.to_owned())
        } else {
            rel.strip_prefix(&format!("{old}/"))
                .map(|rest| format!("{new}/{rest}"))
        }
    }
    fn apply_rename(&mut self, library: &Library, rel: &str, new_rel: &str) {
        let remap = |r: &str| Self::replace_prefix(r, rel, new_rel);
        for tab in &mut self.tabs {
            if let Some(n) = remap(tab) {
                *tab = n;
            }
        }
        if let Some(tabs) = self.pending_tabs.as_mut() {
            for tab in tabs {
                if let Some(n) = remap(tab) {
                    *tab = n;
                }
            }
        }
        if let Some(target) = self.tab_menu_target.as_mut()
            && let Some(n) = remap(target)
        {
            *target = n;
        }
        self.expanded = self
            .expanded
            .iter()
            .map(|r| remap(r).unwrap_or_else(|| r.clone()))
            .collect();
        self.selected_node = Some(new_rel.to_owned());
        if let Some(active) = self.document.as_ref().and_then(|d| remap(&d.rel))
            && let Ok(doc) = library.document(&active)
        {
            self.document = Some(doc);
        }
    }
    /// Keeps the file name in step with the H1 once the title is saved.
    fn sync_file_name(&mut self, cx: &mut Cx) {
        let (Some(library), Some(document)) = (self.library.clone(), self.document.clone()) else {
            return;
        };
        if self.persisted.as_ref() != Some(&self.snapshot()) {
            return;
        }
        match library.sync_name(&document.rel, self.workbench.text()) {
            Ok(Some(new_rel)) => {
                self.apply_rename(&library, &document.rel, &new_rel);
                self.refresh_documents(cx);
                self.persist_ui();
            }
            Ok(None) => {}
            Err(e) => self.status(cx, &e.to_string()),
        }
    }
    fn commit_dialog(&mut self, cx: &mut Cx) {
        let Some(dialog) = self.dialog.take() else {
            return;
        };
        if !self.tree_idle(cx) {
            self.dialog = Some(dialog);
            return;
        }
        let Some(library) = self.library.clone() else {
            return;
        };
        match dialog {
            Dialog::Rename(rel) => {
                let name = self.ui.text_input(cx, ids!(dialog_input)).text();
                match library.rename(&rel, name.trim()) {
                    Ok(new_rel) => {
                        self.apply_rename(&library, &rel, &new_rel);
                        self.hide_dialog(cx);
                        self.refresh_documents(cx);
                        self.persist_ui();
                    }
                    Err(e) => {
                        self.dialog = Some(Dialog::Rename(rel));
                        self.ui
                            .label(cx, ids!(dialog_status))
                            .set_text(cx, &e.to_string());
                    }
                }
            }
            Dialog::Delete(rel) => match library.delete(&rel) {
                Ok(()) => {
                    let inside = |r: &str| Self::replace_prefix(r, &rel, "").is_some();
                    let active_gone = self.document.as_ref().is_some_and(|d| inside(&d.rel));
                    self.tabs.retain(|t| !inside(t));
                    self.expanded.retain(|t| !inside(t));
                    self.selected_node = None;
                    self.hide_dialog(cx);
                    if active_gone {
                        self.document = None;
                        self.persisted = None;
                        self.epoch += 1;
                        let next = self.tabs.last().cloned();
                        self.launch_library(cx, LibraryCommand::Initialize(next));
                    } else {
                        self.refresh_documents(cx);
                        self.persist_ui();
                    }
                }
                Err(e) => {
                    self.dialog = Some(Dialog::Delete(rel));
                    self.ui
                        .label(cx, ids!(dialog_status))
                        .set_text(cx, &e.to_string());
                }
            },
        }
    }
    fn create_folder(&mut self, cx: &mut Cx) {
        let Some(library) = self.library.clone() else {
            return;
        };
        let dir = self.target_dir();
        match library.create_folder(&dir, "新文件夹") {
            Ok(rel) => {
                if !dir.is_empty() {
                    self.expanded.insert(dir);
                }
                self.selected_node = Some(rel.clone());
                self.refresh_documents(cx);
                self.persist_ui();
                self.show_dialog(cx, Dialog::Rename(rel));
            }
            Err(e) => self.status(cx, &e.to_string()),
        }
    }
    fn show_switcher(&mut self, cx: &mut Cx) {
        self.switcher_visible = true;
        self.ui
            .view(cx, ids!(switcher_overlay))
            .set_visible(cx, true);
        let input = self.ui.text_input(cx, ids!(switcher_input));
        input.set_text(cx, "");
        input.set_key_focus(cx);
        self.update_switcher(cx, "");
    }
    fn hide_switcher(&mut self, cx: &mut Cx) {
        self.switcher_visible = false;
        self.ui
            .view(cx, ids!(switcher_overlay))
            .set_visible(cx, false);
    }
    fn update_switcher(&mut self, cx: &mut Cx, query: &str) {
        let query = query.trim().to_lowercase();
        self.switcher_hits = self
            .nodes
            .iter()
            .filter(|n| !n.is_dir)
            .filter(|n| query.is_empty() || n.rel.to_lowercase().contains(&query))
            .take(SWITCH_SLOTS)
            .map(|n| n.rel.clone())
            .collect();
        for i in 0..SWITCH_SLOTS {
            let button = self.ui.button(cx, &[switch_id(i)]);
            match self.switcher_hits.get(i) {
                Some(rel) => {
                    button.set_visible(cx, true);
                    button.set_text(cx, rel.strip_suffix(".md").unwrap_or(rel));
                }
                None => button.set_visible(cx, false),
            }
        }
    }
    fn close_tab(&mut self, cx: &mut Cx, index: usize) {
        if let Some(target) = self.tabs.get(index).cloned() {
            self.close_tabs(cx, &target, tabs::Close::Current);
        }
    }
    fn hide_tab_menu(&mut self, cx: &mut Cx) {
        self.tab_menu_target = None;
        self.tab_menu_pointer = false;
        self.tab_menu_opening = false;
        if let Some(mut menu) = self
            .ui
            .widget(cx, ids!(tab_menu))
            .borrow_mut::<tab_menu::TabMenu>()
        {
            menu.hide(cx);
        }
    }
    fn close_tabs(&mut self, cx: &mut Cx, target: &str, command: tabs::Close) {
        if self.pending_switch.is_some()
            || self.pending_tabs.is_some()
            || self.closing
            || self.save_failed
        {
            return;
        }
        let Some(remaining) = tabs::remaining(&self.tabs, target, command) else {
            return;
        };
        if remaining == self.tabs {
            return;
        }
        if self
            .document
            .as_ref()
            .is_some_and(|d| !remaining.contains(&d.rel))
        {
            self.pending_tabs = Some(remaining);
            self.agent_queue.clear();
            self.epoch += 1;
            self.readonly(cx, true);
            self.flush_save(cx);
        } else {
            self.apply_closed_tabs(cx, remaining);
        }
    }
    fn apply_closed_tabs(&mut self, cx: &mut Cx, tabs: Vec<String>) {
        let removed = self
            .document
            .as_ref()
            .is_some_and(|d| !tabs.contains(&d.rel));
        self.tabs = tabs;
        if removed {
            if let Some(next) = self.tabs.last().cloned() {
                if let Some(library) = &self.library
                    && let Ok(document) = library.document(&next)
                {
                    self.pending_switch = Some(LibraryCommand::Open(document));
                }
            } else {
                self.cancel_save_timer(cx);
                self.document = None;
                self.workbench = document_core::Workbench::new("").expect("empty workbench");
                self.persisted = Some(self.snapshot());
                self.baseline = String::new();
                self.thread = None;
                self.selected = None;
                self.rebind_target = None;
                self.reply_drafts.clear();
                self.agent_states.clear();
                self.agent_running = None;
                self.hide_composer(cx);
                self.readonly(cx, false);
                self.sync_document(cx);
                self.show_thread(cx);
            }
        }
        self.ui
            .view(cx, ids!(article_surface))
            .set_visible(cx, self.document.is_some());
        self.ui
            .view(cx, ids!(empty_workspace))
            .set_visible(cx, self.document.is_none());
        self.draw_tabs(cx);
        self.persist_ui();
    }
    fn touch_tab(&mut self, rel: &str) {
        if !self.tabs.iter().any(|t| t == rel) {
            if self.tabs.len() >= MAX_TABS {
                self.tabs.remove(0);
            }
            self.tabs.push(rel.to_owned());
        }
    }
    fn launch_library(&mut self, cx: &mut Cx, command: LibraryCommand) {
        if self.file_rx.is_some() {
            return;
        }
        let Some(library) = self.library.clone() else {
            return;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        self.file_rx = Some(rx);
        std::thread::spawn(move || {
            let reply = match command {
                LibraryCommand::Initialize(preferred) => {
                    LibraryReply::Loaded(library.list().and_then(|docs| {
                        let wanted = preferred
                            .and_then(|rel| docs.iter().find(|d| d.rel == rel))
                            .or(docs.first());
                        if let Some(document) = wanted {
                            library.load(document)
                        } else {
                            library.create()
                        }
                    }))
                }
                LibraryCommand::CreateNote(dir) => {
                    LibraryReply::Loaded(library.create_note(&dir, "未命名文档"))
                }
                LibraryCommand::Open(document) => LibraryReply::Loaded(library.load(&document)),
                LibraryCommand::Save {
                    document,
                    state,
                    baseline,
                } => LibraryReply::Saved {
                    result: library.save(&document, &state, Some(&baseline)),
                    state,
                },
            };
            let _ = tx.send(reply);
            SignalToUI::set_ui_signal();
        });
        self.sync_status(cx);
    }
    fn cancel_save_timer(&mut self, cx: &mut Cx) {
        cx.stop_timer(self.save_timer);
        self.save_timer = Timer::default();
        self.autosave.clear();
    }
    /// Called only after a document mutation (or enabling auto-save).
    fn queue_save(&mut self, cx: &mut Cx) {
        self.cancel_save_timer(cx);
        if self.document.is_none()
            || self.save_failed
            || !self.preferences.auto_save_enabled
            || self.persisted.as_ref() == Some(&self.snapshot())
        {
            return;
        }
        self.autosave.changed(std::time::Instant::now());
        self.save_timer = cx.start_timeout(autosave::DELAY.as_secs_f64());
    }
    fn flush_save(&mut self, cx: &mut Cx) {
        if self.file_rx.is_some() || self.save_failed {
            return;
        }
        if self.document.is_none() || self.persisted.as_ref() == Some(&self.snapshot()) {
            self.cancel_save_timer(cx);
            if let Some(tabs) = self.pending_tabs.take() {
                self.apply_closed_tabs(cx, tabs);
            }
            if let Some(command) = self.pending_switch.take() {
                self.launch_library(cx, command);
            } else if self.quitting {
                cx.quit();
            } else if self.closing {
                self.ui.window(cx, ids!(main_window)).close(cx);
            }
            return;
        }
        if let Some(document) = self.document.clone() {
            self.cancel_save_timer(cx);
            self.launch_library(
                cx,
                LibraryCommand::Save {
                    document,
                    state: self.snapshot(),
                    baseline: self.baseline.clone(),
                },
            );
        }
    }
    fn switch_document(&mut self, cx: &mut Cx, command: LibraryCommand) {
        self.hide_table_menu(cx);
        if self.save_failed {
            self.status(cx, "保存失败，不能切换并丢弃当前编辑。");
            return;
        }
        self.agent_queue.clear();
        self.pending_switch = Some(command);
        self.readonly(cx, true);
        self.epoch += 1; // invalidate inflight Agent before loading a new document
        self.flush_save(cx);
    }
    fn poll_file(&mut self, cx: &mut Cx) {
        let reply = match self.file_rx.as_ref().map(|rx| rx.try_recv()) {
            Some(Ok(reply)) => reply,
            Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                self.file_rx = None;
                self.save_failed = true;
                self.status(cx, "保存任务异常，内存内容未丢弃。");
                self.sync_status(cx);
                return;
            }
            _ => return,
        };
        self.file_rx = None;
        match reply {
            LibraryReply::Loaded(result) => match result {
                Ok(loaded) => {
                    self.cancel_save_timer(cx);
                    self.readonly(cx, false);
                    self.workbench = loaded.workbench;
                    self.agent_states.clear();
                    self.agent_running = None;
                    self.reply_drafts.clear();
                    self.ui.text_input(cx, ids!(comment_input)).set_text(cx, "");
                    let rel = loaded.document.rel.clone();
                    self.document = Some(loaded.document);
                    self.ui
                        .view(cx, ids!(article_surface))
                        .set_visible(cx, true);
                    self.ui
                        .view(cx, ids!(empty_workspace))
                        .set_visible(cx, false);
                    self.selected_node = Some(rel.clone());
                    let mut dir = rel.as_str();
                    while let Some((parent, _)) = dir.rsplit_once('/') {
                        self.expanded.insert(parent.to_owned());
                        dir = parent;
                    }
                    self.touch_tab(&rel);
                    self.baseline = loaded.baseline;
                    self.persisted = Some(self.snapshot());
                    self.auto = self.preferences.default_auto_modify;
                    self.agent_queue.clear();
                    self.epoch += 1;
                    self.thread = self.workbench.thread_count().checked_sub(1);
                    self.filter_resolved = self
                        .thread
                        .and_then(|id| self.workbench.thread(id))
                        .is_some_and(|t| t.resolved());
                    self.rebind_target = None;
                    self.selected = None;
                    self.show_thread(cx);
                    self.sync_document(cx);
                    self.mode(cx, true);
                    // A fresh (or body-empty) document invites writing: put
                    // the caret in the trailing empty row so its placeholder
                    // shows instead of a blank page below the title.
                    let units = live_editor::visible_units(self.workbench.text());
                    if units.iter().all(|unit| unit.is_empty())
                        && let Some(last) = units.last()
                        && let Some(mut live) = self
                            .ui
                            .widget(cx, ids!(live_editor))
                            .borrow_mut::<live_editor::LiveEditor>()
                    {
                        live.activate_at(cx, last.start);
                    }
                    self.refresh_documents(cx);
                    self.persist_ui();
                    self.status(
                        cx,
                        if loaded.recovered_comments {
                            "选中文本，右键添加批注。"
                        } else {
                            "正文已加载；旧评论元数据不匹配，未恢复过期锚点。"
                        },
                    );
                }
                Err(e) => {
                    self.readonly(cx, false);
                    self.closing = false;
                    self.status(cx, &e.to_string());
                    self.save_failed = true;
                }
            },
            LibraryReply::Saved { state, result } => match result {
                Ok(()) => {
                    self.baseline = Workbench::restore(state.clone())
                        .expect("local validated snapshot")
                        .text()
                        .to_owned();
                    self.persisted = Some(state);
                    self.sync_file_name(cx);
                    self.refresh_documents(cx);
                    self.ui
                        .label(cx, ids!(modified_label))
                        .set_text(cx, "刚刚修改");
                    // IO completion is not an edit. A newer snapshot must
                    // respect its own trailing-edge deadline, unless switching
                    // or closing requires an immediate durability barrier.
                    if self.pending_switch.is_some()
                        || self.pending_tabs.is_some()
                        || self.closing
                        || self.autosave.ready(std::time::Instant::now())
                    {
                        self.flush_save(cx);
                    }
                }
                Err(e) => {
                    self.cancel_save_timer(cx);
                    self.save_failed = true;
                    self.closing = false;
                    self.pending_switch = None;
                    self.pending_tabs = None;
                    self.quitting = false;
                    self.readonly(cx, false);
                    self.status(cx, &e.to_string());
                }
            },
        }
        self.sync_status(cx);
    }
    /// Feishu-style ⌘+⇧+M: capture the current selection into popup_selection
    /// and open the comment composer. Mirrors the toolbar's add_comment path
    /// so both entry points behave identically.
    fn comment_shortcut_action(&mut self, cx: &mut Cx) {
        let range = if self.editing {
            self.ui
                .widget(cx, ids!(live_editor))
                .borrow::<live_editor::LiveEditor>()
                .and_then(|e| e.selection(cx))
        } else {
            None
        };
        match range {
            Some(range)
                if range.start < range.end
                    && self.workbench.text().get(range.clone()).is_some() =>
            {
                self.popup_selection = Some((range, self.workbench.revision()));
                let position = self
                    .ui
                    .widget(cx, ids!(live_editor))
                    .borrow::<live_editor::LiveEditor>()
                    .and_then(|e| e.selection_anchor(cx))
                    .unwrap_or(dvec2(120.0, 160.0));
                self.open_composer(cx, position);
            }
            _ => self.status(cx, "请先在文档中选中要评论的文字"),
        }
    }
    /// Open the inline comment card at the selection. The card carries the
    /// quote, the draft input and the per-comment 「问 AI」 switch; the
    /// sidebar thread list only opens once the comment is sent.
    fn open_composer(&mut self, cx: &mut Cx, position: DVec2) {
        let Some((range, revision)) = self.popup_selection.clone() else {
            self.status(cx, "请先在文档中选中要评论的文字");
            return;
        };
        if revision != self.workbench.revision() {
            self.status(cx, "选区原文已变化，请重新选择。");
            return;
        }
        if self.thread.is_some() {
            self.rebind_target = self.thread;
        }
        self.selected = Some(range.clone());
        // Comment anchor and model replacement authority are the exact
        // selected source range, never an implicitly enlarged paragraph.
        self.thread = None;
        let quote = self.workbench.text().get(range).unwrap_or("").to_owned();
        self.composer_ask_ai = self.preferences.default_auto_modify;
        if let Some(mut menu) = self
            .ui
            .widget(cx, ids!(comment_menu))
            .borrow_mut::<context_menu::CommentMenu>()
        {
            menu.hide(cx);
        }
        self.show_thread(cx);
        if let Some(mut composer) = self
            .ui
            .widget(cx, ids!(comment_composer))
            .borrow_mut::<context_menu::CommentComposer>()
        {
            composer.show(cx, position, &quote, self.composer_ask_ai);
        }
    }
    fn hide_composer(&mut self, cx: &mut Cx) {
        if let Some(mut composer) = self
            .ui
            .widget(cx, ids!(comment_composer))
            .borrow_mut::<context_menu::CommentComposer>()
        {
            composer.hide(cx);
        }
    }
    /// Markdown style buttons on the selection toolbar: wrap (or unwrap) the
    /// selected range with the marker and commit through the workbench, so
    /// undo/autosave/comment anchors all follow the normal edit path.
    fn apply_format(&mut self, cx: &mut Cx, marker: &str) {
        let Some((mut range, revision)) = self.popup_selection.clone() else {
            return;
        };
        if revision != self.workbench.revision() {
            self.status(cx, "选区原文已变化，请重新选择。");
            return;
        }
        // Visible selection excludes syntax. Include matching adjacent
        // delimiters for toggle-off, not another nested pair of markers.
        if marker != "link" {
            let close = if marker == "<u>" { "</u>" } else { marker };
            let text = self.workbench.text();
            if range.start >= marker.len()
                && text.get(range.start - marker.len()..range.start) == Some(marker)
                && text.get(range.end..range.end + close.len()) == Some(close)
            {
                range.start -= marker.len();
                range.end += close.len();
            }
        }
        let Some(selected) = self.workbench.text().get(range.clone()) else {
            return;
        };
        let selected = selected.to_owned();
        let replacement = if marker == "link" {
            format!("[{selected}](https://)")
        } else {
            let open = marker;
            let close = match marker {
                "<u>" => "</u>",
                _ => marker,
            };
            if selected.starts_with(open)
                && selected.ends_with(close)
                && selected.len() > open.len() + close.len()
            {
                selected[open.len()..selected.len() - close.len()].to_owned()
            } else {
                format!("{open}{selected}{close}")
            }
        };
        let mut next = self.workbench.text().to_owned();
        next.replace_range(range, &replacement);
        if let Err(e) = self.workbench.edit(next) {
            self.status(cx, &e.to_string());
            return;
        }
        self.popup_selection = None;
        self.selected = None;
        if let Some(mut menu) = self
            .ui
            .widget(cx, ids!(comment_menu))
            .borrow_mut::<context_menu::CommentMenu>()
        {
            menu.hide(cx);
        }
        self.sync_document(cx);
        self.show_thread(cx);
    }
    fn selection_popup(&mut self, cx: &mut Cx, position: DVec2) {
        // The inline composer replaces the selection toolbar while open.
        if self
            .ui
            .widget(cx, ids!(comment_composer))
            .borrow::<context_menu::CommentComposer>()
            .is_some_and(|composer| composer.is_open())
        {
            return;
        }
        let menu = self.ui.widget(cx, ids!(comment_menu));
        if menu
            .borrow::<context_menu::CommentMenu>()
            .is_some_and(|m| m.contains(cx, position))
        {
            return;
        }
        let range = if self.editing {
            self.ui
                .widget(cx, ids!(live_editor))
                .borrow::<live_editor::LiveEditor>()
                .and_then(|e| e.selection(cx))
        } else {
            self.ui
                .widget(cx, ids!(preview))
                .borrow::<reading::Reading>()
                .and_then(|r| r.selected_block(cx, position))
        };
        if let Some(range) = range
            && range.start < range.end
            && self.workbench.text().get(range.clone()).is_some()
        {
            self.popup_selection = Some((range, self.workbench.revision()));
            self.popup_position = Some(position + dvec2(8.0, 14.0));
            if let Some(mut menu) = menu.borrow_mut::<context_menu::CommentMenu>() {
                menu.show(cx, position + dvec2(8.0, 14.0));
                menu.set_agent_visible(cx, self.preferences.default_auto_modify);
                menu.set_format_visible(cx, self.editing);
            }
        } else {
            self.popup_selection = None;
            if let Some(mut menu) = menu.borrow_mut::<context_menu::CommentMenu>() {
                menu.hide(cx);
            }
        }
    }
    fn handle_context(&mut self, cx: &mut Cx, event: &Event) -> bool {
        if let Event::MouseDown(mouse) = event
            && mouse.button == MouseButton::SECONDARY
        {
            let target = if self.editing {
                ids!(live_panel)
            } else {
                ids!(preview_panel)
            };
            if self.ui.widget(cx, target).point_hits_area(cx, mouse.abs) {
                match self.capture_selection(cx, mouse.abs) {
                    Ok(()) => {
                        self.popup_selection = self
                            .selected
                            .clone()
                            .map(|r| (r, self.workbench.revision()));
                        if let Some(mut menu) = self
                            .ui
                            .widget(cx, ids!(comment_menu))
                            .borrow_mut::<context_menu::CommentMenu>()
                        {
                            menu.show(cx, mouse.abs);
                        }
                    }
                    Err(error) => self.status(cx, &error),
                }
                return true; // preserve the selection; do not reposition caret
            }
        }
        false
    }
}
impl MatchEvent for App {
    fn handle_startup(&mut self, cx: &mut Cx) {
        cx.preload_font_set();
        if let Some((width, height)) = std::env::args().find_map(|arg| workspace::window_size(&arg))
        {
            self.ui.window(cx, ids!(main_window)).configure_window(
                cx,
                dvec2(width, height),
                dvec2(0.0, 0.0),
                false,
                "DocGenie".into(),
            );
            self.window_width = width;
            self.layout_sidebars(cx);
        }
        let mut column = self.ui.view(cx, ids!(article_column));
        let width = workspace::ARTICLE_WIDTH;
        script_apply_eval!(cx, column, {max_width: #(width)});
        #[cfg(target_os = "macos")]
        self.ui
            .button(cx, ids!(preferences_button))
            .set_visible(cx, false);
        self.preferences = preferences::load();
        self.auto = self.preferences.default_auto_modify;
        fonts::warm_scan();
        self.apply_preferences(cx);
        self.editing = true;
        match Library::from_env() {
            Ok(library) => {
                let ui = library.load_ui();
                self.expanded = ui.expanded.into_iter().collect();
                self.tabs = ui.tabs;
                self.library = Some(library);
                self.set_right_tab(cx, ui.right_tab == "outline");
                self.launch_library(cx, LibraryCommand::Initialize(ui.active));
            }
            Err(e) => {
                self.save_failed = true;
                self.status(cx, &e.to_string());
            }
        }
    }
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        for (id, command) in [
            (
                ids!(table_row_above),
                document_core::table::Command::RowAbove,
            ),
            (
                ids!(table_row_below),
                document_core::table::Command::RowBelow,
            ),
            (
                ids!(table_delete_row),
                document_core::table::Command::DeleteRow,
            ),
            (
                ids!(table_column_left),
                document_core::table::Command::ColumnLeft,
            ),
            (
                ids!(table_column_right),
                document_core::table::Command::ColumnRight,
            ),
            (
                ids!(table_delete_column),
                document_core::table::Command::DeleteColumn,
            ),
        ] {
            if self.ui.button(cx, id).clicked(actions)
                && let Some((range, row, column, revision)) = self.table_target.clone()
            {
                self.hide_table_menu(cx);
                if !self.editing
                    || self.document.is_none()
                    || self.pending_switch.is_some()
                    || self.pending_tabs.is_some()
                    || self.closing
                    || self.save_failed
                {
                    return;
                }
                match self
                    .workbench
                    .edit_table(revision, range, row, column, command)
                {
                    Ok(()) => {
                        self.selected = None;
                        self.popup_selection = None;
                        self.sync_document(cx);
                        self.show_thread(cx);
                        self.queue_save(cx);
                        self.status(cx, "表格已更新 · 可撤销");
                    }
                    Err(error) => self.status(cx, &error.to_string()),
                }
            }
        }
        if self.ui.button(cx, ids!(toggle_navigation)).clicked(actions)
            || self.ui.button(cx, ids!(expand_navigation)).clicked(actions)
        {
            let shown = self.ui.view(cx, ids!(navigation_panel)).visible();
            let comments = !self.comments_collapsed
                && (shown || self.window_width == 0.0 || self.window_width >= 1100.0);
            self.set_sidebar_visibility(cx, !shown, comments);
        }
        if self.ui.button(cx, ids!(toggle_comments)).clicked(actions)
            || self.ui.button(cx, ids!(expand_comments)).clicked(actions)
        {
            let shown = self.ui.view(cx, ids!(comments_panel)).visible();
            self.set_sidebar_visibility(cx, !self.navigation_collapsed, !shown);
        }
        for (id, command) in [
            (ids!(tab_close), tabs::Close::Current),
            (ids!(tab_close_left), tabs::Close::Left),
            (ids!(tab_close_others), tabs::Close::Others),
            (ids!(tab_close_all), tabs::Close::All),
        ] {
            if self.ui.button(cx, id).clicked(actions)
                && let Some(target) = self.tab_menu_target.take()
            {
                self.hide_tab_menu(cx);
                self.close_tabs(cx, &target, command);
            }
        }
        if self.ui.button(cx, ids!(empty_new)).clicked(actions)
            || (self.document.is_none() && self.ui.button(cx, ids!(new_document)).clicked(actions))
        {
            self.switch_document(cx, LibraryCommand::CreateNote(self.target_dir()));
        }
        // Preferences overlay: Obsidian-style, every control applies the
        // moment it changes (no save/cancel step).
        if self
            .ui
            .button(cx, ids!(preferences_button))
            .clicked(actions)
        {
            self.toggle_preferences(cx);
        }
        if self.ui.button(cx, ids!(preferences_done)).clicked(actions) {
            self.hide_preferences(cx);
        }
        for (page, nav) in [
            (0, ids!(pref_nav_appearance)),
            (1, ids!(pref_nav_editor)),
            (2, ids!(pref_nav_agent)),
        ] {
            if self.preferences_visible && self.ui.button(cx, nav).clicked(actions) {
                self.set_preferences_page(cx, page);
            }
        }
        if let Some(active) = self
            .ui
            .check_box(cx, ids!(pref_default_auto))
            .changed(actions)
        {
            self.preferences.default_auto_modify = active;
            self.auto = active;
            self.save_preferences(cx);
        }
        if let Some(active) = self.ui.check_box(cx, ids!(pref_auto_save)).changed(actions) {
            self.preferences.auto_save_enabled = active;
            self.save_preferences(cx);
            // Both directions matter: disable cancels an existing deadline,
            // enable starts one for the current dirty document.
            self.queue_save(cx);
        }
        if self.preferences_visible
            && let Some(changed) = self
                .ui
                .text_input(cx, ids!(pref_font_size))
                .changed(actions)
        {
            match changed.trim().parse::<u32>() {
                Ok(value) if (8..=48).contains(&value) => {
                    self.preferences.editor_font_size = value;
                    self.save_preferences(cx);
                    self.apply_preferences(cx);
                    self.ui.label(cx, ids!(pref_size_status)).set_text(cx, "");
                }
                _ => {
                    self.ui
                        .label(cx, ids!(pref_size_status))
                        .set_text(cx, "字号需在 8–48 之间。");
                }
            }
        }
        // Code-font priority list: reorder, remove, add — each applied live.
        let mut font_edit: Option<(usize, i32)> = None; // (index, -1 up, +1 down, 0 remove)
        for i in 0..FONT_ROW_SLOTS.min(self.preferences.editor_code_fonts.len()) {
            let row = &[font_row_id(i)];
            if self
                .ui
                .button(cx, &[row[0], id!(font_remove)])
                .clicked(actions)
            {
                font_edit = Some((i, 0));
            } else if self.ui.button(cx, &[row[0], id!(font_up)]).clicked(actions) {
                font_edit = Some((i, -1));
            } else if self
                .ui
                .button(cx, &[row[0], id!(font_down)])
                .clicked(actions)
            {
                font_edit = Some((i, 1));
            }
        }
        let dropdown = self.ui.drop_down(cx, ids!(pref_font_add));
        let picked: Option<String> = actions
            .filter_widget_actions_cast::<DropDownAction>(dropdown.widget_uid())
            .find_map(|action| match action {
                DropDownAction::Select(index) if index > 0 => {
                    let labels = fonts::menu_names();
                    let listed = &self.preferences.editor_code_fonts;
                    // The dropdown shows the same filtered list sync_font_rows
                    // built: installed families not already in the priority list.
                    Some(
                        labels
                            .into_iter()
                            .filter(|name| !listed.contains(name))
                            .nth(index - 1),
                    )
                    .flatten()
                }
                _ => None,
            });
        if let Some(name) = picked {
            if self.preferences.editor_code_fonts.len() < FONT_ROW_SLOTS {
                self.preferences.editor_code_fonts.push(name);
                self.save_preferences(cx);
                self.sync_font_rows(cx);
                self.apply_code_font_pref(cx);
            } else {
                self.ui
                    .label(cx, ids!(pref_font_status))
                    .set_text(cx, "最多 8 个代码字体。");
            }
            self.ui
                .drop_down(cx, ids!(pref_font_add))
                .set_selected_item(cx, 0);
        }
        if let Some((index, op)) = font_edit {
            {
                let fonts = &mut self.preferences.editor_code_fonts;
                match op {
                    0 => {
                        fonts.remove(index);
                    }
                    -1 if index > 0 => {
                        fonts.swap(index, index - 1);
                    }
                    1 if index + 1 < fonts.len() => {
                        fonts.swap(index, index + 1);
                    }
                    _ => {}
                }
            }
            self.save_preferences(cx);
            self.sync_font_rows(cx);
            self.apply_code_font_pref(cx);
        }
        // Feishu-style big title: a single TextInput that maps to the first
        // `# heading` line. Empty input deletes that line; non-empty input
        // creates/replaces it. The first-line edit is local to the workbench
        // text, so it shares the same revision/undo pipeline.
        if self.document.is_none()
            && (self.ui.button(cx, ids!(empty_open)).clicked(actions)
                || self.ui.button(cx, ids!(document_search)).clicked(actions))
        {
            self.show_switcher(cx);
        }
        if self.document.is_none() {
            let tree = self.ui.widget(cx, ids!(document_list));
            for (id, row) in tree.portal_list(cx, ids!(list)).items_with_actions(actions) {
                let node = tree_rows().lock().unwrap().get(id).cloned();
                if row.button(cx, ids!(heading)).clicked(actions)
                    && let Some(node) = node
                {
                    if node.is_dir {
                        self.toggle_folder(cx, &node.rel);
                    } else {
                        self.open_rel(cx, &node.rel);
                    }
                }
            }
        }
        if self.document.is_none() && self.switcher_visible {
            if let Some(query) = self
                .ui
                .text_input(cx, ids!(switcher_input))
                .changed(actions)
            {
                self.update_switcher(cx, &query);
            }
            for i in 0..SWITCH_SLOTS {
                if self.ui.button(cx, &[switch_id(i)]).clicked(actions)
                    && let Some(rel) = self.switcher_hits.get(i).cloned()
                {
                    self.hide_switcher(cx);
                    self.open_rel(cx, &rel);
                }
            }
        }
        if self.document.is_none()
            || self.pending_switch.is_some()
            || self.pending_tabs.is_some()
            || self.closing
        {
            return;
        }
        let before = self.snapshot();
        if self.ui.button(cx, ids!(document_search)).clicked(actions) {
            self.show_switcher(cx);
        }
        for (id, resolved) in [(ids!(filter_open), false), (ids!(filter_resolved), true)] {
            if self.ui.button(cx, id).clicked(actions) {
                self.filter_resolved = resolved;
                self.show_thread(cx);
            }
        }
        let cards = self
            .ui
            .widget(cx, ids!(threads))
            .borrow::<thread_list::ThreadList>()
            .map(|list| list.cards(cx))
            .unwrap_or_default();
        for (id, row) in cards {
            if row.button(cx, ids!(view_change)).clicked(actions)
                && let Some(mut list) = self
                    .ui
                    .widget(cx, ids!(threads))
                    .borrow_mut::<thread_list::ThreadList>()
            {
                list.toggle_change(cx, id);
            }
            if let Some(draft) = row.text_input(cx, ids!(comment_input)).changed(actions) {
                self.reply_drafts.insert(Some(id), draft);
                row.button(cx, ids!(comment_send)).set_enabled(
                    cx,
                    !row.text_input(cx, ids!(comment_input))
                        .text()
                        .trim()
                        .is_empty(),
                );
            }
            if row.button(cx, ids!(quote_jump)).clicked(actions) {
                self.thread = Some(id);
                self.selected = None;
                self.rebind_target = None;
                self.show_thread(cx);
                self.reveal_active_thread(cx);
            }
            if row.button(cx, ids!(thread_resolve_toggle)).clicked(actions) {
                if self.workbench.thread(id).is_some_and(|t| t.resolved()) {
                    let _ = self.workbench.reopen_thread(id);
                } else {
                    let _ = self.workbench.resolve_thread(id);
                }
                if self.agent_states.get(&id).is_some_and(|p| p.busy()) {
                    self.agent_states
                        .insert(id, agent_progress::Progress::Ignored);
                }
                self.thread = Some(id);
                self.show_thread(cx);
            }
            if row.button(cx, ids!(thread_rebind)).clicked(actions)
                && let Some(range) = self.selected.clone()
            {
                match self.workbench.rebind_thread(id, range) {
                    Ok(()) => {
                        self.selected = None;
                        self.rebind_target = None;
                        self.thread = Some(id);
                        self.show_thread(cx);
                    }
                    Err(e) => self.status(cx, &e.to_string()),
                }
            }
            if row.button(cx, ids!(comment_send)).clicked(actions) {
                let text = row.text_input(cx, ids!(comment_input)).text();
                match self.workbench.reply(id, &text) {
                    Ok(()) => {
                        self.stamp_message(id, "你");
                        self.reply_drafts.remove(&Some(id));
                        row.text_input(cx, ids!(comment_input)).set_text(cx, "");
                        self.thread = Some(id);
                        self.show_thread(cx);
                        if self.auto || self.workbench.thread(id).is_some_and(|t| t.ask_ai()) {
                            self.run_agent(cx);
                        }
                    }
                    Err(e) => self.status(cx, &e.to_string()),
                }
            }
        }
        if let Some(action) =
            actions.find_widget_action(self.ui.widget(cx, ids!(live_editor)).widget_uid())
            && let live_editor::LiveAction::SelectionReady(point) = action.cast()
        {
            self.selection_popup(cx, point);
        }
        if let Some(action) =
            actions.find_widget_action(self.ui.widget(cx, ids!(live_editor)).widget_uid())
            && let live_editor::LiveAction::Rejected = action.cast()
        {
            self.status(
                cx,
                "此编辑涉及格式或表格结构边界，未修改；请在同一格式内编辑，表格暂不支持原始 | 或换行。",
            );
        }
        if self.ui.button(cx, ids!(retry_save)).clicked(actions) {
            self.save_failed = false;
            self.flush_save(cx);
        }
        let live_change = actions
            .find_widget_action(self.ui.widget(cx, ids!(live_editor)).widget_uid())
            .and_then(|a| match a.cast::<live_editor::LiveAction>() {
                live_editor::LiveAction::Changed(text) => Some(text),
                _ => None,
            });
        if let Some(text) = live_change {
            if let Err(e) = self.workbench.edit(text) {
                self.status(cx, &e.to_string());
            }
            self.selected = None;
            if let Some(mut menu) = self
                .ui
                .widget(cx, ids!(comment_menu))
                .borrow_mut::<context_menu::CommentMenu>()
            {
                menu.hide(cx);
            }
            self.hide_composer(cx);
            self.sync_document(cx);
            self.show_thread(cx);
        }
        if let Some(action) =
            actions.find_widget_action(self.ui.widget(cx, ids!(preview)).widget_uid())
            && let reading::ReadingAction::Open(id) = action.cast()
        {
            self.thread = Some(id);
            self.filter_resolved = false;
            self.selected = None;
            self.set_sidebar_visibility(cx, !self.navigation_collapsed, true);
            self.set_right_tab(cx, false);
            self.show_thread(cx);
            self.reveal_active_thread(cx);
        }
        if self.rebind_target.is_some()
            && self.ui.button(cx, ids!(thread_rebind)).clicked(actions)
            && let Some(range) = self.selected.clone()
            && let Some(id) = self.rebind_target
        {
            match self.workbench.rebind_thread(id, range) {
                Ok(()) => {
                    self.thread = Some(id);
                    self.selected = None;
                    self.rebind_target = None;
                    self.show_thread(cx);
                    self.status(cx, "线程已绑定新段落，请审阅后再处理。");
                }
                Err(e) => self.status(cx, &e.to_string()),
            }
        }
        if self.ui.button(cx, ids!(new_document)).clicked(actions) {
            let dir = self.target_dir();
            if !dir.is_empty() {
                self.expanded.insert(dir.clone());
            }
            self.switch_document(cx, LibraryCommand::CreateNote(dir));
        }
        if self.ui.button(cx, ids!(new_folder)).clicked(actions) && self.tree_idle(cx) {
            self.create_folder(cx);
        }
        if self.ui.button(cx, ids!(rename_node)).clicked(actions)
            && let Some(rel) = self.selected_node.clone()
            && self.tree_idle(cx)
        {
            self.show_dialog(cx, Dialog::Rename(rel));
        }
        if self.ui.button(cx, ids!(delete_node)).clicked(actions)
            && let Some(rel) = self.selected_node.clone()
            && self.tree_idle(cx)
        {
            self.show_dialog(cx, Dialog::Delete(rel));
        }
        if self.ui.button(cx, ids!(tab_outline)).clicked(actions) {
            self.set_right_tab(cx, true);
        }
        if self.ui.button(cx, ids!(tab_comments)).clicked(actions) {
            self.set_right_tab(cx, false);
        }
        let docs = self
            .ui
            .widget(cx, ids!(document_list))
            .portal_list(cx, ids!(list));
        for (id, item) in docs.items_with_actions(actions) {
            let clicked = item.button(cx, ids!(heading)).clicked(actions)
                || item.button(cx, ids!(fold_open)).clicked(actions)
                || item.button(cx, ids!(fold_closed)).clicked(actions);
            if clicked {
                let row = tree_rows().lock().unwrap().get(id).cloned();
                if let Some(row) = row {
                    if row.is_dir {
                        self.toggle_folder(cx, &row.rel);
                    } else {
                        self.selected_node = Some(row.rel.clone());
                        self.open_rel(cx, &row.rel);
                    }
                }
            }
        }
        for i in 0..MAX_TABS {
            let tab = self.ui.view(cx, &[tab_id(i)]);
            if tab.button(cx, ids!(close)).clicked(actions) {
                self.close_tab(cx, i);
                break;
            }
            if tab.button(cx, ids!(title)).clicked(actions)
                && let Some(rel) = self.tabs.get(i).cloned()
            {
                self.selected_node = Some(rel.clone());
                self.open_rel(cx, &rel);
                break;
            }
        }
        if self.dialog.is_some() {
            if self.ui.button(cx, ids!(dialog_cancel)).clicked(actions) {
                self.hide_dialog(cx);
            } else if self.ui.button(cx, ids!(dialog_ok)).clicked(actions)
                || self
                    .ui
                    .text_input(cx, ids!(dialog_input))
                    .returned(actions)
                    .is_some()
            {
                self.commit_dialog(cx);
            }
        }
        if self.switcher_visible {
            if let Some(query) = self
                .ui
                .text_input(cx, ids!(switcher_input))
                .changed(actions)
            {
                self.update_switcher(cx, &query);
            }
            let mut target = self
                .ui
                .text_input(cx, ids!(switcher_input))
                .returned(actions)
                .and_then(|_| self.switcher_hits.first().cloned());
            for i in 0..SWITCH_SLOTS {
                if self.ui.button(cx, &[switch_id(i)]).clicked(actions) {
                    target = self.switcher_hits.get(i).cloned();
                }
            }
            if let Some(rel) = target {
                self.hide_switcher(cx);
                self.selected_node = Some(rel.clone());
                self.open_rel(cx, &rel);
            }
        }
        if self.ui.button(cx, ids!(mode_edit)).clicked(actions) {
            self.mode(cx, false);
        }
        if self.ui.button(cx, ids!(mode_read)).clicked(actions) {
            self.mode(cx, true);
        }
        let list = self
            .ui
            .widget(cx, ids!(navigation))
            .portal_list(cx, ids!(list));
        for (id, item) in list.items_with_actions(actions) {
            if item.button(cx, ids!(heading)).clicked(actions) {
                let entry = outline().lock().unwrap().get(id).cloned();
                if let Some(entry) = entry {
                    if self.editing {
                        if let Some(mut live) = self
                            .ui
                            .widget(cx, ids!(live_editor))
                            .borrow_mut::<live_editor::LiveEditor>()
                        {
                            live.activate_at(cx, entry.range.start);
                        }
                    } else if let Some(mut reading) = self
                        .ui
                        .widget(cx, ids!(preview))
                        .borrow_mut::<reading::Reading>()
                    {
                        reading.reveal(cx, entry.range);
                    }
                }
            }
        }
        if self.ui.button(cx, ids!(add_comment)).clicked(actions) {
            let position = self.popup_position.unwrap_or(dvec2(120.0, 160.0));
            self.open_composer(cx, position);
        }
        // Format toolbar on the selection popup (edit mode only).
        for (id, marker) in [
            (ids!(fmt_bold), "**"),
            (ids!(fmt_italic), "*"),
            (ids!(fmt_underline), "<u>"),
            (ids!(fmt_code), "`"),
            (ids!(fmt_link), "link"),
        ] {
            if self.ui.button(cx, id).clicked(actions) {
                self.apply_format(cx, marker);
            }
        }
        if self.ui.button(cx, ids!(composer_cancel)).clicked(actions) {
            self.hide_composer(cx);
        }
        if let Some(active) = self
            .ui
            .check_box(cx, ids!(composer_ask_ai))
            .changed(actions)
        {
            self.composer_ask_ai = active;
        }
        if self.ui.button(cx, ids!(composer_submit)).clicked(actions) {
            let text = self.ui.text_input(cx, ids!(composer_input)).text();
            let result = match self.popup_selection.clone() {
                Some((range, revision)) if revision == self.workbench.revision() => self
                    .workbench
                    .comment_with_ai(range, &text, self.composer_ask_ai)
                    .map_err(|e| e.to_string()),
                Some(_) => Err("选区原文已变化，请重新选择。".to_string()),
                None => Err("请先在文档中选中要评论的文字".to_string()),
            };
            match result {
                Ok(id) => {
                    self.stamp_message(id, "你");
                    self.thread = Some(id);
                    self.filter_resolved = false;
                    self.selected = None;
                    self.popup_selection = None;
                    self.rebind_target = None;
                    self.hide_composer(cx);
                    self.set_sidebar_visibility(cx, !self.navigation_collapsed, true);
                    self.set_right_tab(cx, false);
                    self.show_thread(cx);
                    if self.composer_ask_ai || self.auto {
                        self.run_agent(cx);
                    }
                }
                Err(e) => self.status(cx, &e),
            }
        }
        // Feishu-style toolbar Agent button: same code path as the old
        // `agent_run` button but gated by Preferences.default_auto_modify.
        // Visible only when Preferences allow the agent (see selection_popup).
        if self.ui.button(cx, ids!(toolbar_agent)).clicked(actions) {
            if !self.preferences.default_auto_modify {
                self.status(cx, "先在 Preferences 中启用默认自动修改 (⌘,)");
            } else {
                let _ = self.popup_selection.take();
                self.run_agent(cx);
            }
            if let Some(mut menu) = self
                .ui
                .widget(cx, ids!(comment_menu))
                .borrow_mut::<context_menu::CommentMenu>()
            {
                menu.hide(cx);
            }
        }
        if self.ui.button(cx, ids!(comment_new)).clicked(actions) {
            self.thread = None;
            self.selected = None;
            self.rebind_target = None;
            self.show_thread(cx);
        }
        if self.thread.is_none() && self.ui.button(cx, ids!(comment_send)).clicked(actions) {
            let text = self.ui.text_input(cx, ids!(comment_input)).text();
            let result = if let Some(id) = self.thread {
                self.workbench.reply(id, &text)
            } else {
                self.selected
                    .clone()
                    .ok_or(document_core::EditError::InvalidSelection)
                    .and_then(|r| self.workbench.comment_with_ai(r, &text, self.auto))
                    .map(|id| self.thread = Some(id))
            };
            match result {
                Ok(()) => {
                    if let Some(id) = self.thread {
                        self.stamp_message(id, "你");
                    }
                    self.ui.text_input(cx, ids!(comment_input)).set_text(cx, "");
                    self.rebind_target = None;
                    self.hide_composer(cx);
                    self.show_thread(cx);
                    let wants_agent = self.auto
                        || self
                            .thread
                            .and_then(|id| self.workbench.thread(id))
                            .is_some_and(|t| t.ask_ai());
                    if wants_agent {
                        self.run_agent(cx);
                    }
                }
                Err(e) => self.status(cx, &e.to_string()),
            }
        }
        if self.ui.button(cx, ids!(thread_prev)).clicked(actions) {
            self.thread = self.thread.map(|id| id.saturating_sub(1));
            self.show_thread(cx);
            self.reveal_active_thread(cx);
        }
        if self.ui.button(cx, ids!(thread_next)).clicked(actions) {
            if self.workbench.thread_count() > 0 {
                self.thread =
                    Some((self.thread.unwrap_or(0) + 1).min(self.workbench.thread_count() - 1));
            }
            self.show_thread(cx);
            self.reveal_active_thread(cx);
        }
        // thread_resolve_toggle is wired once above; no second handler.
        if self.ui.button(cx, ids!(agent_cancel)).clicked(actions) {
            self.auto = false;
            for state in self.agent_states.values_mut() {
                if state.busy() {
                    *state = agent_progress::Progress::Cancelled;
                }
            }
            self.show_thread(cx);
            self.agent_queue.clear();
            self.epoch += 1;
            self.status(cx, "自动修改已停止；进行中结果将忽略。");
        }
        if self.ui.button(cx, ids!(undo_button)).clicked(actions)
            || actions
                .find_widget_action(self.ui.widget(cx, ids!(live_editor)).widget_uid())
                .is_some_and(|a| {
                    matches!(
                        a.cast::<live_editor::LiveAction>(),
                        live_editor::LiveAction::UndoRequested
                    )
                })
        {
            self.undo_document(cx);
        }
        let title = self.ui.text_input(cx, ids!(title_input));
        if actions
            .filter_widget_actions_cast::<TextInputAction>(title.widget_uid())
            .any(|action| {
                matches!(
                    action,
                    TextInputAction::KeyFocus | TextInputAction::KeyFocusLost
                )
            })
        {
            self.sync_document(cx);
        }
        if let Some(text) = self.ui.text_input(cx, ids!(title_input)).changed(actions) {
            self.apply_title_change(cx, &text);
        }
        if before != self.snapshot() {
            self.queue_save(cx);
        }
        self.sync_status(cx);
    }
    fn handle_timer(&mut self, cx: &mut Cx, event: &TimerEvent) {
        if self.hint_timer.0 != 0 && self.hint_timer.0 == event.timer_id {
            self.hint_timer = Timer::default();
            if let Some((text, point)) = &self.hint_target
                && let Some(mut hint) = self
                    .ui
                    .widget(cx, ids!(ui_hint))
                    .borrow_mut::<ui_hint::UiHint>()
            {
                hint.show(cx, *point, text);
            }
        }
        if self.save_timer.0 != 0 && self.save_timer.0 == event.timer_id {
            self.save_timer = Timer::default();
            self.flush_save(cx);
            // A busy worker leaves the expired deadline intact; poll_file
            // will flush it on completion, without restarting the clock.
        }
    }
}
impl AppMain for App {
    fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
        makepad_widgets::theme_mod(vm);
        script_eval!(vm, { mod.theme = mod.themes.light });
        makepad_widgets::widgets_mod(vm);
        context_menu::script_mod(vm);
        discussion::script_mod(vm);
        comment_quote::script_mod(vm);
        change_view::script_mod(vm);
        thread_list::script_mod(vm);
        tab_menu::script_mod(vm);
        table_menu::script_mod(vm);
        styled_input::script_mod(vm);
        markdown::script_mod(vm);
        live_editor::script_mod(vm);
        reading::script_mod(vm);
        ui_hint::script_mod(vm);
        ui::script_mod(vm)
    }
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::MouseMove(mouse) = event
            && !cx
                .fingers
                .is_area_captured(self.ui.widget(cx, ids!(live_editor)).area())
        {
            self.hover_hint(cx, mouse.abs);
        }
        if matches!(
            event,
            Event::MouseDown(_) | Event::Scroll(_) | Event::KeyDown(_) | Event::WindowGeomChange(_)
        ) {
            self.clear_hint(cx);
        }
        if let Event::QuitRequested(quit) = event
            && (self.file_rx.is_some()
                || (self.document.is_some() && self.persisted.as_ref() != Some(&self.snapshot())))
        {
            quit.handle();
            if self.save_failed {
                self.status(cx, "保存失败，退出已取消；请重试保存。");
                return;
            }
            self.pending_switch = None;
            self.pending_tabs = None;
            self.quitting = true;
            self.closing = true;
            self.epoch += 1;
            self.readonly(cx, true);
            self.flush_save(cx);
            return;
        }
        if let Event::MouseDown(mouse) = event
            && mouse.button == MouseButton::SECONDARY
            && self.editing
        {
            let target = self
                .ui
                .widget(cx, ids!(live_editor))
                .borrow::<live_editor::LiveEditor>()
                .and_then(|live| live.table_cell_at(cx, mouse.abs));
            if let Some((range, row, column)) = target {
                self.hide_table_menu(cx);
                self.hide_tab_menu(cx);
                self.hide_composer(cx);
                if let Some(mut menu) = self
                    .ui
                    .widget(cx, ids!(comment_menu))
                    .borrow_mut::<context_menu::CommentMenu>()
                {
                    menu.hide(cx);
                }
                self.table_target = Some((range.clone(), row, column, self.workbench.revision()));
                self.table_opening = true;
                if let Some(mut menu) = self
                    .ui
                    .widget(cx, ids!(table_menu))
                    .borrow_mut::<table_menu::TableMenu>()
                {
                    menu.show(cx, mouse.abs);
                }
                let columns = document_core::table::Table::parse(&self.workbench.text()[range])
                    .ok()
                    .map_or(0, |t| t.cells[0].len());
                self.ui
                    .button(cx, ids!(table_row_above))
                    .set_enabled(cx, row > 0);
                self.ui
                    .button(cx, ids!(table_delete_row))
                    .set_enabled(cx, row > 0);
                self.ui
                    .button(cx, ids!(table_delete_column))
                    .set_enabled(cx, columns > 1);
                return;
            }
        }
        if self.table_target.is_some() {
            if self.table_opening && matches!(event, Event::MouseUp(_)) {
                self.table_opening = false;
                return;
            }
            if matches!(event, Event::Scroll(_)) {
                return;
            }
            let inside = matches!(event,Event::MouseDown(m) if self.ui.widget(cx,ids!(table_menu)).borrow::<table_menu::TableMenu>().is_some_and(|menu|menu.contains(cx,m.abs)));
            if inside || self.table_pointer {
                if matches!(event, Event::MouseDown(_)) {
                    self.table_pointer = true;
                }
                self.ui
                    .widget(cx, ids!(table_menu))
                    .handle_event(cx, event, &mut Scope::empty());
                if matches!(event, Event::MouseUp(_)) {
                    self.table_pointer = false;
                }
                return;
            }
            if matches!(event, Event::MouseDown(_) | Event::KeyDown(_)) {
                self.hide_table_menu(cx);
            }
        }
        if let Event::MouseDown(mouse) = event
            && mouse.button == MouseButton::SECONDARY
        {
            for i in 0..self.tabs.len() {
                if self
                    .ui
                    .view(cx, &[tab_id(i)])
                    .point_hits_area(cx, mouse.abs)
                {
                    self.hide_tab_menu(cx);
                    self.tab_menu_target = Some(self.tabs[i].clone());
                    self.tab_menu_opening = true;
                    if let Some(mut menu) = self
                        .ui
                        .widget(cx, ids!(tab_menu))
                        .borrow_mut::<tab_menu::TabMenu>()
                    {
                        menu.show(cx, mouse.abs);
                    }
                    self.ui
                        .button(cx, ids!(tab_close_left))
                        .set_enabled(cx, i > 0);
                    self.ui
                        .button(cx, ids!(tab_close_others))
                        .set_enabled(cx, self.tabs.len() > 1);
                    return;
                }
            }
        }
        if self.tab_menu_target.is_some() {
            // The opening right-click release belongs to the opener, not a
            // menu item or the document below it. Trackpad inertia is not a
            // dismissal command either; keep the overlay until explicit exit.
            if self.tab_menu_opening && matches!(event, Event::MouseUp(_)) {
                self.tab_menu_opening = false;
                return;
            }
            if matches!(event, Event::Scroll(_)) {
                return;
            }
            let inside = match event {
                Event::MouseDown(m) => self
                    .ui
                    .widget(cx, ids!(tab_menu))
                    .borrow::<tab_menu::TabMenu>()
                    .is_some_and(|menu| menu.contains(cx, m.abs)),
                _ => false,
            };
            if inside || self.tab_menu_pointer {
                if matches!(event, Event::MouseDown(_)) {
                    self.tab_menu_pointer = true;
                }
                self.ui
                    .widget(cx, ids!(tab_menu))
                    .handle_event(cx, event, &mut Scope::empty());
                if matches!(event, Event::MouseUp(_)) {
                    self.tab_menu_pointer = false;
                }
                return;
            }
            if matches!(
                event,
                Event::KeyDown(key) if key.key_code==KeyCode::Escape
            ) || matches!(event, Event::MouseDown(_))
            {
                self.hide_tab_menu(cx);
            }
        }
        if let Event::KeyDown(key) = event
            && key.key_code == KeyCode::KeyV
            && (key.modifiers.logo || key.modifiers.control)
            && !key.is_repeat
            && self.editing
            && self.document.is_some()
            && self.pending_switch.is_none()
            && !self.closing
            && self
                .ui
                .widget(cx, ids!(live_editor))
                .borrow::<live_editor::LiveEditor>()
                .is_some_and(|editor| editor.focused(cx))
        {
            match clipboard::pasted_image() {
                Ok(Some(png)) => {
                    let result = self
                        .library
                        .as_ref()
                        .ok_or(project_store::StoreError::InvalidPath)
                        .and_then(|lib| lib.store_image(&png));
                    match result {
                        Ok(rel) => self.ui.handle_event(
                            cx,
                            &Event::TextInput(TextInputEvent {
                                input: format!("\n\n![图片]({rel})\n\n"),
                                was_paste: true,
                                ..Default::default()
                            }),
                            &mut Scope::empty(),
                        ),
                        Err(_) => self.status(cx, "图片保存失败，未插入正文"),
                    }
                    return;
                }
                Err(message) => {
                    self.status(cx, message);
                    return;
                }
                Ok(None) => {}
            }
        }
        if let Event::MouseDown(mouse) = event
            && mouse.button == MouseButton::PRIMARY
        {
            self.popup_pointer = self
                .ui
                .widget(cx, ids!(comment_menu))
                .borrow::<context_menu::CommentMenu>()
                .is_some_and(|menu| menu.contains(cx, mouse.abs));
            self.composer_pointer = self
                .ui
                .widget(cx, ids!(comment_composer))
                .borrow::<context_menu::CommentComposer>()
                .is_some_and(|composer| composer.contains(cx, mouse.abs));
        }
        // Overlay gets pointer events before the underlying live-preview row.
        // Otherwise its first MouseDown can activate a different source line
        // and swallow the toolbar click as though it were a document gesture.
        if self.popup_pointer
            && matches!(
                event,
                Event::MouseDown(_) | Event::MouseMove(_) | Event::MouseUp(_)
            )
        {
            self.ui
                .widget(cx, ids!(comment_menu))
                .handle_event(cx, event, &mut Scope::empty());
            if matches!(event, Event::MouseUp(_)) {
                self.popup_pointer = false;
            }
            return;
        }
        if self.composer_pointer
            && matches!(
                event,
                Event::MouseDown(_) | Event::MouseMove(_) | Event::MouseUp(_)
            )
        {
            self.ui
                .widget(cx, ids!(comment_composer))
                .handle_event(cx, event, &mut Scope::empty());
            if matches!(event, Event::MouseUp(_)) {
                self.composer_pointer = false;
            }
            return;
        }
        if self.handle_context(cx, event) {
            return;
        }
        if let Event::WindowCloseRequested(close) = event
            && (self.persisted.as_ref() != Some(&self.snapshot()) || self.file_rx.is_some())
        {
            close.accept_close.set(false);
            if self.save_failed {
                self.status(cx, "保存失败，关闭已取消；请处理错误并重试保存。");
                return;
            }
            self.epoch += 1;
            self.readonly(cx, true);
            self.closing = true;
            self.flush_save(cx);
            return;
        }
        if matches!(event, Event::MacosMenuCommand(command) if *command == id!(undo)) {
            self.undo_document(cx);
            return;
        }
        if let Event::KeyDown(key) = event
            && key.key_code == KeyCode::KeyZ
            && key.modifiers.is_primary()
            && !key.modifiers.shift
            && !key.is_repeat
            && !self.preferences_visible
            && !self.switcher_visible
            && self.dialog.is_none()
            && !self
                .ui
                .text_input(cx, ids!(title_input))
                .borrow()
                .is_some_and(|i| i.key_focus(cx))
            && !self
                .ui
                .widget(cx, ids!(threads))
                .borrow::<thread_list::ThreadList>()
                .is_some_and(|list| list.reply_focused(cx))
            && !self
                .ui
                .text_input(cx, ids!(comment_input))
                .borrow()
                .is_some_and(|i| i.key_focus(cx))
            && !self
                .ui
                .widget(cx, ids!(comment_composer))
                .borrow::<context_menu::CommentComposer>()
                .is_some_and(|c| c.is_open())
            && !self
                .ui
                .widget(cx, ids!(live_editor))
                .borrow::<live_editor::LiveEditor>()
                .is_some_and(|e| e.focused(cx))
        {
            self.undo_document(cx);
            return;
        }
        if matches!(event, Event::MacosMenuCommand(command) if *command == id!(settings))
            || matches!(event, Event::KeyDown(key) if !key.is_repeat && live_editor::preferences_key(key))
        {
            self.show_preferences(cx);
            return;
        }
        if let Event::KeyDown(key) = event
            && !key.is_repeat
            && key.key_code == KeyCode::Escape
            && self
                .ui
                .widget(cx, ids!(comment_composer))
                .borrow::<context_menu::CommentComposer>()
                .is_some_and(|composer| composer.is_open())
        {
            self.hide_composer(cx);
            return;
        }
        if let Event::KeyDown(key) = event
            && !key.is_repeat
            && key.key_code == KeyCode::Escape
            && self.preferences_visible
        {
            self.hide_preferences(cx);
            return;
        }
        if let Event::KeyDown(key) = event
            && !key.is_repeat
            && key.key_code == KeyCode::Escape
            && (self.dialog.is_some() || self.switcher_visible)
        {
            self.hide_dialog(cx);
            self.hide_switcher(cx);
            return;
        }
        if let Event::KeyDown(key) = event
            && !key.is_repeat
            && live_editor::switcher_key(key)
            && self.dialog.is_none()
            && !self.preferences_visible
        {
            if self.switcher_visible {
                self.hide_switcher(cx);
            } else {
                self.show_switcher(cx);
            }
            return;
        }
        if let Event::KeyDown(key) = event
            && !key.is_repeat
            && live_editor::comment_shortcut_key(key)
        {
            self.comment_shortcut_action(cx);
            return;
        }
        if let Event::WindowGeomChange(change) = event
            && self.ui.window(cx, ids!(main_window)).window_id() == Some(change.window_id)
        {
            self.window_width = change.new_geom.inner_size.x;
            self.layout_sidebars(cx);
        }
        self.match_event(cx, event);
        if matches!(event, Event::LiveEdit | Event::ScriptReapply) {
            #[cfg(target_os = "macos")]
            self.ui
                .button(cx, ids!(preferences_button))
                .set_visible(cx, false);
            // DSL reload resets declarative text/defaults, not Rust truth.
            // Rehydrate runtime projections without Startup or IO/AI commands.
            self.apply_preferences(cx);
            self.sync_document(cx);
            self.ui
                .view(cx, ids!(article_surface))
                .set_visible(cx, self.document.is_some());
            self.ui
                .view(cx, ids!(empty_workspace))
                .set_visible(cx, self.document.is_none());
            self.sync_status(cx);
            self.layout_sidebars(cx);
        }
        if matches!(event, Event::Signal) && !self.fonts_ready && fonts::ready() {
            self.fonts_ready = true;
            self.apply_code_font_pref(cx);
        }
        if matches!(event, Event::Signal) {
            self.poll_file(cx);
            let before = self.snapshot();
            self.poll_agent(cx);
            if before != self.snapshot() {
                self.queue_save(cx);
            }
        }
        self.ui.handle_event(cx, event, &mut Scope::empty());
        let selected = self
            .ui
            .widget(cx, ids!(live_editor))
            .borrow::<live_editor::LiveEditor>()
            .is_some_and(|live| live.document_selected());
        if selected != self.document_selection {
            self.document_selection = selected;
            let title = self.ui.text_input(cx, ids!(title_input));
            use makepad_widgets::makepad_draw::text::selection::{Cursor, Selection};
            title.set_selection(
                cx,
                Selection {
                    anchor: Cursor {
                        index: 0,
                        prefer_next_row: false,
                    },
                    cursor: Cursor {
                        index: if selected { title.text().len() } else { 0 },
                        prefer_next_row: false,
                    },
                },
            );
        }
        if let Event::MouseUp(mouse) = event
            && mouse.button == MouseButton::PRIMARY
        {
            let target = if self.editing {
                ids!(live_panel)
            } else {
                ids!(preview_panel)
            };
            let hits = self.ui.widget(cx, target).point_hits_area(cx, mouse.abs);
            if !self.popup_pointer && !self.composer_pointer && hits {
                self.selection_popup(cx, mouse.abs);
            }
            self.popup_pointer = false;
            self.composer_pointer = false;
        }
        if let Event::KeyDown(key) = event
            && live_editor::select_all_key(key)
            && self.editing
            && self
                .ui
                .widget(cx, ids!(live_editor))
                .borrow::<live_editor::LiveEditor>()
                .is_some_and(|live| live.focused(cx))
        {
            let editor = self.ui.widget(cx, ids!(live_editor));
            let position = editor
                .borrow::<live_editor::LiveEditor>()
                .and_then(|live| live.selection_anchor(cx))
                .unwrap_or(editor.area().rect(cx).pos + dvec2(24.0, 42.0));
            self.selection_popup(cx, position);
        }
    }
}

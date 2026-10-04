# DocGenie 验证记录：本地 MVP 交付

以下历史记录保留原 package 名 `agent-docs-desktop`，当前名称为 `docgenie-desktop`，可执行检查命令见 README。

## DocGenie 发布准备

- 应用窗口、导航品牌、桌面 package / binary 与运行入口统一为 DocGenie / `docgenie-desktop`；旧 vault、Preferences、侧车锁与 `AGENT_DOCS_*` 兼容保留。
- README 更新快速开始、本地无 key 体验、MiniMax 授权与外发范围、数据存储、开发测试和已知限制；补充派生 renderer 上游许可证声明。
- 固定 runtime 校验、新包名 `cargo check`、限定 package 的 fmt、41 项核心 / 存储测试、all-targets Clippy 均通过。
- 新 package 下重跑 `cargo test --locked -p docgenie-desktop --tests --no-fail-fast -- --test-threads=1 --nocapture`：16 项 App 单测 + 40 项原生集成测试通过，2 项真实网络测试跳过。测试实例已退出；查看合成文档截图，窗口与导航品牌均为 DocGenie。
- 待发布文件扫描未发现私钥、常见 API / GitHub token、私人文档链接或开发者绝对路径。`.runtime/`、构建产物、临时 vault、`.env`、缓存与私人截图不入库。该扫描不替代完整安全审计。
- UI 设计文档整理为 `docs/ui-ux-plan.md`，仅保留项目自身设计原则、当前实现和后续路线。

## UI/UX 首轮增量：最终验证

- `cargo fmt -p document-core -p project-store -p agent-docs-desktop -- --check`：通过。
- `cargo test -p document-core -p project-store`：41 项通过，含修改 / 拒绝 / 撤销 / 冲突与 vault 权限检查。
- `cargo test -p agent-docs-desktop --bin agent-docs-desktop -- --test-threads=1`：16 项通过，1 项真实网络 ignore；新加线程过滤 / Unicode 摘要 / 布局断点 / 启动参数单测。
- `cargo clippy -p document-core -p project-store -p agent-docs-desktop --all-targets -- -D warnings` 与 `cargo check -p agent-docs-desktop`：通过。
- 自己重跑 `cargo test -p agent-docs-desktop --tests --no-fail-fast -- --test-threads=1 --nocapture`：40 项原生集成测试通过，真实 MiniMax 用例 ignore，App 单测 16 项通过。不是沿用其他会话的测试结论。最终日志临时位于 `/tmp/agent-docs-verified-ui.log`。
- 新 `workspace` 3 项通过：限宽 / 居中与侧栏组合、模式保持 / 大纲跳转、900×800 窄窗与草稿、线程筛选 / 解决重开 / 同段多线程 / 独立草稿。`vault` 两项均通过，包括此前失败的标题改名。
- 已查看真实原生截图：阅读工作台、线程摘要和详情、900 宽窗、批注卡片。初始 revision label 太浅导致 ink 验证失败，调深后通过；线程列表高度不足裁掉摘要，调整后查看最终截图两张摘要完整。截图只含合成文档。
- 旧测试 helper 适配既有就地 CommentComposer，并将全文替换合成一个 text-input 事件，避免清空后 relayout 抢走焦点；重绑用例使用不含旧锚点文字的改写，避免核心正确平移锚点时误报失败。
- 过程中另一会话并发加入调试探针导致编译输入变化，已与用户确认暂停并发修改。最终源码无 `/tmp/modeprobe.log` 探针，重跑 fmt / Clippy / check / 全量测试通过。
- 全部 GUI 使用 makepad-test 隐藏独立实例 / 临时 vault，结束后进程已退出；没有模型真实网络调用，没有发送用户原文，未修改 `.runtime/`。

本轮仅完成 P1 / P2 首个增量。尚未验收全部规划交互、跨块选择、真实中文 IME、长标题换行、目录刻度浮层、历史恢复与 thread 级 Agent 状态。

## 本轮 Markdown 图片 / 链接验证

- `cargo fmt -p document-core -p project-store -p agent-docs-desktop -- --check`：通过。
- `cargo test -p document-core -p project-store`：40 项通过（包括新增附件路径/符号链接/限额/重命名/undo）。
- `cargo test -p agent-docs-desktop --bin agent-docs-desktop`：12 项通过，1 项真实网络 ignore；新增 PNG 编码/限额与链接协议白名单检查。
- `cargo clippy -p document-core -p project-store -p agent-docs-desktop --all-targets -- -D warnings`：通过；`cargo check -p agent-docs-desktop` 通过。
- GUI：`markdown_media` 1、`drag_selection` 4、`highlights` 2、`live_preview` 2、`typography` 1 全部通过；图片真实绿色像素与链接蓝色像素在两种模式均验证。
- `vault`：树/文件夹/重命名/标签/快速切换/布局恢复用例通过；`editing_the_title_renames_the_file` 失败，等待 `heading=改过的标题` 超时，单独重跑仍失败。本轮不宣称全部 GUI 测试通过。
- 初次新 renderer GUI 启动失败（Octoscript ImageFit / widget namespace），修复后媒体及原编辑回归通过。测试实例均由 makepad-test 关闭，没有退出用户原来的 App。
- 尚未验收：系统剪贴板实际截图粘贴、系统浏览器实际跳转；渲染像素和编码/协议测试不能替代这两项人工检查。无模型网络调用。

## 环境

macOS / Apple silicon / Metal，Rust 1.95.0；Makepad pin `c155f61d0e1600d2ec474209374444a38a09a470`。独立 `.runtime/makepad`，未修改其他 checkout，不启动 Rinx。

## Preferences + 选区自动对焦（已落地）

- `apps/desktop/src/preferences.rs`：5 项单测通过（默认值、round-trip、格式错误回退、未知字段拒绝、原子写入）。
- 5 个 GUI 测试更新：移除 `thread_locate` / `auto_edit` / `agent_state` 引用，改为 Preferences overlay 入口。
- 全部 25 项 GUI 测试 + 1 项 stock render + 5 项 preferences 单测通过；`cargo clippy --all-targets -- -D warnings` 干净。
- 选区自动对焦：删除 `thread_locate` 按钮；`ReadingAction::Open` / `thread_prev` / `thread_next` 三处入口触发 `reveal_active_thread(cx)`，按当前模式（preview/edit/source）分别走 `Reading::reveal` / `LiveEditor::activate_at` / `TextInput::set_selection`。revision 不匹配时 status_label 显示「原段落已变化」，不切模式、不假装已定位。
- Preferences overlay：toolbar `Preferences…` 按钮 + `⌘,` 全局快捷键（`live_editor::preferences_key`）；只读展示 Agent endpoint/model/key_present；可编辑 default_auto_modify / editor_font_size(8–48) / auto_save_enabled；保存原子 rename 到 `$AGENT_DOCS_HOME/preferences.json`（macOS 默认 `~/Library/Application Support/agent-docs/`），无 API key 字段。
- Agent 默认状态从 Preferences `default_auto_modify` 读取，在文档加载时设置 `self.auto`；工具栏 `agent_cancel` 仅在 inflight 请求时显示。

## Feishu 风格交互（已落地）

- `crates/document-core/src/lib.rs`：`Workbench::set_text` 公开方法，绑定 trusted-internal 写入（标题输入框使用）。新增 2 项单测（正向路径 + 超限拒绝）。
- `apps/desktop/src/main.rs`：
  - `title_input` TextInput + author/time 行；`apply_title_change` 解析首行 `# heading` 范围并通过 `set_text` 替换；`sync_document` 增量同步首行到 widget 文本（避免 IME 抢断）。
  - 选区浮窗（`context_menu::CommentMenu`）：双按钮 + Agent 可见性随 `Preferences.default_auto_modify` 切换。
  - `live_editor::comment_shortcut_key` + `App::comment_shortcut_action` 处理 ⌘+⇧+M。
  - `App::reveal_active_thread` 已在之前轮次实现（保持不变）。
  - 批注面板头部：上下导航、✓/○ 切换、`thread_resolve_toggle` 单 handler。
  - `reading::Reading`：Highlight 变体改为 `#fff5d6` 背景 + 顶部 3px 橙色 + 文字加重。
- 5 个测试文件适配：`thread_resolve` → `thread_resolve_toggle`（ui.rs、highlights.rs）。
- 全部 25 项 GUI 测试 + 1 项 stock render + 5 项 preferences 单测 + 2 项 `set_text` 单测通过；`cargo clippy --all-targets -- -D warnings` 干净。

## 最新实现与测试

- 核心/存储：32 项，包含 UTF-8、提案、评论多轮、锚点安全平移/显式重绑、快照、undo、文件保存/冲突/恢复。
- App 单元：密钥字面量解析、Markdown parser block range；真实单轮模型测试默认 ignore。
- 原生端到端：13 项（ui 4、files 2、projects 1、preview 1、agent 1、sidebars 2、highlights 2）。另加 lifecycle 2 项验证停止与迟到 round，总计 15 项。
- highlights 验证粗体且重复内容分别定位、选段右键、逐条消息卡片、点击 badge 打开线程、resolve/reopen、显式重绑，以及截图黄底像素。
- projects 重启后恢复评论，定位原文与 badge，授权仍关闭。
- lifecycle 用延迟 HTTP 测试服务：停止后结果不写入；新回复使旧结果过期，队列处理最新 round，旧响应不追加 Agent 回复。
- 原生所有测试使用临时 AGENT_DOCS_HOME，结束清理；不创建用户真实文档。
- fmt、all-targets Clippy、runtime pin check、Git whitespace 检查为交付检查。

## 真实 MiniMax

使用用户授权的 `~/.zshrc` MINIMAX_API_KEY，仅发送合成文本。

历史单轮检查：`.io` 连接失败，国内 `.com` 可用，模型实际返回 MiniMax-M3。reasoning_split + JSON response_format 支持，JSON 仍必须严格验证。

本轮 `tests/live.rs`：真实原生两轮评论 → 模型修改 → Agent 卡片回复 → 自动保存普通 `.md` → 定位高亮，通过。核验第二轮预期合成句子与结尾保留，磁盘与 editor 内容一致。

一次先前运行返回非约定 JSON，被 App 拒绝且未写入；没有放宽 schema 或执行自由文本来掩盖。后续完整重试成功。另行增加 finish_reason=length 明确拒绝截断输出。服务端不确定性仍存在，需用「让 Agent 处理当前评论」重试，不保证所有自然语言约束都语义正确。

真实检查为 opt-in ignore，普通 test 不调用真实服务。可复现：

```sh
cargo test --release -p agent-docs-desktop --test live -- --ignored --test-threads=1
```

可能产生少量 token 费用，截图只有合成内容，不提交 key/prompt/响应/logs。

## 视觉验收与根因修复

人工检查最新高亮截图：白底三栏、第二个重复格式化段落淡黄背景、右侧两张评论卡片；真实两轮截图：原生文档已修改、黄色锚点、Agent 浅蓝卡片可滚动。

初期 V001 灰屏/文字缺失：字体/栅格化异步、浅色 UI 套用深色控件 focus 色；preload + light theme、等待真实 ink 后验证，已解除基础渲染阻塞。

本轮发现与修复：
- PortalList 不仅遍历逻辑范围，Empty template 必须完成 turtle 绘制。
- 普通 View color 没有 SolidView shader，状态正确仍无背景；改为 SolidView 模板并复查黄底。
- widget tree 不保证动态行按源码排序，测试按几何位置选重复段落，不通过全局 nth 猜 byte range。
- TextInput.set_text 会清除 native history，每次键入不能反复调用；相同文本不重设。
- parser block 尾换行不能包括在替换目标中，避免不带尾换行的模型输出粘连邻段。

## 已知边界（不声称生产发布完成）

- 持久批注高亮在阅读模式，源码编辑仅原生 selection；阅读选区限定单个 top-level Markdown 块，完整列表/表格/代码作为块处理。
- 正文变化阅读回到顶部，定位按钮可跳回；没有 inline source-map 跨块选择。
- 同块多个线程优先高亮第一个，其余上/下条切换；没有线程总览、时间戳、word diff。
- 全文/元数据双文件非事务，crash mismatch 以 `.md` 为准；强制退出 debounce 内容可能未落盘。
- 保存冲突保留内存并阻止切换，未做外部修改合并；目录大规模扫描/长文性能未 benchmark。
- 没有测试真实中文 IME、键盘无障碍、窄窗口或其他 OS；没有签名安装包/凭据 UI。
- `cargo fmt --all` 会进入 upstream vendored workspace，按三个本项目 package 格式化。

结论：用户当前约定的独立本地 Markdown、自动保存、右键批注、多轮 MiniMax、可折叠侧栏的 MVP 主流程已实现；后续增强和生产打包仍是独立工作。

## 本轮 UI/UX 与 Live Preview

- Obsidian 风格改版：`tests/vault.rs` 覆盖文件树展开/折叠、新建文件夹、重命名、新建文档标签、⌘O 切换、右栏页签与重启恢复、删除；已查看截图（左树/顶标签/右大纲）。
- 使用 ui-ux-pro-max，按截图优化工具栏层级、左侧对齐、回复框对比度、条件操作显示，不照搬 skill 的 landing-page 推荐。截图已查看。
- 默认 Live Preview，活动行源码/其他行渲染；原生测试验证行切换、上下方向键、离焦恢复、Enter split 后继续输入、当前行右键批注、自动保存及阅读模式。
- 新增 live_preview 2 项，之前 15 项原生回归通过；核心/存储 32、App 3 项通过，Clippy 与 fmt 通过。最终只修改普通行方向键后重跑 live_preview 2 项通过。
- 蓝色 caret 在活动实时预览行的截图中检查周期性像素变化，验证不只是有焦点；源码模式展示重新修复 focus/blink。
- 曾有 UI 启动测试长等待，检查原生日志发现 align 误用 vec2 类型（应 Align），已修正并重跑；不是把超时当作通过。测试停止未影响用户自己启动的 debug 实例。
- 复杂结构使用整块源码，当前不是完整 Obsidian；Live Preview 不支持跨行 selection/共享 native undo 栈，完整源码模式保留。多行 IME、鼠标拖选、窄窗口和长文性能未人工验收。真实 MiniMax 本轮未再次调用，旧模型流程用模拟 HTTP 回归。

## 两阶段全选 / 浮动批注 / 字号调整

- 编辑与源码模式：Ctrl+A 和 Cmd+A 首次选当前源码行，连续第二次选全文；Live Preview 使用真实全文 TextInput 选区，可替换并自动保存。鼠标/其它按键重置首选状态，repeat 不升级。
- 选中后自动显示「添加批注」overlay；鼠标拖选和快捷键均测试，无需右键。按钮 grab_key_focus=false，click 后显式聚焦评论，绑定范围和 revision；防止同一 MouseUp 再弹出。旧右键/显式重绑测试通过。
- 正文 14 pt，H1–H6 28/22/18/16/14/14。typography.rs 统一层级，原生布局测试确认标题递减且阅读/Live Preview 高度一致，截图已审阅。
- 新增 selection 3 项、typography 1 项，连同所有已有原生流程最终 21 项通过；Clippy/格式/whitespace 检查通过。App 字号单元测试通过，真实网络本轮未调用。
- 浮动工具条暂只做批注入口，未增加截图中的字体/粗体/链接菜单。阅读批注仍以完整 Markdown 块定位，任意跨行拖选的 Live Preview 支持仍不在此次范围。

## 鼠标拖选丢失修复

复现：首次从渲染文字拖动，旧 LiveEditor 消耗 MouseDown 后仅记录 click_position，新 TextInput 没有对应 pointer capture；快速拖动/反向会丢选区。原有最近光标算法也错误：cached_caret_rect 只在 draw 更新，循环 move_cursor_right 不是实时 glyph map，不能用它解析鼠标落点。

修复：保存 anchor/cursor/released，在 active layout 完成后通过原生 TextInput hit-test 解析两端并设真实 selection；Move/Up 同批仍缓冲；一次性 next-frame/短时 timer 等待布局完成，键盘/成功/readonly/切换终止，避免延迟覆盖 Ctrl+A。SelectionReady 才提示 App 显示批注入口。

另修：overlay body 隐藏时旧 snapshot 仍把按钮标可见，测试可点到不存在的工具条；新增显式 visible，absolute Walk 对齐画面/命中位置，模式切换隐藏旧菜单。App 优先处理 overlay pointer，避免工具条 press 激活底层源码行。

新增 drag_selection 4 项，先失败复现、修复后通过：首次正向、快速反向、自动换行中文/emoji/组合字符、已激活源码拖选替换与保存。检查截图看见跨视觉行 selection 和 toolbar；正文/邻段不被批注动作修改。

最新相关 13 项原生测试（drag 4 + selection 3 + live_preview 2 + ui 4）通过，all-targets Clippy 通过。其他 12 项原生回归在本轮此前通过；同时有 Preferences 代码更新，先前一次旧 agent_run UI 检查失败，最终 ui 重跑全部通过。没有修改/回退这些外部更新或使用用户真文档测试。

限制：此修复支持单源码单元内拖选，单段自动换行可选择多条视觉行；跨不同源码行/段落的拖选仍未支持，需要全文源码模式。修复不扩大模型权限或改变正文存储。

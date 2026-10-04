# DocGenie 实施文档：本地 Markdown + 评论 + MiniMax

项目对外名称为 DocGenie，桌面 package / binary 为 `docgenie-desktop`。旧 vault `~/.agent-docs/`、Preferences 路径和 `AGENT_DOCS_*` 环境变量保持兼容。

## UI/UX：第一轮工作台 / 评论增量（已实现）

- App 内实现，未改 `.runtime/`、未引入 WebView 或第三方 UI 框架。保留 Markdown Live Preview、6 标签、⌘O、vault 与 Agent 授权契约。
- 正文 `article_column` 最大 760 Makepad 逻辑点，在可用区域居中；标题 28pt，作者 / 修改时间合并一行，revision 降到页脚。阅读 / 编辑与右侧页签改为文字入口，左栏增加品牌、主新建按钮和复用 ⌘O 的可点击跳转入口。
- `workspace.rs`：窄窗口布局策略。760–1099 宽且评论栏打开时隐藏导航；低于 760 两侧隐藏。自动隐藏不覆盖用户收栏意图；窄窗口点展开导航会先关闭评论。可用 `--window-size=900x800` 指定启动尺寸，参数范围受限。
- `thread_list.rs`：PortalList 线程摘要，含原文摘要、最后消息、当前线程标识与过期锚点提示；未解决 / 已解决过滤含数量和空状态。摘要点击通过已有 Workbench thread ID 打开详情并定位，不改变正文或授权。
- 回复草稿按当前文档的 thread ID 在内存隔离，切换线程 / 收栏保留，切换文档清空；不写 `.state/ui.json`。
- 评论弹窗沿用此前已有 `CommentComposer`；新增空白提交禁用，移除未实现的 `@AI` 提示，准确说明单条「问 AI」与全局默认自动修改两条授权路径。选区 B / I / U / code / link 按钮也属于本轮开始前已有能力，不计作此次新增。
- 大纲跳转不再强制切换编辑模式：阅读模式调用 `Reading::reveal`，编辑模式调用 `LiveEditor::activate_at`。
- 新增 `tests/workspace.rs`：限宽 / 居中 / 左右栏组合、搜索入口、阅读大纲跳转、900 宽度收栏与草稿、线程概览 / 筛选 / 解决重开 / 同段多线程 / 独立草稿。

仍未完成：条目级更多菜单、目录刻度浮层、长标题自动换行、全部线程完整内联回复卡片、thread 级 Agent 状态、历史快照、跨块选区与精确 inline 批注。本轮是 P1 / P2 的首个增量，不代表整体设计路线完成。验证结果见 `docs/validation.md`。

## Feishu 风格交互（已落地）

- **大标题输入框**：编辑器顶部新增 `title_input`，绑到文档第一行 `# heading`；空文档显示「请输入标题」灰色占位（28pt）。编辑通过 `Workbench::set_text` 走标准 commit/revision/undo 通路，不绕过 proposal 流程。`document-core` 新增 2 项单测（覆盖正向 + 超限拒绝）。
- **author + 修改时间**：标题下方一行，「你」+「今天修改」/「刚刚修改」。「刚刚」在 save 成功后写入。
- **选区浮窗**：选中文字后浮窗包含两个按钮——`💬 评论`（点击进入批注 composer）和 `✨ Agent 完善`（Preferences `default_auto_modify` 开启时才显示）。所有 agent 相关配置已统一移到 Preferences overlay（`⌘,`），侧栏不再承载任何 agent 控件。
- **⌘+⇧+M 评论快捷键**：Feishu 标准快捷键。等价于浮窗上的「评论」按钮，无选区时状态栏提示「请先在文档中选中要评论的文字」。
- **批注高亮重排**：feishu 风格的淡黄底 `#fff5d6` + 顶部 `#ff9800` 橙色 3px 边线 + 文案加重 `#8a4b00`；badge 文字改为 `💬 查看批注`。
- **批注面板头部重排**：移除大黄条；评论数 + `↑↓` 上下条 + `✓/○` 解决切换（toggle 而非两个按钮）+ 1px 分隔线。quote 框保留顶部 3px 橙色边线。

## 当前产品契约（用户最新决定）

1. 独立 Makepad + Octoscript App，无 Rinx、小程序、多人协作。
2. Agent 优先使用 `minimax-m3`；用户自行配置 MiniMax 凭据，读取方式见 README。
3. 文档由用户新建，在 `~/.agent-docs` 统一管理，普通 `.md` 正文实时自动保存；不存在外部 Markdown 导入或手动项目文件管理。
4. 选中文本 → 右键添加段落批注 → 右侧评论线程；用户回复驱动 Agent 多轮完善。

## 已实现

### 本地文档库

- `project-store::library` 管理根目录，默认 `~/.agent-docs`；`AGENT_DOCS_HOME` 可覆盖。
- 首次启动自动新建，之后启动打开最近修改文件；左侧文件树切换（folder 展开/折叠，新建文档/文件夹、重命名、删除），顶部文档标签（最多 6 个），⌘O 快速切换。
- 新文档为可读文件名的标准 UTF-8 Markdown；旧根目录 `<UUID>.md` 不迁移，显示名取首个一级标题。右栏「大纲/评论」页签，与展开状态一起持久化到 `.state/ui.json`。
- 隐藏 `.state/<UUID>.json` 持久化评论、revision、undo，与正文不一致则不恢复旧锚点。
- debounce 350 ms 后台保存；保存中有新编辑再次保存；切换前 flush，普通窗口关闭等待保存。
- 失败不丢弃内存，阻止切换与关闭，显示状态并支持重试；原子 temp+sync+rename，拒绝外部不同内容覆盖。
- 已移除路径输入、导入、手动保存、SQLite `.agentdocs` 和素材模块及依赖。旧文件不会自动迁移或删除。

### 评论与 Agent

- 编辑/阅读模式均支持选区右键原生 overlay 菜单「添加段落批注」，保留选区并聚焦右侧评论框。
- 阅读区按 parser 的 top-level byte ranges 分块，选中块内文字右键即绑定该 Markdown 块；粗体/链接/重复段落不再通过文本搜索猜定位。编辑模式将选区扩展至完整 Markdown 块；块末尾换行留在文档中，避免模型替换时粘连邻段。列表/表格/代码围栏作为完整结构块处理，阅读模式不支持跨块选区。
- 本地多轮评论、逐条原生消息卡片（User/Agent 视觉区分）、解决/重新打开、前后切换、保存恢复。
- 阅读模式未解决且有效的批注块持续淡黄高亮，点击段落或「查看批注」打开线程；上一条/下一条切换或点开线程时，自动按当前编辑器模式 reveal：阅读模式滚动到高亮块，编辑模式跳到该块，原文变化则显示「原段落已变化」并保持当前选区等待重新绑定；不再需要「定位原文」按钮。
- 自动修改仅在 Preferences（⌘,）中作为「默认启用 Agent 自动修改」开关存在；评论侧栏不再承载这一配置项；开启后发送评论/回复自动触发，段落与线程是唯一外发上下文。Agent 状态只在工具栏「停止 Agent」按钮和底部 status_label 反映。
- default MiniMax endpoint 为 `https://api.minimaxi.com/v1/chat/completions`，model `minimax-m3`。实测服务返回 `MiniMax-M3`。
- 密钥优先环境，fallback 只解析 `.zshrc` 字面量赋值，不执行 shell；key 不入日志、文件或 JSON 元数据。
- provider reasoning_split 与 JSON response_format，严格 replacement/explanation，支持 fenced JSON 与完整 think 前缀移除；非法结果不写入。
- 60 秒 timeout、2 MiB response、禁用 redirects；按 HTTP status/timeout/连接类反馈，不输出原始响应或凭据。
- request epoch + document UUID + revision + round + original 校验。新回复使旧 round 作废；队列去重串行处理最新评论，取消/切换清空队列。
- 非重叠编辑仅在原文校验成立时安全平移批注锚点；碰到批注内容/undo 则标记过期，必须人重新选段绑定。即使锚点平移，编辑前的 inflight request 也不能写入。
- 普通键入不重复 set_text 重置原生 IME/撤销历史；恢复或 Agent 修改才同步不同正文。

### Markdown 图片与链接

- 正文编辑焦点下 ⌘V / Ctrl+V 支持系统剪贴板位图（例如截图）；保留原生文字粘贴优先级。Rust 将位图编码为 PNG，存到 vault `assets/<UUID>.png`，插入 `![图片](assets/<UUID>.png)`，走现有 Workbench/revision/自动保存/撤销通路。
- `apps/desktop/src/markdown.rs` 为 App-owned 原生 renderer，编辑实时预览与阅读均显示本地图片；`.runtime/` 未修改。图片引用为 vault 根相对地址，重命名文档/文件夹不失效。附件目录不显示在文件树，撤销/删除正文不自动删除附件（避免破坏其它引用）。
- `[fdsafd](http://www.bing.com)` 显示蓝色 `fdsafd`，单击通过系统默认浏览器打开；编辑模式点链接不再激活源码行，点击该行其它位置仍可编辑。只允许无凭据的 HTTP/HTTPS URL，拒绝 file/javascript/data/自定义协议。
- 图片限 16 MiB / 1600 万像素，拒绝外部路径、隐藏项和 symlink；不加载网络图片，不把图片内容发给 Agent。第一版不支持复制文件路径粘贴、HTML 富格式粘贴或远程图片。
- `markdown_media` GUI 测试已通过，验证编辑/阅读的蓝色链接与真实图片像素。系统剪贴板实际粘贴和外部浏览器打开仍需人工验收；编码、协议安全、附件路径/限额/重命名/撤销已覆盖回归。

### 原生界面

本轮使用 ui-ux-pro-max 指南并参照用户截图：减少顶部工具密度、左对齐文档/大纲、文档标题占弹性空间、右侧淡色卡片与白色回复框、只在错误时显示保存重试/任务时显示停止、空线程隐藏定位与前后按钮。没有引入网页框架或照搬 skill 的 landing-page 布局。

默认「编辑 · 实时预览」：点击普通行显示源码 TextInput，其他行 Markdown；点击别行/焦点转到评论框恢复旧行渲染。支持上下切行、Enter 新行、行首 Backspace 合并；选中当前源码可右键评论。列表/表格/引用/围栏代码作为结构单元编辑，避免拆行破坏语法。「阅读」只读并保留持久批注高亮。

光标颜色显式设为蓝色，0.5 秒闪烁；source-mode 每次展示重新 take_key_focus；实时预览激活新行后布局完成才聚焦，用原生 TextInput hit-test 解析点击和拖选两端，离焦停止。

Ctrl+A / Cmd+A 两阶段：第一次选当前源码行（结构块内也只选光标所在行），连续再次按选择全文；自动重复按键不算第二次。Live Preview 将全文作为同一原生输入单元展示真实 selection，copy/cut/paste/delete 生效，不跳转模式或借用隐藏代理。普通输入后恢复逐行渲染。

选区 MouseUp 自动显示轻量「添加批注」overlay，Ctrl/Cmd+A 也显示；不抢焦点，点击后才展开右栏并聚焦评论。首次从渲染段落拖选会保留整个 down/move/up 手势，支持正向、反向、快速事件及单段自动换行。已激活行上的按下先交给原生输入框（放置光标、拿焦点、持有真实指针捕获），拖动移动仍走合成事件并按手势缓存命中映射；释放时复用缓存，不再补发合成按下——平台只在自己 mouse-up 时释放捕获，合成按下泄漏的捕获会经快速路径抢走下一次真实点击（命中最坏情况是吞掉侧栏点击），残留在下一次按下时被隔离、随后由平台释放清除。原生选区布局未完成时延迟显示工具条；隐藏按钮不会留下旧 hit 区域，工具条点击优先于下层文档。popup 绑定 range+revision，过期选区不能创建评论；Escape/滚动/外部点击关闭，右键入口保留。没有增加截图中的其它格式按钮。

正文与源码字号统一 14 pt；H1–H6 为 28/22/18/16/14/14 pt。typography.rs 统一补偿 native Markdown heading scale，实时预览/阅读一致，避免低级标题比正文更小；评论字号保留 14。

当前不是完整 Obsidian：除两阶段全文 selection 外，实时预览任意跨行 selection、跨行 native undo 共享栈、复杂结构逐行折叠、剪贴板富格式、窄窗口与 IME 人工验收仍未完成；源码模式已移除。

白底三栏沿用用户参考图：我的文档/大纲，Markdown 文档，本地评论。顶部独立开关可收起/展开左右侧栏，中间编辑/阅读区域自动填满释放的空间。隐藏保留评论草稿与线程，右键添加批注自动展开评论栏；当前会话保持布局，重启默认全部展开。原生 Markdown 的 runsplash 块只显示文本。右键菜单可 Escape/外部点击/滚动关闭。

Preferences 覆盖层（⌘, 或工具栏按钮）只承载跨会话/全局配置：默认自动修改、编辑器字号（8–48）、自动保存开关，以及只读的 Agent endpoint / model / key 来源展示；不暴露 API key 字段（继续走环境变量）。持久化到 `$AGENT_DOCS_HOME/preferences.json`（默认 macOS `~/Library/Application Support/agent-docs/`），原子 temp + rename 写入；缺失/格式错误/未知字段一律回退到默认并打印到 stderr，不阻塞启动。开启自动保存会跳过内部 queue_save。

## 运行

```sh
python3 tools/setup-runtime.py
cargo run -p docgenie-desktop
```

本地编辑不需要 key，Agent 请求需用户自行配置凭据。开发覆写：`AGENT_DOCS_HOME`、`AGENT_DOCS_MODEL`、`AGENT_DOCS_ENDPOINT`、`AGENT_DOCS_API_KEY`；不要在共享脚本内写 key。

## 接下来的实施顺序

当前用户要求的本地 MVP 闭环已实现，最终检查见 validation.md。后续增强不作为本轮完成条件：
1. 时间戳、局部 word diff、多线程概览、同段多线程快捷切换。
2. 保存冲突合并、搜索/命名/删除与备份。
3. 真实中文 IME 人工验收、长文性能、窄窗口适配。
4. 安全凭据 UI 与签名 `.app` 分发。

当前高亮在阅读模式，源码编辑仍使用原生选择高亮。阅读选区限定单块（完整列表/表格/代码也是一个块），不是跨块 source map；两次合成文本真实模型检查不能代表所有写作任务的语义质量。

不重新增加导入、多人协作、素材库、SQLite 项目抽象、视频剪辑或自动发布。

## 检查与交付标准

```sh
cargo fmt -p document-core -p project-store -p docgenie-desktop -- --check
cargo test -p document-core -p project-store
cargo test -p docgenie-desktop --bin docgenie-desktop
cargo clippy -p document-core -p project-store -p docgenie-desktop --all-targets -- -D warnings
cargo test -p docgenie-desktop --tests -- --test-threads=1
# 单独的真实网络检查，使用合成文本，可能产生少量 token 费用
cargo test -p docgenie-desktop --bin docgenie-desktop live_minimax_paragraph_edit -- --ignored
```

测试目录必须隔离，不写用户的 `~/.agent-docs`。真实模型、模拟 HTTP、业务状态、截图验收分开记录。强制退出不能承诺所有最近输入已保存；Markdown/元数据两个文件不是单一事务，元数据不匹配时以正文为准。存储限额与安全约束见 architecture.md。


## UI 调整：图标与模式

- 工具栏与侧栏头部改用 SVG 图标（`apps/desktop/resources/icons/`）：« / » 收起展开导航与评论，铅笔方框 = 编辑，书本 = 阅读，另有撤销、新建、重试、偏好、停止。
- 仅保留「编辑（实时预览）」与「阅读」；删除源码模式、`slash_menu` 与对应 `editor` 输入框。
- 首个单行 `# 标题` 由 `reading::title_block` 识别：阅读与实时编辑均不渲染它，仅 `title_input` 显示并通过 `apply_title_change` 写回。
- 图形测试 `ui::title_renders_once_and_mode_icons_toggle` 覆盖标题单次渲染与图标切换。

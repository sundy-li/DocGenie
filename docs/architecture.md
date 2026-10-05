# DocGenie 架构：Managed Markdown / 本地评论 / MiniMax

应用名称与桌面 package 已改为 DocGenie / `docgenie-desktop`。存储目录、侧车锁后缀和 `AGENT_DOCS_*` 配置保持旧格式兼容，不自动迁移已有文档。

## 工作台菜单与标签生命周期

`WindowMenu` 在 macOS 构建原生应用菜单，settings LiveId 与 ⌘, 共用 App::show_preferences；macOS 不在工作台展示第二入口。退出使用系统 QuitRequested durability barrier（不直接依赖 menu quit 命令跳过存储）。平台默认 app 菜单只覆写本 App 提供的 Settings / Quit，不展示未实现的业务动作。

三栏顶层直接包围 navigation_panel / center_panel / comments_panel；中心 toolbar / ScrollXView 标签仅占中间区域，两侧固定开关与原宽度策略一致。TabMenu 不持有写入能力，点击转为纯 Close plan + rel 校验。pending_tabs 在保存完成前不修改 tabs，active 被移除先 flush 最后 snapshot，失败恢复可编辑且保留标签；保存引起 rel 重命名必须同时 remap pending plan。空工作台释放 active document / drafts，正文不放进 UiState，可创建或重开 vault 文档。关闭视图不等于删除文件。

## 工作台视图层（UI/UX 首轮增量）

- `apps/desktop/src/workspace.rs` 只定义正文限宽 / 窄窗口侧栏策略与受限启动尺寸；不进入 document-core。
- `edit_projection.rs` 是无 UI / IO 的呈现映射与保守编辑层；完整 Markdown 仍是 Workbench 真值。`styled_input.rs` 只改原生布局以绘制 styled runs，不隐藏源码代理、不修改 runtime。部分格式跨界编辑与解码实体歧义必须拒绝而不是丢格式；错误经 UI 提示。
- `apps/desktop/src/thread_list.rs` 从 Workbench 派生可过滤的线程摘要，持有稳定 thread ID；点击后由 App 打开既有详情 / 定位。列表不创建请求、不写正文，不增大 Agent 处理范围。
- App 持有文档内逐线程回复草稿；切换文档清空，不持久化到布局文件。解决 / 重开仍走核心方法，变更仍使用原自动保存。
- 继续保留右侧大纲 / 评论页签，阅读大纲跳转不切编辑模式。目录浮层、完整多线程回复卡片与历史版本不是本轮架构承诺。
- 既有选区格式浮栏和就地 CommentComposer 在本轮保留；每条 ask_ai 与 Preferences.default_auto_modify 都可允许模型处理，说明文案不得把其中一路说成唯一授权来源。

## 产品边界

独立 Makepad + Octoscript + Rust 原生 App，无 Rinx、小程序、多人协作。当前主线是本地创建、自动保存、右键段落评论、Agent 多轮完善。没有外部文档导入、手动绝对路径或项目数据库管理。

```text
Makepad / Octoscript UI
  文件树 | 文档标签 + Markdown 编辑/阅读 | 大纲 / 评论 切换
     │ 新建/切换/输入     │ 右键 CommentMenu     │ 回复/授权
     └───────────────────┼─────────────────────┘
                         ▼
Rust App controller
  debounce timer / flush / single file worker / document UUID / epoch
          │                   │                    │
          ▼                   ▼                    ▼
  document-core        project-store::library      agent.rs
  Workbench/revision   ~/.agent-docs/<UUID>.md      MiniMax-M3
  comments/round/undo  .state/<UUID>.json           selection + thread
```

UI 使用固定 Makepad `script_mod!` Octoscript dialect，不是独立 canonical VM。Rust 是文档真值；App-owned StyledInput（派生 pinned TextInput）管正文 IME / 选区 / history，Markdown+TextFlow 管块绘制与表格网格。所有源自文档的脚本（runsplash）显示为文本；模型没有任意文件读写、shell、工具、批准接口。

## 文档库与自动保存

`Library` 只接受绝对 root，默认 HOME/.agent-docs（AGENT_DOCS_HOME 覆盖）。root/.state 权限 0700，文件 0600，目标 symlink/non-regular 拒绝。vault 为 root 下的目录树：只列 `.md` 与文件夹，跳过隐藏项、symlink，深度≤12、节点≤5000。路径为 vault 相对 `rel`，每个父级必须是真实目录；名称拒绝 `/ \\ :`、前导点与过长。根目录 UUID.md 保留 UUID 为 id 并以首个 # 标题显示，其余路径用确定性哈希 id。重命名/删除同步移动或删除 `.state/<id>.json`，删除文件夹仅在只含 .md 与文件夹时允许。`.state/ui.json` 仅存界面布局，不含正文。不从外部路径导入。

Markdown 是 canonical 正文。元数据 JSON version 1 包含文档 UUID、Workbench snapshot（正文副本用于一致性检查、评论与 undo）；不含 key 或 auto-edit 授权。加载时先受限读取正文，再校验 JSON/DTO/UUID/text；元数据缺失或损坏或正文不同，以正文重新建立 Workbench 并提示没有恢复旧锚点。当前无自动迁移旧 `.agentdocs`。

每次正文、评论、解决、undo、Agent 应用变更重置 3 秒 trailing-edge debounce：最后一次修改后静默 3 秒才保存。`autosave.rs` 持有单调时钟 deadline，只有实际修改重置；非修改 action、UI signal 和 file worker 完成不延后 deadline。file worker 单次仅一个请求，不阻塞 UI；保存期间新编辑保持 dirty 并拥有新 deadline，旧任务完成不能提前追存；若 deadline 到期时 worker 仍忙，完成后保存最新快照。新建 / 切换 / 关闭 / 显式重试直接 flush，不等待 debounce。Preferences 关闭自动保存时取消定时保存，但切换 / 关闭仍保留 flush 防丢失。列表与显示标题随保存更新；列表重建暂为同步小目录扫描，大目录性能待改善。

新建/切换先 flush 当前快照；期间编辑器只读并拦截修改命令；加载完成重建 Workbench、取消旧 epoch 授权。普通 WindowCloseRequested 阻止关闭并 flush，成功后 close；失败取消关闭和只读，保留内存，不自动丢弃。强制进程终止、系统崩溃不保证最后 debounce 窗口中的内容。

正文先 save(temp+sync+atomic rename)，元数据后独立 temp+sync+rename；两文件不是事务。崩溃后正文仍可独立打开，metadata mismatch 不复活锚点。若正文已成功而 metadata 失败，可在 disk == intended content 时幂等重试。外部不同内容变化则拒绝覆盖并显示失败；无需自动合并。保存成功时所保存快照成为 baseline，后续新输入继续 dirty。

低层文件 save 用侧车 create_new lock、双基线检查防合作 writer；非合作 writer 仍可在最后 check/rename 之间修改。仅本地单用户编辑器，不承诺并发多实例评论状态合并。lock 崩溃遗留需人工处理；父目录链接/敌对文件系统不是 OS 沙箱保证。

## Markdown 媒体与外部链接

`clipboard.rs` 使用原生系统剪贴板，仅正文编辑焦点下的显式 paste 读取位图；文字优先，图片转为 PNG 后由 `Library::store_image` 创建不覆盖的 0600 文件。正文使用 vault 根相对 `assets/<UUID>.png`，附件目录 0700 且为保留名称，不出现在 vault 树或文档删除命令中。`read_image` 仅接受此格式并检查真实目录/普通文件、PNG header、16 MiB 与 1600 万像素上限；不接受文档提供的绝对路径、越界路径或网络 URL。附件不随 undo/删除清理，避免共享引用失效，垃圾回收暂未实现。

App-owned `markdown.rs` 沿用固定 Makepad parser/TextFlow 的原生布局，补全 link label 与 Image 渲染，不修改依赖 checkout。编辑与阅读使用 `DocMarkdown`；LiveEditor 普通命中 link 时编辑显示 label，⌘/Ctrl+命中 link 时将 pointer 事件交给 link widget；阅读模式仍普通点击打开。只有用户点击才 `Cx::open_url`，只允许无 URL credentials 的 HTTP/HTTPS，禁止脚本和本地文件协议。图片插入通过原生 TextInput paste action → LiveAction::Changed → Workbench，不绕过 revision、保存及拒绝检查；剪贴板/解码不属于核心 domain，模型不接触图片字节。

## 评论呈现与撤销路由

评论锚点和 Agent write range 为用户实际选择的 source bytes，不扩展 paragraph block。LiveEditor 使用 Projection；Reading/DocMarkdown 收集实际 draw text 的 parser offset 与 TextFlow index，保留重复文字的确切位置；decoded/gap mapping 不确定必须拒绝。引用摘要只缩略视觉，不改变锚点。未解决锚点按 text band 绘制而非块底色；复杂 child 绘制仍需专项。CommentQuote 保留完整语义 text，实际 button 46 字符灰色摘要，定位须 revision 有效。

Message optional author / created_at 由 host 提供并一次 stamp；core 校验字符串、时间与 state_bytes，不读取 clock / environment。persist snapshot 包含真实元数据，旧消息没有字段保持 unknown，CommentRequest 包含元信息快照也被完整 fresh 校验。单机作者显示“你”，无身份冒认。UI local time 转换在 comment_meta.rs；Agent 元信息不扩权。

正文 UndoRequested 统一 Workbench undo；原生 input history 不再接管正文 ⌘Z，离开活动行仍能撤文档操作，成功后恢复焦点 / cursor 并排队保存。标题、评论 / composer、Settings 原生撤销独立；native redo 尚未统一，不能宣称完整跨块历史管理。

## 评论模型与自动应用

CommentThread：byte range、original、base revision、round、User/Agent messages、resolved；线程 ID 为 Workbench 内 append-only index，document UUID 隔离跨文档结果。

- comment 捕获原文；reply 推进 round；comment_request 创建只读请求。
- apply_comment_result 校验完整 request，再于 candidate clone 中提交精确选区修改与 Agent 回复，最终成功才替换 Workbench。
- successful result 将当前线程锚点更新为新文本、新 revision；用户继续回复支持下一轮。
- commit 用字符边界安全的共同前/后缀找最小改变区间，仅平移完全位于区间外且原文校验成立的线程锚点，并推进线程 revision；重叠线程保持旧 revision，标记过期。旧请求捕获的 revision 不变，因此任何文档改变都会使 inflight response 过期；新的请求可用平移后的锚点继续。
- rebind_thread 是显式用户命令，捕获新选段、推进 round；resolved/reopen 同样推进 round，防止解决后再打开让旧请求复活。
- undo 恢复正文但不回退 revision，不删除线程回复，旧锚点过期。
- resolved 不允许 Agent 写入；恢复仅恢复本地数据，不自动批准或外发。

限额：正文 1 MiB、undo 最多 100、批注/旧提案各 256、线程 128、每线程消息 128、单消息 16 KiB、总 domain 文本负载 32 MiB、metadata 64 MiB。旧提案模型当前保留 domain 回归，不出现在 App。

## 侧栏布局

## Preferences 与全局设置

- `apps/desktop/src/preferences.rs` 提供持久化的 `Preferences`：默认自动修改、编辑器字号、自动保存、代码字体优先级列表（`editor_code_fonts`）。serde + `deny_unknown_fields`，加载缺失/格式错误/未知字段一律回退默认并打 stderr；保存原子 temp + rename。设置面板即时生效：控件变更立即保存并调用 `apply_preferences`（字号经 `typography::apply`/`StyledInput::set_body_font_size` 重排，代码字体经 `fonts::apply_code_font` 在 loader 层重定义 `theme.font_code` 家族并强制重排）。
- 路径解析：`$AGENT_DOCS_HOME/preferences.json` 优先（测试隔离），否则 macOS `~/Library/Application Support/agent-docs/preferences.json`。
- API key 不经 Preferences；`agent::Config::probe()` 仅展示 endpoint/model/key_present，给 Preferences overlay 做只读展示。
- Preferences 不属于 Workbench / project-store 范畴，独立于项目状态；切换文档不写回。

`navigation_panel` / `comments_panel` 是可见性可控的原生 View，顶部开关不在侧栏内部，收起后始终能再次展开。两个独立会话态 bool，不进入 Workbench snapshot、不触发保存或更改 Agent 授权。隐藏仅改变布局，不重建子控件，保留评论草稿与线程。中间 Fill 区域自动扩宽，编辑/阅读都适用。

右键菜单创建批注时主动展开评论栏并聚焦输入框；不连带展开导航栏。侧栏切换隐藏已打开的 overlay 菜单，避免沿用失效的屏幕位置，但不清空文档选区。

## 选区右键

App 在 Event::MouseDown SECONDARY 且命中文档区域时，在子控件处理之前捕获选区并消费右键，不让 caret 被右击移动。菜单用独立 overlay draw list+root turtle 绘制，按鼠标坐标 clamp 到窗口；添加动作聚焦右侧 comment_input。

编辑选区是 UTF-8 byte offset；阅读模块 `reading.rs` 使用与原生 Markdown 相同版本 parser 的 into_offset_iter 提取 top-level 源码块，每块独立 Markdown/TextFlow。右键命中的块有 selection 时捕获该块 source range，不需要渲染文本搜索，重复/格式化段落都可确定。替换范围剔除块末尾换行，保留块分隔符。阅读跨块选择不支持，列表/表格/围栏代码保持整块语法。

有效未解决线程与块范围重叠则使用 Highlight SolidView template 持续淡黄背景，点击打开右栏线程；右侧定位调用 PortalList set_first_id_and_scroll。重叠多个线程优先显示第一个，其余通过上/下条访问。恢复根据 validated thread revision/range 重建高亮，不持久化屏幕坐标。

`discussion.rs` 使用 PortalList 逐条 User/Agent 卡片，白色/浅蓝 template 与作者 label，内容为只读 selectable Markdown；runsplash 同样降级文字。template 不能仅使用普通 View 赋 color（未定义 solid shader），必须 SolidView。消息全文作为 Widget.text 提供状态核验，不隐藏第二份 UI。

source block 虚拟化避免全长文绘制；正文改变时重建块，当前回到阅读顶部，定位按钮可跳回线程。不承诺跨块选择或 inline 精确高亮，编辑模式仍仅原生选择高亮。

大纲为 ATX heading PortalList，忽略 ``` fence 内 heading；填充条目用 Empty template 完成 turtle 绘制。窗口 light theme、wrapper View 切换编辑/阅读，像素回归检查真实 ink。

## MiniMax 与隐私

默认完整端点 `https://api.minimaxi.com/v1/chat/completions`，model `minimax-m3`。真实服务返回 MiniMax-M3；不会静默 fallback 其他型号。端点 override 必须 HTTPS 或 localhost/127.0.0.1 HTTP，拒绝 URL credentials/fragment，禁用 redirects。

key 优先 AGENT_DOCS_API_KEY → MINIMAX_API_KEY → HOME/.zshrc literal MINIMAX_API_KEY。仅扫描 assignment；拒绝变量展开/command substitution/backticks 等，不 source shell。不打印/persist key/raw request/response/error；环境和 zshrc 均为开发凭据方式，OS credential store 待补。

开启 auto-edit 后评论/回复发段落和当前线程；其余正文/其他评论/文件不外发。request reasoning_split=true + JSON response_format；最多 2 MiB 响应，60 秒 timeout，严格 replacement/explanation 解析；兼容完整 think 前缀/fenced JSON，非法 JSON 不修补不应用。

epoch + UUID 过滤取消/切换后的迟到结果，domain revision/round 再校验。VecDeque 去重保存等待线程，单请求串行，处理时读最新 round；旧 round 返回不写入，若仍有效则处理最新回复。不遍历恢复的所有线程；授权开启时仅当前线程及随后显式发送的评论进队列。停止/切换清队列，只本地忽略结果不撤回已发请求。临时 epoch / 单条授权不保存；Preferences.default_auto_modify 会持久化并在加载时恢复，因此默认开关仍可授权后续显式发送。请求故障不伪造 Agent 消息。

## 呈现式编辑模式

活动行布局契约由 Rust / `styled_layout` 管理，而不是按 Markdown 类型派生 Octoscript wrapper。正文输入不继承表单的上下 margin，内 inset 对齐 DocMarkdown；样式行盒使用 normal font metrics（对应 TextFlow `align_row_height`），自动换行使用同样的 ascender wrap spacing。列表 marker 为不可编辑装饰，item 的 inset / pitch / 内容缩进进入 LaidoutText 几何，使 caret / selection / hit-test 和文字使用同一行盒。紧凑单层列表、普通正文 / 标题与混合样式的激活前后有坐标和截图像素回归；复杂嵌套 / loose lists 不属于当前已验收范围。

`live_editor.rs` 是原生 Widget，维护受 domain 真值约束的 UI 缓冲、UTF-8 source units、活动单元、光标/选区。普通正文每源码行一个 unit，结构 Markdown 保持完整块。PortalList 活动单元使用 App-owned Active StyledInput，其余 Markdown；`edit_projection` 以 parser offset runs 映射显示 / 源码范围，活动输入 buffer 不含隐藏 inline 标记。变化通过保守的 lossless source edit 拼回完整正文，经 LiveAction::Changed 交给 Workbench 及自动保存，不把无标记文字直接覆盖源块。表格保留 TextFlow 网格，以 cell range + projection 实现独立单元格输入，锁定时同步只读。

Enter split 后计算 global caret → 新 unit/local caret，下一次 draw 完成后 take_key_focus；旧控件 focus-lost 必须校验 UID，不能关闭新行。Backspace 合并仅 collapsed selection 行首触发，不能截获 Cmd+A 清空；上/下可切普通行，Shift selection 不跨单元接管。首次静态渲染→活动样式输入切换用 DragSelection 保存 MouseDown 及 anchor/cursor/released，Move/Up 即使同批发生也不会丢失。布局完成后通过 TextInput 原生 hit-test 解析两端，不遍历 cached_caret_rect（该 rect 只有 draw 才更新，逐字移动时会一直返回旧位置）。本地调用输入控件 pointer handler，不递归派发 App/OS 事件。next-frame 等待首次有效布局；拖选 interval 在未释放时保持，仅当 pointer 到达 list 可视区顶部 / 底部 20 逻辑点驱动 offset，正文中间不驱动滚动（含滚轮 / trackpad 惯性）。释放 / 键盘 / 切模式 / readonly / 新正文停止 timer；quiet selection 不自动 scroll-to-cursor，避免中间选字使正文跳动。指针移动时更新 source mapping，防延迟手势覆盖后来的 Ctrl+A。

SelectionReady 通知 App 在释放手势的最终选区准备好后才展示批注入口。当前范围仍在单呈现单元内，经 projection 映射 source range（可包含自动换行），跨多个源码行/段落不是此修复的新能力。

编辑投影保留 parser 默认省略的段落前后空格与纯空白 buffer，并映射到真实源码范围；活动重绘不能把刚输入的空格当成 Markdown 空白裁掉，否则会重写输入、回退 caret / 清掉 history。编辑标题还保留没有空格的 marker-only ATX（`#` / `##`）和前缀全部空格，避免 CommonMark 空标题解析后重绘吞键；逐键输入与反向删除均有投影和 GUI 回归。set_text 仅文本不同才调用，防清空 composition/history；主动 focus repair 开始 blink。失焦或切阅读 deactivates 单元恢复渲染，正文不变化。只读切换/加载锁同样约束 LiveEditor，切文档/Agent 新正文重建 unit；外部更新不会偷偷保留 stale selection。

两种模式：编辑（呈现式编辑，默认）与阅读（只读，Reading 高亮），以模式按钮切换；首个 `# 标题` 单独在 title_input 渲染，不进入正文单元；顶部标题不显示前缀，输入 buffer 始终只有标题内容；仅序列化时写 H1 标记，避免程序性 `set_text` 插入前缀破坏 caret、IME 和 undo。全空正文画布内任意点击可激活末尾真实空行，不能只依赖 Blank 模板的 24px 命中区域；布局后一次 next-frame 交接空画布焦点；旧失焦通知必须确认当前输入确实已失焦且不在 canvas hand-off 中，防同批通知关掉新行。新 MouseDown 取消交接，避免抢走后续外部点击。蓝色 caret 0.5s blink 由原生 animator/timer 管理，像素测试校验实际闪烁。Live Preview 支持两阶段全文 selection，但不承诺任意跨行拖选或共享 native undo 历史。复杂语法保持完整块，活动单元使用显示文字投影；不承诺完整 Obsidian 内核。

StyledInput 的选区为浅青色直角 band，按每个 selected row 的实际文字宽度绘制，连接相邻行的 interline spacing。band 在字形前绘制，避免 opaque 背景遮字。全文选择不再聚合输入或重排：Rust shared `all_document` range 保留 PortalList 原块树，DocMarkdown 复用实际绘制 row area（而非另算 TextFlow selection layout），以 App-owned translucent DrawColor 画 band，避免特殊 selection draw-call group 在列表 turtle 下不绘制。表格每格保留 StyledInput selection；顶部标题原位高亮。图片布局保留，图片本身不增加文本 band。

Ctrl/Cmd+A 由 editor 拦截：第一次选择 cursor 所在 source_line（包括结构块内），第二次需 line_selected latch 和选区仍匹配；鼠标/其它按键重置 latch，自动重复忽略。第二次保留当前活动 StyledInput 和 rendered blocks，返回完整 source range；TextCopy / TextCut 处理完整 Markdown，TextInput / Delete / Backspace 通过 LiveAction::Changed 提交全篇 replacement，不隐藏代理、不合并输入。焦点 / selection pending 过渡也返回完整 range，不能用尚未绘制旧控件覆盖 popup。失焦或修改后恢复普通 units。

选区结束后 MouseUp / Ctrl-A 触发 overlay，位置取选区末端或鼠标坐标，不清选区不抢焦点。菜单用 absolute Walk 绘制，并显式维护 WidgetNode visible 与 body visible，隐藏后不能继续暴露旧按钮 hit rect。顶部模式切换清 popup；工具条 pointer 在 App 优先路由，防下层 LiveEditor 把工具条 MouseDown 当作激活新行。点击工具条捕获 popup_pointer，避免按钮处理完成后同一 MouseUp 在文档区再次弹出。range+revision 绑定延迟 click，内容变更拒绝创建过期评论；右键扩展段落路径继续保留，Esc/滚动/其它键/外部点击收起。

`typography.rs` 默认 BODY=14、H1–H6=28/22/18/16/14/14；Preferences body 8–48 比例缩放正文与标题，StyledInput 同步字号与 invalidate。原生 Markdown heading_base_scale=2，单块渲染时补偿 native heading factors，低级标题不会小于正文。实时预览/阅读同一来源且有布局高度回归；正文 pt 是逻辑单位，不是 Retina 截图像素。

## UI 开发刷新

工作台 `script_mod!` 独立于 controller 位于 `src/ui.rs`，避免主入口宏展开生成的 script block 与文件 watcher 提取数量不一致。开发 opt-in `--hot` 使用固定 Makepad 的真实文件观察器，LiveEdit → App script apply；事件后从 Rust 重建标签、Preferences 和文档投影。热更新不新增 IO / Agent 授权，正文真值、revision 保留。Rust 方法 / struct / 插值注册变化仍须编译，不把旧 live_design! 机制嫁接到 Octoscript。实际 watcher 测试默认 ignored（临时修改 ui.rs，RAII 恢复，不并发）。GUI runner 必须用项目 `test-ui-debug.py`，因为固定 makepad-test 内部硬编码 release；test artifacts debug，测试进程的 CARGO adapter 仅过滤内部 release、使用 rustup real cargo 防 shell wrapper 递归，应用 paths 为 target/debug，不改 dependency。开发入口和边界见 ui-development.md。未修改 `.runtime/`。

## 下一步

当前本地 MVP 核心流程已实现。后续可增强 inline 精确高亮、跨块 source map、word diff、时间戳、失败冲突合并 UI、IME/性能/分发。不新增外部导入、多人协作、素材库或项目数据库。

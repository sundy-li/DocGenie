# DocGenie 实施文档：本地 Markdown + 评论 + MiniMax

项目对外名称为 DocGenie，桌面 package / binary 为 `docgenie-desktop`。旧 vault `~/.agent-docs/`、Preferences 路径和 `AGENT_DOCS_*` 环境变量保持兼容。

## 表格增删行列（当前增量）

- 编辑模式单元格右键原生TableMenu：上方/下方新增行、删除行、左侧/右侧新增列、删除列。命中真实cell native区域，通过Table::position得到row/column；target绑定完整table source range与revision，不用文本搜索。阅读/readonly不提供结构菜单，切模式/文档关闭旧menu；opener释放/菜单pointer分离。
- 纯Rust document-core::table定义受限Command、表格结构parser与Workbench::edit_table；新增列同步header/delimiter/body，保留其它cell原byte内容（格式、URL、转义pipe、CRLF、列对齐），新增body行不替换header。删除表头/最后列拒绝，删最后数据行允许header-only表；默认最多64列/4096数据行。每个命令fresh revision检查，然后标准Workbench edit/revision/undo/锚点失效/3s保存路径。
- 当前支持显式首尾pipe、列数一致、有效alignment separator的表格。不支持不规则行/无outer pipe、排序/移动/复制/整表删除，菜单不提供无效项。单元格仍不允许原始pipe/换行，转义pipe原文结构不丢失。
- 新core table测试覆盖插入/删除/undo/拒绝/conflict/邻段/CRLF，GUI table_edit依次六操作、禁用header/last column、单元格后续输入、header-only保存重启与reading只读；菜单原生截图已查看。

## 未闭合代码围栏编辑（当前修复）

- `markdown_parse.rs` 在显示层调整 CommonMark 未闭合 fenced block 的行为：没有匹配 closing fence 时，opening line 与后续内容按普通 Markdown 正文呈现，不能吞到文末；输入同类型且长度不少于 opening 的独立结束行后，恢复代码渲染。支持顶层 ``` / ~~~、语言标记、0–3空格缩进，过短/不同符号/附正文的行不关闭。
- parser buffer仅以等长ASCII替换未闭合opening marker，用原文恢复Text event与原byte offsets，绝不保存该buffer；edit_projection / DocMarkdown / Reading block_ranges使用同一events契约。LiveEditor不把未闭合opening文本作为原子代码单元，新输入继续真实行编辑；closed block仍保持结构单元。不执行runsplash。
- 单元覆盖中文/emoji原文offset、无结束/更短/合法更长fence，原生fence_live逐字 ```、输入正文、再逐字结束、失焦代码与后续正文独立通过；嵌套list/blockquote fence、超长多层结构仍需专项。

## 图标操作 / hover 提示 / 侧栏归位 / 完成高亮（当前增量）

- 常用操作统一线性 SVG 图标：评论回复 send、解决 check-circle / 重开 reopen、修改对比 chevron-down / up、评论上一条/下一条、新评论、编辑/阅读切换。新建和搜索保留图标+文字，主新建蓝色底，关键状态文字不隐藏；危险删除保留确认流程。
- `UiHint` 原生非交互 overlay，悬停 400ms 后显示深色提示，离开、点击、滚动、键盘、窗口重排关闭。引用定位、diff展开/收起、发送、解决/重开、侧栏、撤销、停止、文件操作有明确提示；不抢焦点，不改正文。未宣称OS screen-reader完整验收。
- 修改对比图标只有 before!=after 可见，click沿原 thread ID切换展开，不调用模型；SVG状态只在改变时script apply，不每帧重新求值。
- 左右collapse移至各侧栏顶部内侧，center toolbar不再放这两项；隐藏后保留36pt边缘rail的展开入口。宽度策略/草稿/窄窗口行为不变，增加严格坐标断言。
- 黄色文字band只表示未解决、未处理、当前revision的评论。resolved与processed任一成立均移除；对同位置其它未处理线程保留。DocMarkdown与StyledInput使用set_comment_ranges刷新原生子组件，避免正文未变化时缓存滞留。已处理仍与resolved独立，卡片继续可回复。

## Agent 工作动画 / 无改动对比 / 已处理标记（当前增量）

- 卡片 busy（Queued / Preparing / Waiting / Validating）显示紫色原生 LoadingSpinner 与“Agent 正在处理…”，具体请求阶段继续在下一行展示。shader 使用 draw_pass.time 绘制旋转圆弧，不模拟百分比；完成 / 失败 / 取消后隐藏图标，普通本地评论不动画。不修改 runtime。
- 最近 change.before == change.after 时隐藏 view_change 按钮与展开面板（包括已有旧 no-op 记录），保留 Agent 回复 / 完成状态。解释型成功结果不 commit 相同正文，不增加正文 revision / undo；元数据与消息仍通过原 debounce 保存。
- CommentThread processed_round optional 字段标记成功处理，独立于人工 resolved；失败 / 过期 / 取消不标。追加用户回复或显式 rebind 清待处理标记，解决 / 重开不伪造新 Agent 完成。卡片顶部绿色“已处理”，保存重启恢复；兼容旧 last_change 成功记录的当前轮次推导，无字段普通评论保持未处理。
- 像素专项验证 spinner 有紫色 ink 且两帧不同；解释型 HTTP Mock 验证正文/revision不变、没有diff入口与面板、已处理及重启恢复。状态仍取真实 Agent events；动画不代表 token streaming / 原文已授权。

## 跨正文块选区 / 标签菜单生命周期（当前增量）

- LiveEditor 拖选跨块时用 source_anchor + global source range 维护共享选区，不合并输入 / 重排 block；端点来自活动 StyledInput native hit-test 或 rendered DocMarkdown source runs，保留中间 newline。每块按实际 source overlap 绘制浅色 band，起止块仅选中文字、中间块完整选中。
- 跨普通正文段落支持正反拖选、输入替换、Delete/Backspace、copy/cut（原生系统 clipboard 仍需人工验收）、精确评论与 Workbench undo；跨多块之后 Ctrl/⌘A 转整篇选择。Reading 同样支持普通正文跨段落拖选与精确评论，仍只读。素材不经搜索猜重复文本位置。
- cross range 不提供写权限：TextInput / Delete / clipboard 仅实际正文 native key focus处理，评论 / 设置输入不会被保留 selection抢走；普通 MouseDown 清跨块状态。复杂 table/coded child gap、图片或链接 glyph端点无法安全映射暂不支持，不把不确定点静默扩段落。
- TabMenu 与 App 统一 dismiss 契约：打开右键 MouseUp 消费不透传，惯性 Scroll冻结底层但不隐藏菜单，modifier不关闭。选择操作、外部 MouseDown、Escape才关闭；保持 close scopes / flush / conflict barrier不变。tab_actions增加惯性与modifier/Escape外点回归。

## 线程 Agent 进度与选区修改对比（当前增量）

- 评论卡片内紫色状态条显示真实 host/transport milestones：排队、准备选区与评论、等待 AI 回复、响应返回后校验、成功应用后完成。没有模拟百分比或模型 reasoning 展示；非 streaming chat-completions 不承诺 token 级进度。失败 / 取消 / 过期独立状态，完成不是 resolved，只有人工解决评论才移动过滤。
- worker 发 Progress/Finished 事件带 epoch、文档 UUID、thread ID 与 round；UI 按隔离标识接收，Signal 不改权限。完成仍通过 comment_request fresh revision/round/text 与 schema 校验。状态只在 session 内，重启不复活进行中网络任务；成功已处理标记从 processed_round（或旧 last_change 当前轮次）恢复。
- `CommentThread.last_change` optional DTO 保存最近一次成功应用的精确选区 before / after、base_revision / applied_revision / round；core 与正文修改/Agent 回复在 candidate 中原子提交，结果失败或冲突不产生记录。计入 MAX_STATE_BYTES、文本上限与 restore 校验；兼容旧无字段快照，不修改旧线程。
- 卡片正文发生实际变化时“查看本次修改”展开原生只读红/绿修改前后对比和 revision，按 thread ID 保存展开状态。撤销 / 后续编辑后标明历史修改，不把历史文本当当前真值，也不提供未授权 apply。当前每个线程只保留最近一次成功 AI 修改，不是全版本管理，非逐词 inline diff。

## 完整评论线程卡片（当前增量）

- 右栏改为单一 PortalList 滚动的完整线程卡片，不再分线程摘要 / 当前详情两个区域。每张白色边框卡片含灰色原文摘要、全部消息（作者 / 时间 / 正文），活动卡片顶部 5pt 黄色标识和卡片内回复框。保留未解决 / 已解决筛选、线程前后切换和新评论入口，不改变精确锚点或模型授权。
- `ThreadList` 从 Workbench 派生完整 messages，以 index 对接外层虚拟列表，同时用稳定 thread ID 路由 quote / resolve / reply / rebind。筛选改变时重新绑定 index→thread ID；App 按 thread ID 保留草稿，文档切换清空。普通 redraw 不 set_text 回旧 draft，避免丢输入 / undo / IME。
- `Discussion` 改为 Fit 高度原生消息 View，无嵌套 PortalList；从受信任 script message_template 实例化消息，插入后标记 widget tree dirty。消息不执行 runsplash。所有消息撑开卡片，长列表由外层滚动。
- 灰色原文点击选择卡片并按有效锚点定位；各卡片解决 / 重开、活动卡片回复走原 Workbench / stamp / debounce / Agent 许可。过期提示与显式重绑保留，读取 / 切换期间卡片回复只读。resolved 卡片没有回复框。
- 单机作者仍为“你” / “Agent”，头像为紫色作者标记。不绘制无效附件上传 / 分享链接；真实 AI 状态条已接入。

## 引用块高亮 / 连续换行与滚动条（当前修复）

- Markdown `>` 引用背景原先复用了黄色 theme highlight，造成精确部分评论仍看起来整块变黄。`DocMarkdown.draw_block.quote_bg_color` 改中性浅灰 `#f5f6f8`，黄色只来自有效 comment source range 的文字 band；不修改引用正文或已有评论锚点。
- 连续 Enter 新活动行越出可视区，PortalList 不绘制该行，pending focus 无法交接。输入事件在 split 后依据旧行 geometry 预先滚入新行，draw / next-frame 再检查实际 caret；仅键入与键盘上下移动 follow，布局完成前不读旧 caret cache。普通鼠标激活 / 选区 / Scroll 取消 follow，不破坏 20pt 拖选边缘规则。
- 编辑 / 阅读原生 PortalList scrollbar 使用可见灰色 handle（7pt，hover/drag 加深），溢出时显示，短文无无效轨道。编辑器右侧 12pt scrollbar gesture 优先交给 native list，避免 pointer 被行选区接管。
- `newline_scroll` 每次 Enter 检查活动输入仍在视口，38 次换行、尾部写字、保存后继续、实际鼠标拖 scrollbar 回到开头；`quote_highlight` 测试长引用部分评论，编辑 / 阅读像素核对黄色面积显著小于中性背景并检查全文不变。

## 精确选区评论 / 文档撤销 / 评论元信息（当前增量）

- 评论入口不再调用 comment_block：活动编辑的 visible→source range 直接作为评论锚点，读模式通过实际 TextFlow event source runs 与 pointer indices 映射。不靠全文搜索、不扩段落 / 列表 / 表格；无法安全映射的 child gap / decoded entity 拒绝，不偷偷改成整块。选中完整粗斜体 run 的读模式范围包含其 paired delimiter，部分 run 仍只引用内文；链接 label 不包含 URL。
- unresolved 且 current revision 评论以选中文字 band 高亮，不使用 RenderHighlight / ActiveHighlight / reading Highlight 全块底色。普通 / 粗斜体渲染按 comment range 拆 run，原生活动输入按 projection 画 band；网格 cell 也按 source range 投影。复杂 code/image/link 子控件仍有局部绘制限制，非所有 Markdown inline 高亮验收。
- 正文 Ctrl/⌘Z 统一发送 LiveAction::UndoRequested 走 Workbench，不在活动 native history 与文档 history 来回切换。格式工具 / 跨行 / AI 与普通输入共享 domain undo，失焦 / 阅读也可用；成功后恢复活动行焦点、UTF-8 安全 cursor，debounce 保存。title_input、评论草稿与 Settings 仍原生局部撤销，不误撤正文；redo 及 composition 完整专项尚未完成。
- `Message.author` / `created_at` 为 optional serde 字段兼容旧元数据；App 在用户 / Agent 消息成功追加后传入作者与 Unix 秒时间，core 不读取时钟 / 系统用户，校验限额、不改变 range/revision/round。旧消息显示“时间未知”，不伪造创建时间。单机无登录身份：作者为“你” / “Agent”。
- 灰色 `CommentQuote` 显示 46 字符缩略摘要；完整原文仍用于语义 / 锚点，不塞进只读输入。点击 quote_jump 或线程摘要定位有效锚点，过期锚点拒绝定位；Discussion 消息显示紫色作者标识、作者与本地月日时分，线程摘要也有作者 / 时间。已改为完整线程卡片列表与活动卡片内联回复框；真实等待 AI 状态条已接入，不模拟 token 进度。

## macOS 菜单 / 三栏标签 / 选区滚动（当前增量）

- `ui.rs` 配置原生 WindowMenu：DocGenie → Settings…（⌘,）打开既有全局设置，重复触发保持打开；macOS 隐藏工具栏 settings icon，其他平台保留。不是窗口内 MenuBar 模拟。Quit DocGenie 走系统 QuitRequested，dirty / worker busy 延后退出待 flush；错误取消退出。原生菜单点击和系统退出仍需人工验收，快捷键和 Settings 页面已 GUI 回归。
- page 为左 navigation / 中 center_panel / 右 comments：`center_toolbar` 与可横向滚动的 `tab_strip` 仅位于中栏。`toggle_navigation` / `toggle_comments` 位于侧栏顶部，隐藏侧栏后显示边缘rail的expand入口，复用窄窗口策略。正文限宽不变。
- `TabMenu` 根据右键目标 rel（不是当前文档）展示关闭 / 关闭左侧 / 关闭其他 / 关闭所有。关闭计划在纯 `tabs.rs`，active 被关闭时 `pending_tabs` + readonly + save barrier；保存失败不移除标签。flush 完成才更新标签与下一文档 / 空工作台；只关闭视图不删除文件。自动 H1 重命名同步 pending rel。
- 无标签时显示空工作台（新建 / 打开），Settings 和左文件树仍可用；右键操作外点 / Escape 收起，失效命中不可触发。完整规则、坐标约束、dirty 全关保存与重开回归见 `tests/tab_actions.rs`。
- 编辑选区仅顶部 / 底部 20 逻辑点启用边缘滚动。中间拖选与已选中时中间滚轮不滚动；使用 quiet selection，禁用 PortalList 自行选择接管。边缘 interval 驱动同一个 list offset，松开 / 模式切换 / readonly / 新正文停止 timer。普通正文跨块拖选已支持；复杂结构端点仍不属于全面验收。阅读选区阻止中间滚轮，同样保留边缘滚动。
- GUI 测试默认全流程 debug：`just ui` / `just render` → `tools/test-ui-debug.py`，测试进程 CARGO adapter 绕过 pinned makepad-test 内部 --release，不修改 runtime。操作说明与限制见 `docs/ui-development.md`。

## 顶部标题与正文标题输入修复（已实现）

- 顶部 `title_input` 是特殊的文档名称字段：始终仅显示标题文字，不显示合成 `#` 装饰；写入 Markdown 时仍序列化为首个 H1。删除 `title_prefix` 及其焦点显示逻辑，保留蓝色、原生 caret / IME / undo 和保存路径。
- 空正文不能仅靠 24px Blank 行命中：正文画布内点击空行下方 / filler 项也激活真实末尾空行，并在布局后 next-frame 做一次焦点交接，防同批外部点击失焦通知关掉新行；新 MouseDown 取消待交接，不抢回后续标题 / 评论点击。只影响全空正文，不把有内容的其它块误当空白覆盖。
- 正文 ATX 编辑保留 marker-only heading：逐键 `# → ## → ##空格 → ## ABC` 均可见，映射包括所有前缀分隔空白，失焦用 Markdown 渲染蓝色 H2 `ABC`；再激活恢复可编辑前缀。原测试一次粘贴完整标题未覆盖空标题 parser 中间态，现改成逐键回归。
- 新建 / 改标题 / 点击正文空白 / 输入 / 再改标题 / 选正文替换 / 保存后继续输入回归见 `tests/new_title_body.rs`；标题原生 cursor / undo 和前缀不出现回归见 `tests/title_input.rs`。

## 两阶段全选视觉（已实现）

- StyledInput 原生选区改为浅青色 `#e1efed`、直角、纯色；相邻 visual rows 的选区衔接，仍按每行实际选中文字宽度收尾，不画整块灰色圆角背景。活动块恢复白底，批注黄色高亮保留。
- 第二次 Ctrl/⌘A 使用 Rust `all_document` 共享范围，不重建 `ranges` / 合并全文输入：PortalList 保持原有活动行、正文、标题、列表、表格与图片布局。DocMarkdown 的实际行绘制 area 画浅色 band（包括列表 marker），表格由 cell input 画；顶部标题原位置高亮，不重复呈现。新增严格坐标 / height / per-block 像素回归。
- 浅色 opaque 选区先画、字形后画，避免重用 append draw-call 后高亮盖住文字（Metal 截图已发现并修正）。像素测试同时检验浅青色 band 和文字 ink，不能仅根据 buffer / selection 状态判定视觉通过。
- 全选时复制 / 剪切输出完整 Markdown 原文；输入 / Delete / Backspace 替换完整 source，并经 LiveAction → Workbench → debounce 保存。保持布局与全篇替换 / 删除 / undo / 评论已回归；standalone GUI harness 不支持 clipboard forward，系统剪贴板 copy/cut 仍需人工验收。普通正文跨块拖选已支持，复杂混合结构端点仍有边界；此能力不等于完整 Obsidian 内核。

## UI 开发热重载与字体接续

- 工作台 DSL 独立到 `src/ui.rs`，Rust App / Outline controller 保留在 main.rs。固定 runtime `--hot` watcher 可在同一原生进程更新 script_mod!；不是旧 live_design! 迁移。`LiveEdit` / `ScriptReapply` 后重新应用 Rust 状态投影，避免文档标题 / revision 标签回到声明默认值；不重跑 Startup 或触发保存 / 模型授权。
- `tools/dev-ui.sh` 为一次构建 + 常驻热更新入口，默认隔离开发 vault；调试边界和 opt-in 真实文件 watcher 测试见 `docs/ui-development.md`。
- 恢复之前停在 BISECT 的启动字体扫描 / Preferences 应用。`tests/preferences_restore.rs` 验证预存字号 / 不存在字体回退、编辑与阅读真实字号重排、激活零位移、revision 不变以及重启加载；既有 preferences 测试验证优先级移除与实际安装字体解析。已安装字体增删 / 各平台完整扫描仍不属于全部验收承诺。

## 正文空格与自动保存（已修复）

- `edit_projection.rs` 保留 CommonMark parser 裁掉的段落前后水平空白和纯空白输入 buffer，映射到原始 byte range；活动输入 redraw 不再把尾空格覆盖掉，也不再回退 caret 或清空原生 history。
- 连续空格后继续输入、保存后继续输入、undo、失焦后重新激活保留空格，正文仍是 Markdown 真值。revision 对每次有效编辑递增（用于撤销 / 冲突 / Agent 过期校验），不等同于磁盘保存次数。
- 自动保存改为最后一次实际修改后静默 3 秒触发，后续修改重启计时；正常 worker 完成不越过最新 deadline，非修改操作不延后保存。立即 flush 仅用于新建 / 切换 / 关闭 / 显式重试，保持数据安全。Preferences 关闭自动保存取消计时，重新开启为当前 dirty 文档启动计时。
- 新增无 IO 的 deadline 单元测试及原生 `tests/autosave.rs`：磁盘写入延后 / 重置、空间输入与 undo、切换立即保存。验证结果见 validation.md。

## 激活行布局稳定性（已实现）

- 不增加内容类型 wrapper / 动态脚本模板。`StyledInput::set_document_layout` 在 Rust 层移除表单默认 margin，统一为 DocMarkdown 的内边距；原 Active / Render wrapper 保持不变。普通正文 / 标题原先激活后额外 +6px 的表单 margin 已消除。
- `styled_layout.rs` 按 TextFlow `align_row_height` 的 normal font 行盒计算样式输入几何。纯文字 / 标题复用 native word wrapping，基线间距使用 TextFlow 的 row height + ascender wrap spacing；紧凑列表保留每 item 行盒、marker 缩进和上下 inset，不用 wrapper padding 抵消高度。
- 有序 / 无序列表激活时保留只读 marker，不加入可编辑 buffer；换行后的 continuation 保持 item 内容缩进，marker 仅在 item 首行绘制。当前验证的是紧凑单层列表；嵌套 / loose list / 复杂块组合仍需独立验收。
- `tests/activation_layout.rs` 覆盖正文、标题、两类列表、长文换行、长列表换行、inline 粗斜体与混合样式换行；连续 3 轮激活 / 失焦，严格比较当前盒坐标 / 高度、后续段落 y，以及文字像素垂直边界。只改变焦点不改变正文或 revision。

## 原生呈现式编辑（当前增量）

- 评论 / 就地 composer 将可见选区映射到精确源码范围，不再扩到完整 parser block；正文编辑布局的列表 / 表格 / 代码仍保持结构块，但评论不因此扩大范围。格式工具仍使用精确选区。
- 活动正文不再以 Markdown 源码作为 TextInput buffer。`edit_projection.rs` 提供 visible runs 与 source range 映射；键入先生成保留隐藏格式 / URL 的源码改动，再沿 LiveAction → Workbench → 保存 / 评论 revision 路径提交。
- `styled_input.rs` 为 App-owned pinned TextInput 派生组件，保留原生 IME、caret、selection、history，使用 `styled_layout.rs` 绘制粗体 / 斜体 / code / 蓝色链接 / 蓝色标题；未修改 `.runtime/`。派生声明见 THIRD_PARTY_NOTICES。
- 正文蓝色标题仅在获得编辑焦点时显示 ATX `#`；首个 H1 的独立 title_input 是例外，始终蓝色纯标题，不显示前缀。保存同步时跳过焦点输入与已保存标题仅首尾空白不同的重写。正文获焦点不露 `**`、`[label](URL)` 或代码 delimiters。编辑中普通点击链接是编辑文字，⌘/Ctrl+点击打开；阅读模式单击打开。
- pipe 开头的表格保持原生 TextFlow 网格，通过独立 StyledInput 编辑单元格，保留原 delimiter、空白、邻格和列对齐；不切成整块源码。锁定保存 / 切换期间同步只读单元格与标题。
- 同格式 run 编辑、完整 inline 容器替换与全选替换支持；部分跨格式和解码实体修改尚不支持，拒绝而不损坏原文。两阶段全选共享完整 source range，保留各块呈现，不暴露整篇源码。普通正文跨块拖选与 domain 撤销已支持，任意复杂混合结构端点、统一 native redo、RTL 与完整中文 IME 验收仍未完成。
- 表格增删行列已通过受限右键菜单支持；仍不支持单元格原始 pipe / 换行输入、排序/移动/复制。复杂语法不是完整富文本承诺；系统截图附件插入的人工流程仍需回归。当前增量验证结果见 validation.md。
- 旧文档中涉及活动源码的描述是历史实现，不代表当前交互；下文已更新主要契约。

## UI/UX：第一轮工作台 / 评论增量（已实现）

- App 内实现，未改 `.runtime/`、未引入 WebView 或第三方 UI 框架。保留 Markdown Live Preview、6 标签、⌘O、vault 与 Agent 授权契约。
- 正文 `article_column` 最大 760 Makepad 逻辑点，在可用区域居中；标题 28pt，作者 / 修改时间合并一行，revision 降到页脚。阅读 / 编辑与右侧页签改为文字入口，左栏增加品牌、主新建按钮和复用 ⌘O 的可点击跳转入口。
- `workspace.rs`：窄窗口布局策略。760–1099 宽且评论栏打开时隐藏导航；低于 760 两侧隐藏。自动隐藏不覆盖用户收栏意图；窄窗口点展开导航会先关闭评论。可用 `--window-size=900x800` 指定启动尺寸，参数范围受限。
- `thread_list.rs`：PortalList 线程摘要，含原文摘要、最后消息、当前线程标识与过期锚点提示；未解决 / 已解决过滤含数量和空状态。摘要点击通过已有 Workbench thread ID 打开详情并定位，不改变正文或授权。
- 回复草稿按当前文档的 thread ID 在内存隔离，切换线程 / 收栏保留，切换文档清空；不写 `.state/ui.json`。
- 评论弹窗沿用此前已有 `CommentComposer`；新增空白提交禁用，移除未实现的 `@AI` 提示，准确说明单条「问 AI」与全局默认自动修改两条授权路径。选区 B / I / U / code / link 按钮也属于本轮开始前已有能力，不计作此次新增。
- 大纲跳转不再强制切换编辑模式：阅读模式调用 `Reading::reveal`，编辑模式调用 `LiveEditor::activate_at`。
- 新增 `tests/workspace.rs`：限宽 / 居中 / 左右栏组合、搜索入口、阅读大纲跳转、900 宽度收栏与草稿、线程概览 / 筛选 / 解决重开 / 同段多线程 / 独立草稿。

仍未完成：条目级更多菜单、目录刻度浮层、长标题自动换行、完整历史快照、复杂混合结构的跨块选区与全部 Markdown 语法的精确 inline 批注。本轮是 P1 / P2 的首个增量，不代表整体设计路线完成。验证结果见 `docs/validation.md`。

## Feishu 风格交互（已落地）

- **新文档模板**：正文为空（仅 `# 标题`），由正文空行的灰色占位符「开始写作…」引导输入，不再出现正文 `开始写作。` 与占位符重复。占位符在聚焦时保持灰色（`color_empty_focus` 用 placeholder 色，`get_color` 按 empty 态混色）。加载后正文全空时自动激活末尾空行。
- **投影输入**：仅图片 / 分隔线等"可见内容为空"的段落也建立 0..0 占位 run，光标落在其中时输入前置于隐藏内容之前，不再整体拒绝按键。
- **大标题输入框**：编辑器顶部新增 `title_input`，绑到文档第一行 `# heading`；空文档显示「请输入标题」灰色占位（28pt）。编辑通过 `Workbench::set_text` 走标准 commit/revision/undo 通路，不绕过 proposal 流程。`document-core` 新增 2 项单测（覆盖正向 + 超限拒绝）。
- **author + 修改时间**：标题下方一行，「你」+「今天修改」/「刚刚修改」。「刚刚」在 save 成功后写入。
- **选区浮窗**：选中文字后浮窗包含两个按钮——`💬 评论`（点击进入批注 composer）和 `✨ Agent 完善`（Preferences `default_auto_modify` 开启时才显示）。所有 agent 相关配置已统一移到 Preferences overlay（`⌘,`），侧栏不再承载任何 agent 控件。
- **⌘+⇧+M 评论快捷键**：Feishu 标准快捷键。等价于浮窗上的「评论」按钮，无选区时状态栏提示「请先在文档中选中要评论的文字」。
- **批注高亮**：未解决且锚点有效的评论块渲染纯黄底 `#xffe999`（阅读模式与实时编辑一致，橙色边线与加重文案已移除）；badge 文字 `💬 查看批注`。
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
- 最后一次修改后静默 3 秒 debounce 后台保存；新输入自动延后。保存中有新编辑仍遵守自己的 3 秒 deadline，旧保存完成不立即追存；若 deadline 已到且 worker 忙，完成后保存最新快照。非修改 action / signal 不重新计时。切换前立即 flush，普通窗口关闭等待保存。
- 失败不丢弃内存，阻止切换与关闭，显示状态并支持重试；原子 temp+sync+rename，拒绝外部不同内容覆盖。
- 已移除路径输入、导入、手动保存、SQLite `.agentdocs` 和素材模块及依赖。旧文件不会自动迁移或删除。

### 评论与 Agent

- 编辑/阅读模式均支持选区右键原生 overlay 菜单「添加段落批注」，保留选区并聚焦右侧评论框。
- 阅读布局按 parser top-level byte ranges 分块，评论通过绘制 source runs 映射精确选中字节；编辑评论保留 projection 精确范围。不扩整段 / 列表 / 表格，也不通过搜索重复文字猜位置。复杂 child 映射不支持时拒绝，而非扩大选区。普通正文阅读跨段落选择已支持；复杂结构端点仍未全面验收。
- 本地多轮评论、逐条原生消息卡片（User/Agent 视觉区分）、解决/重新打开、前后切换、保存恢复。
- 阅读模式未解决且有效的批注选中文字持续淡黄高亮，点击段落或「查看批注」打开线程；上一条/下一条切换或点开线程时，自动按当前编辑器模式 reveal：阅读模式滚动到高亮块，编辑模式跳到该块，原文变化则显示「原段落已变化」并保持当前选区等待重新绑定；不再需要「定位原文」按钮。
- 自动修改仅在 Preferences（⌘,）中作为「默认启用 Agent 自动修改」开关存在；评论侧栏不再承载这一配置项；开启后发送评论/回复自动触发，精确选区与线程是唯一外发上下文。Agent 状态在对应线程卡片实时显示，工具栏保留「停止 Agent」，底部 status_label 补充结果。
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
- `[fdsafd](http://www.bing.com)` 显示蓝色 `fdsafd`；阅读模式单击通过系统默认浏览器打开，编辑模式单击编辑文字，⌘/Ctrl+单击打开。只允许无凭据的 HTTP/HTTPS URL，拒绝 file/javascript/data/自定义协议。
- 图片限 16 MiB / 1600 万像素，拒绝外部路径、隐藏项和 symlink；不加载网络图片，不把图片内容发给 Agent。第一版不支持复制文件路径粘贴、HTML 富格式粘贴或远程图片。
- `markdown_media` GUI 测试已通过，验证编辑/阅读的蓝色链接与真实图片像素。系统剪贴板实际粘贴和外部浏览器打开仍需人工验收；编码、协议安全、附件路径/限额/重命名/撤销已覆盖回归。

### 原生界面

本轮使用 ui-ux-pro-max 指南并参照用户截图：减少顶部工具密度、左对齐文档/大纲、文档标题占弹性空间、右侧淡色卡片与白色回复框、只在错误时显示保存重试/任务时显示停止、空线程隐藏定位与前后按钮。没有引入网页框架或照搬 skill 的 landing-page 布局。

默认「编辑」：活动普通行使用 StyledInput 呈现文字与格式，其他行 Markdown；点击别行 / 焦点转到评论框退出活动单元但不切源码。支持上下切行、Enter 新行、行首 Backspace 合并；显示选区经 projection 映射到源码后右键评论。列表 / 引用 / 围栏代码仍是结构单元，表格单独使用单元格原生输入。「阅读」只读并保留持久批注高亮。

光标颜色显式设为蓝色，0.5 秒闪烁；呈现式编辑激活新行后布局完成才聚焦，沿用原生输入 hit-test 解析点击和拖选两端，离焦停止。

Ctrl+A / Cmd+A 两阶段：第一次选当前显示行，连续再次按选择全文；自动重复按键不算第二次。第二次按键只选择完整 source range，并在现有渲染块与活动单元画 band，布局不变；整篇替换走标准 domain 通路。

选区 MouseUp 自动显示轻量「添加批注」overlay，Ctrl/Cmd+A 也显示；不抢焦点，点击后才展开右栏并聚焦评论。首次从渲染段落拖选会保留整个 down/move/up 手势，支持正向、反向、快速事件及单段自动换行。已激活行上的按下先交给原生输入框（放置光标、拿焦点、持有真实指针捕获），拖动移动仍走合成事件并按手势缓存命中映射；释放时复用缓存，不再补发合成按下——平台只在自己 mouse-up 时释放捕获，合成按下泄漏的捕获会经快速路径抢走下一次真实点击（命中最坏情况是吞掉侧栏点击），残留在下一次按下时被隔离、随后由平台释放清除。原生选区布局未完成时延迟显示工具条；隐藏按钮不会留下旧 hit 区域，工具条点击优先于下层文档。popup 绑定 range+revision，过期选区不能创建评论；Escape/滚动/外部点击关闭，右键入口保留。选区浮窗同时提供 B / I / U / code / link 格式按钮（仅编辑模式），经 workbench.edit 包裹或解包 Markdown 标记；下划线用 `<u>` 渲染，删除线因 TextFlow 无对应绘制暂未提供。

正文默认 14 pt，可在 Preferences 调为 8–48；H1–H6 在 BODY=14 时为 28/22/18/16/14/14 pt，其他字号按比例缩放。typography.rs 统一补偿 native Markdown heading scale，活动 StyledInput / 阅读一致；评论字号保留 14。

当前不是完整 Obsidian：除两阶段全文 selection 外，实时预览任意跨行 selection、跨行 native undo 共享栈、复杂结构逐行折叠、剪贴板富格式、窄窗口与 IME 人工验收仍未完成；源码模式已移除。

白底三栏沿用用户参考图：我的文档/大纲，Markdown 文档，本地评论。各侧栏顶部独立开关与折叠后的边缘入口可收起/展开左右侧栏，中间编辑/阅读区域自动填满释放的空间。隐藏保留评论草稿与线程，右键添加批注自动展开评论栏；当前会话保持布局，重启默认全部展开。原生 Markdown 的 runsplash 块只显示文本。右键菜单可 Escape/外部点击/滚动关闭。

Preferences 覆盖层（macOS 应用菜单 DocGenie → Settings… / ⌘,；非 macOS 工具栏按钮）只承载跨会话/全局配置，采用 Obsidian 风格设置窗口：左侧导航（外观 / 编辑器 / Agent），右侧对应页面，无保存按钮——每个控件修改即原子写盘并即时生效。外观页：正文字号（8–48，输入合法值立即重排编辑/阅读视图）与代码字体优先级列表（`editor_code_fonts`，从上往下取第一个已安装且可解析的字体族，运行时重定义 `theme.font_code` 的 loader 家族；空表或全部不可解析回退内置 Liberation Mono，行内 ↑↓ 调序、× 删除、下拉框从已安装字体添加，最多 8 项，状态行显示当前生效字体）。编辑器页：自动保存开关。Agent 页：默认自动修改开关与只读的 endpoint / model / key 来源展示；不暴露 API key 字段（继续走环境变量）。持久化到 `$AGENT_DOCS_HOME/preferences.json`（默认 macOS `~/Library/Application Support/agent-docs/`），原子 temp + rename 写入；缺失/格式错误/未知字段一律回退到默认并打印到 stderr，不阻塞启动；旧文件缺少 `editor_code_fonts` 字段时按默认链（Source Code Pro → Bangla Sangam MN → Andale Mono）兼容加载。已安装字体在后台线程扫描（`fonts.rs`，约 1 秒），扫描完成经 UI signal 自动应用一次配置。关闭自动保存会取消内部 queue_save，重新开启则为待保存快照启动 3 秒 debounce；切换 / 关闭仍立即 flush。

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

当前正文块批注在编辑 / 阅读均可显示，活动样式输入仍使用原生选择高亮。普通正文阅读支持跨块 source range，复杂列表/表格/代码端点不属于全部已验收能力；两次合成文本真实模型检查不能代表所有写作任务的语义质量。

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

# DocGenie 验证记录：本地 MVP 交付

以下历史记录保留原 package 名 `agent-docs-desktop`，当前名称为 `docgenie-desktop`，可执行检查命令见 README。

## 精确评论 / ⌘Z / 灰色引用 + 作者时间（本次验收）

- 最终 debug 全量 35 项 App 单测 + 71 项原生 GUI 全部通过；2 项真实网络和 opt-in watcher 默认跳过。44 项 document-core / project-store 测试通过；fmt、Clippy all-targets -D warnings、git diff --check 通过。未修改 runtime、未发真实模型，隐藏实例已退出。
- 删除两处 comment_block 扩范围：部分中文字只在 composer / thread / 持久化精确 original + range，高亮只覆盖选词；读取 source runs 而非搜索重复正文，读模式部分选择和重复格式段落整选回归通过。新增 core 精确选词 Agent 替换不动邻文 / bold URL、undo conflict、shift / resolve / reopen / rebind / restore，以及元信息限额与旧字段兼容回归。
- quote 为 CommentQuote（语义 text 保存完整 original，不是隐藏输入代理），灰色 46 字符摘要，真实 native ellipsis 随宽度裁切。点击 quote_jump 定位有效原文，过期拒绝；comment_style 验证缩略 / 点击 / author / timestamp 保存与重启恢复，截图已审阅。旧测试 API quote.value 改 quote.text，完整持久化 original 不缩略。
- Message 新 author / created_at optional serde fields 由 host 实时时钟注入；core 没有时间/provider 依赖，timestamp 不修改正文 revision，已 stamp 消息拒绝重写；旧消息显示时间未知。当前 User 作者“你”，非登录身份。Discussion 和线程摘要显示作者 / 本地月日时分；未实现所有线程独立回复输入与等待 AI 条。
- 正文 ⌘Z / Ctrl+Z 全走 Workbench（活动行 / 阅读 / 失焦 / 格式 / AI）。test precise_comments 验证按键撤格式、失焦输入、连续两次撤甲/乙、local draft 不误撤正文；test agent 两轮 Mock 更新按键撤销恢复上一轮，过期请求 core 验证。成功恢复活动焦点 / UTF-8 cursor，autosave、cell undo 回归通过。title / Settings / comment/composer 原生局部 history 保留；redo 和完整 IME 专项未验证。
- 早期全量失败包括旧断言仍期待整段引用（drag / live_preview，已改成真正部分 selection）和 remote screenshot transient；最终重跑全量通过。autosave GUI 输入间隔由 1.8s 调为 0.7s，避免 debug remote key latency 跨过首次 3s deadline；deadline 单测精确检验延期，GUI 仍检查磁盘不早存 + 最终数据。
- ellipsis 试用错误 DSL scope 导致半渲染，已停止该测试并改为正确 internal prelude TextOverflow，placeholder 与评论样式专项及最终全量重新通过。不能把中间编译通过当该版本视觉通过。
- 限制：阅读 image / code-widget / table gap 无法安全映射时拒绝精确评论；复杂 child 的局部黄色 band 不是所有语法验收。任意跨块拖选、统一 redo / typed-group undo 尚未完成；domain undo 当前按编辑 commit 粒度。comment block tree 与原文权威保持独立。

## 三栏标签 / 原生 Settings / 选区滚动 / Debug runner（此前验收）

- debug 全量 34 项 App 单测 + 66 项常规原生 GUI 全部通过，随后新增关闭保存冲突测试与 Settings 空工作台关闭回归，再跑 tab_actions 2 项 + selection_edges 1 项均通过（当前常规 GUI 总数 67）。单独 debug hot_reload 1 项通过。2 项真实网络跳过，普通全量 watcher 默认 ignored。核心 / 存储 41 项通过；fmt、Clippy all-targets -D warnings、diff whitespace 和 runner bash/Python 语法检查通过。
- `tab_actions` 实测中心栏标签边界、右键四操作、关闭最后标签进入空工作台、关闭所有前立即写 dirty 文档、重开 / 新建、空工作台 Settings；外部编辑冲突时不关闭标签 / 不覆盖文件 / 保留内存。关闭左侧指右键目标左侧，关闭其他只留右键目标。真实原生截图已检查布局和菜单。
- 两侧开关固定在中间 toolbar 两端，原 sidebars / workspace / 窄窗口测试适配统一 toggle 入口通过。macOS 已配置 WindowMenu 的 DocGenie → Settings… / ⌘, 并隐藏工具栏入口；GUI 检查快捷键、即时设置、重启恢复。隐藏 remote 实例不能完成系统菜单点击 / Quit 人工验收，不把它们标为视觉通过。
- `selection_edges` 实测：中间拖选与带选区的 Scroll 事件不移动；顶部和底部 20pt 边缘分别向前 / 后滚动；MouseUp 后坐标停止变化。已有 selection_scroll / drag selection / 全选 / 字号 / 激活零位移回归通过。阅读模式中心滚轮保护已接入，尚未独立验收所有 reading edge 手势；跨块拖选仍未实现。
- pinned makepad-test 内部 `build_release_binary` 硬编码 release 已发现。项目 `tools/test-ui-debug.py` 先编译 debug tests，再令测试 CARGO 适配过滤子 build --release；真实 toolchain cargo 避免 mbx wrapper 递归。进程及 build-stderr 确认 `target/debug/docgenie-desktop --remote` / unoptimized dev profile；不复制 release 二进制、不修改 runtime。just ui / render / test-hot 均使用新入口。初次直接 cargo test 的内部 release 启动失败，不算 debug 验收。
- 全量发现 Mock composer HTTP 服务未读完请求体即关闭，debug 下会 reset connection；已消费 Content-Length 字节后回复，专项及最终全量通过。仅 Mock，不发送真实模型。
- 独立隐藏实例已由测试结束关闭；未动用户运行中的 App 进程。debug watcher 临时改 ui.rs 已恢复，runtime pin 检查通过。尚未 commit / push。

## 顶部标题无前缀 / 新建后正文焦点 / 逐键标题（此前验收）

- 顶部 title_input 始终纯标题，不显示 `#`，保留蓝色和 Markdown H1 序列化。已移除 title_prefix；旧独立装饰实现是历史记录，非当前产品行为。
- 复现改名后正文空白区域无法激活：标题改动 deactivates 正文，Blank 行命中仅 24px，点击正文下方 filler 不激活。修复画布内空正文命中，并处理同批失焦 / redraw / 新 UID 时序；一次 next-frame 焦点交接，不循环抢焦点，新的 MouseDown 取消 pending hand-off。
- 新 `new_title_body` 严格流程：新建、3 次改标题与点击正文空白、输入、已有正文下再次改名、选正文替换、保存后继续输入。原生截图确认无顶部前缀且正文输入 caret 正常。
- 正文逐键 `#` / `##` 原因是 CommonMark 会解析 marker-only 空标题，而旧 projection 只显示带空格前缀；现在完整映射 marker 与分隔空白。2 项投影回归覆盖 1–6 级、逐键 / 反向删除；GUI 检查 `# → ## → ##空格 → ## ABC`、失焦 H2 蓝色像素、重新激活 / Backspace / 保存后仍保留。
- 最终 locked 全量：32 项 App 单测 + 64 项常规原生集成测试通过；2 项真实网络和 1 项 opt-in watcher 测试默认跳过。41 项核心 / 存储通过，fmt / Clippy all-targets -D warnings / git diff --check 通过；未修改 runtime 或调用真实模型，独立实例已退出。
- 自动保存回归旧 helper ArrowLeft 在当前 standalone remote key route 报 requested input frame could not be submitted，改 Home + End 建立相同 native undo group，不绕过输入语义；最终全量包含所有 autosave 回归通过。

## 字体接续 / 原布局全选 / 热更新（此前验收）

- 30 项 App 单测、63 项常规原生集成测试全部通过；2 项真实网络测试跳过。另单独执行 1 项 ignored 真实文件 watcher 测试通过，总原生 GUI 64 项。document-core / project-store 41 项通过。
- fmt、locked Clippy all-targets -D warnings、cargo check、git diff --check 与 dev-ui.sh bash 语法检查通过；未修改 `.runtime/`、未发送真实模型请求。隐藏实例由 makepad-test 退出。
- 恢复遗留 BISECT 中的启动字体扫描和 Preferences 应用；已安装字体链 / 设置页切换即时生效回归通过；新增预存 20pt → 即时 28pt → 重启恢复、不可解析字体回退、渲染 / 活动 / 阅读字号实际重排且 revision 不变测试。
- 第二次 Ctrl/⌘A 不合并输入、不重排：保留首个标题、正文、活动行、列表、表格网格。严格验证前后 block x/y/width/height 不变、revision 不变；每个 rendered block（包括列表）都有浅色像素、字形仍可见。截图已检查标题单次呈现、marker 和表格 cell 高亮。全篇替换与 Delete / Backspace 清空、domain undo、完整原文评论均通过；系统 copy/cut 因 standalone remote 不支持 TextCopy/TextCut forward，只检查实现，不宣称 GUI 剪贴板验收。
- 工作台 DSL 拆到 `src/ui.rs`：旧 main.rs watcher 运行时 block 数与文件提取数不一致已避开。`--hot` 真实观察器同进程热改文案再恢复，通过 UI watcher + LiveEdit；文档和 revision 保留。该 opt-in 测试临时改 ui.rs，RAII 恢复；未与编译 / 另一 Agent 并发。错误脚本恢复和所有控件状态热重载未全面验收。
- 自动保存接续验证：最后输入后 3 秒、输入重置 deadline、普通非修改动作不延后、切换立即 flush；Preferences 禁用取消已存在 timer，重新启用为 dirty 文档新计时。连续空格保存 / undo / 重新激活仍保留。
- 初次全量三处失败已定位：旧测试还期待活动蓝底；空字符串在 remote snapshot 不提供 text 字段，改验证 revision + 空正文；长文滚动测试误选 PortalList 的隐藏缓存行，改筛 visible 及完整高度。最终重跑全部通过，未只重跑部分后宣称全量通过。
- 未完成：任意跨块拖选、完整共享 native undo、系统 IME 专项、复杂结构所有边界的视觉验收。图片保持布局但没有文本 band；复制完整文档输出 Markdown，不是 rich clipboard。

## 激活行零位移修复

- 复现旧行为：普通正文 / 标题 active input y +3、后续段落 y +6；列表还因 item 行盒不同产生高度变化。根因包括 form-field margin 与 TextFlow / styled 输入行距差异，不使用模板 padding 补偿。
- Rust `set_document_layout` 去除 form-field margin，inner inset 统一；styled_layout normal metrics 对齐 `align_row_height`，wrap baseline 使用 TextFlow row-height + ascender spacing；紧凑列表保留 marker、indent、item inset 与 pitch。原 Octoscript wrapper 保持不变，未修改 runtime。
- 新 `activation_layout` 5 项均通过：正文 / 标题 / 有序与无序列表、inline 样式、长正文、长列表、混合样式换行。每项 3 轮激活 / 失焦，严格验证控件坐标 / 宽高与后续段落 y 不变，文字垂直像素范围不变，revision 不变。标题显式显示 `#`，其左侧抗锯齿边界允许 1 device pixel 差异。查看真实原生截图确认列表 marker 和缩进保持。
- `cargo test --locked -p docgenie-desktop --tests --no-fail-fast -- --test-threads=1 --nocapture`：23 项 App 单测、55 项原生集成测试通过，2 项真实网络测试跳过。随后补充 revision 断言 / 修正 single code run 风格，重跑 activation_layout / presentation / selection_scroll / typing_repro 共 12 项通过。
- 核心 / 存储 41 项通过；fmt、all-targets Clippy（`-D warnings`）、cargo check、diff --check 通过。自己的 GUI 实例已退出；没有触发模型请求，没有记录用户正文。回归日志未发现 script 求值失败，不能据此承诺消除所有 Makepad 引擎问题。
- 本轮验证范围不包含 nested / loose lists、复杂 blockquote / code fences / 图片行激活、超长标题换行与 RTL；这些结构的完整零位移仍需专项覆盖，不宣称所有 Markdown 块已验收。

## 标题连续输入前缀修复

- 根因：聚焦时把 `# ` 注入 title_input buffer，`set_text` 保留旧索引并清空 history；连续输入 / 保存同步导致前缀错位并混入用户标题。
- 修复：蓝色 `title_prefix` 为独立 Label，仅编辑标题聚焦可见，输入 buffer 不含合成前缀；仅边缘空白差异不重写聚焦输入。清空 / 重输标题复用原空行，避免累积正文分隔换行。
- `tests/title_input.rs`：逐字输入 f、等待自动保存、再输入 f，中文 / 中间插入、native undo、失焦重聚焦、尾随空格后继续输入与实际磁盘 Markdown 检查均通过。查看原生截图确认前缀只显示一次，不混入标题文字。
- `presentation` 4 项、`title_input` 1 项、`vault` 2 项原生 GUI 回归通过；核心 / 存储 41 项通过；fmt、all-targets Clippy 与 cargo check 通过。GUI 实例均退出，本修复未调用模型或修改 runtime。本次未重跑完整 App GUI suite，上次全量结果见下节。

## 原生呈现式编辑增量：最终验证

- runtime pin 校验通过，`.runtime/` 未修改；App-owned StyledInput 派生声明已补充。
- `cargo fmt -p document-core -p project-store -p docgenie-desktop -- --check`、`cargo check --locked -p docgenie-desktop` 与 all-targets Clippy（`-D warnings`）通过。
- `cargo test -p document-core -p project-store`：41 项通过。`cargo test --locked -p docgenie-desktop --tests --no-fail-fast -- --test-threads=1 --nocapture`：22 项 App 单测（含 6 项 projection 回归）+ 44 项原生集成测试通过；2 项真实网络测试跳过。
- `presentation.rs` 实测：正文获焦点不露 inline 标记 / URL，输入保留粗体和链接 source、native undo；蓝色标题像素与焦点 `#`；独立 title_input 获焦点前缀 / 失焦隐藏；直接点击链接修改 label 不改变 URL，评论绑定完整 Markdown 块；网格单元格修改 / undo 保留 pipes、列对齐与邻格。
- 更新旧 Live Preview / selection 测试，使用 StyledInput 实际 text 而非 runtime 仅识别原 TextInput 的 value 字段。旧源码显示断言改为呈现文本；评论 quote 断言使用完整 parser block，格式按钮继续精确范围。GUI 覆盖 caret blink、拖选、unicode 换行、格式 toggle、保存恢复、文件树 / 标题改名、Mock Agent 和评论冲突 / 重绑。
- 已查看合成文档的真实原生截图：标题蓝色、正文与蓝色链接在活动行保持显示、表格保持网格。期间一次注册顺序错误导致测试实例无法连接，已修复并退出自己启动的实例，后续全量正常通过。
- 清除未提交的 `/tmp/titleprobe.log` 探针（不再将正文记录到日志），保留标题在异步保存 / 切换期间只读修复。本轮不调用真实 API、不向外部模型服务发送用户素材；GUI 验证只使用合成测试文档。
- 未验收：任意部分跨格式替换、精确 inline 评论、任意跨块鼠标选区 / 共享撤销、增删表格行列、列表活动块的完整项目符号布局、RTL / 长文性能 / 完整中文 IME、系统截图粘贴的人工流程。部分跨界编辑与模糊解码实体会明确拒绝而非损坏 Markdown；不是完整富文本编辑器。

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

- 历史：首次使用源码行 + 全文 TextInput 的两阶段全选；现已移除源码模式与全文聚合输入，当前行为见本次接续验收。鼠标 / 其它按键重置首选状态，repeat 不升级。
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

限制：此修复支持单呈现单元内拖选，单段自动换行可选择多条视觉行；跨不同源码行 / 段落的任意拖选仍未支持。两阶段全选使用独立共享完整源码 range，不提供全文源码模式。修复不扩大模型权限或改变正文存储。

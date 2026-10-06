# DocGenie

**用评论驱动文档迭代的原生 Markdown 工作台。**

DocGenie 是一个基于 **Makepad + Octoscript + Rust** 的独立桌面 App：在本地写作、选中段落留下评论，按需让 Agent 修改该段并回复，再通过线程继续完善。正文始终是普通 Markdown，Rust 负责文档真值、权限与冲突校验。

> GOSIM Agentic App 2026 参赛项目 · 队伍：**A了个A**
>
> 当前为 **macOS-first 开发版**，需要从源码运行；不是已签名的生产安装包。无需 API key 即可使用本地编辑与评论。AI 功能需要自己的 MiniMax 凭据和明确授权。

## 功能概览

| 功能 | 当前实现 |
| --- | --- |
| 本地文档库 | 真实 Markdown 文件夹树，新建、重命名、删除，最多 6 个文档标签与 ⌘O 快速切换 |
| Markdown 写作 | 原生呈现式编辑：正文隐藏 inline 标记，链接直接编辑文字，表格直接编辑单元格；独立只读阅读模式 |
| 阅读排版 | 限宽居中正文、独立标题、作者 / 修改提示、大纲跳转、可收起侧栏与窄窗口布局 |
| 段落评论 | 选区浮栏 / 右键 / ⌘⇧M，原文摘要、就地评论输入、多轮回复与段落高亮 |
| 线程管理 | 线程概览、未解决 / 已解决筛选、解决 / 重开、逐线程回复草稿 |
| Agent 完善 | MiniMax 修改当前段落并回复；迟到、过期或非法结果拒绝写入；支持停止与正文撤销 |
| 自动保存 | 最后一次修改后静默 3 秒后台保存；新输入自动延后，切换 / 普通关闭前立即 flush，失败保留内存并提示重试 |
| 图片与链接 | 本地 PNG 渲染，截图粘贴插入附件的实现；HTTP/HTTPS 链接使用系统浏览器打开 |

原生界面，不使用 WebView。当前为 **Markdown 呈现式编辑**：活动正文保留样式，不切回源码；正文蓝色标题只有获得焦点时显示 `#`，顶部文档标题始终不显示前缀。尚不是支持所有 Markdown 语法和跨格式编辑的完整富文本内核。

## 快速开始

### 环境

- **macOS** 与图形桌面会话；目前验证环境为 Apple silicon / Metal。
- [Rust / rustup](https://rustup.rs/)；`rust-toolchain.toml` 固定 Rust **1.95.0**，含 `rustfmt`、`clippy`。
- Python 3、Git，以及 Xcode Command Line Tools（可用 `xcode-select --install` 安装）。
- 可选：[just](https://github.com/casey/just)，用于运行项目命令。

Makepad 支持多平台，但本项目尚未验收 Windows / Linux，不把框架支持写成应用已支持。

### 获取和运行

```sh
git clone https://github.com/sundy-li/DocGenie.git
cd DocGenie

# 下载并验证固定版本的独立 Makepad checkout
python3 tools/setup-runtime.py

cargo run --locked -p docgenie-desktop
```

`.runtime/` 不提交到仓库。脚本按 `runtime.lock.json` 固定 revision；已有 checkout 不匹配或有本地修改时会拒绝覆盖，不修改其它项目依赖。

首次启动自动创建本地文档。之后恢复文档标签、活动文档、展开目录与右栏页签。

可使用隔离目录体验，不碰已有文档：

```sh
AGENT_DOCS_HOME="$PWD/.local-state/demo" cargo run --locked -p docgenie-desktop
```

可指定启动窗口尺寸，例如：

```sh
cargo run --locked -p docgenie-desktop -- --window-size=900x800
```

### UI 开发：免重复编译调样式

```sh
./tools/dev-ui.sh
```

首次构建后保持原生窗口打开。修改 `apps/desktop/src/ui.rs` 等 `script_mod!` 内的样式 / 布局，保存文件即可在同一进程热更新；修改 Rust 逻辑才需要重启增量编译。默认使用隔离开发 vault，不修改真实文档库。字体字号和代码字体优先级也可通过 Preferences 即时修改，无需编译。

能力边界、实测 watcher 与专项测试命令见 [UI 快速调试](docs/ui-development.md)。这使用当前 Octoscript 的 `--hot`，不迁移回旧 `live_design!`，也不修改 `.runtime/`。

### 两分钟体验

1. 点击「新建文档」，输入蓝色标题与正文；点击正文直接编辑显示文字，格式标记不再露出。
2. 选中一段文字，点击浮栏「评论」或按 **⌘⇧M**；不勾选「问 AI」即可创建本地评论。
3. 在右栏点击线程摘要定位原文，回复评论，体验「解决 / 重开」与状态筛选。
4. 切到「阅读」，点击大纲跳转；收起侧栏检查限宽正文。
5. 如需 AI，先按下方配置凭据，再通过单条「问 AI」授权。完成后检查正文与回复，可用工具栏撤销正文修改。

| 操作 | macOS 快捷键 |
| --- | --- |
| 快速切换文档 | ⌘O |
| Settings | DocGenie → Settings… / ⌘, |
| 为选区添加评论 | ⌘⇧M |
| 当前行 / 全文选择 | ⌘A；连续第二次选择全文（浅色高亮，保持块布局，不露源码） |
| 撤销正文修改 | ⌘Z（输入、格式与 AI 修改统一文档历史） |
| 关闭弹窗 | Esc |

左右侧栏通过各自顶部的图标收起，折叠后在左右窄边缘保留展开入口。文档标签只显示在中栏，右键支持关闭、关闭左侧、关闭其他、关闭所有；关闭所有进入空工作台，不删除文档，待保存内容先落盘。

文件树中的重命名 / 删除作用于选中节点，只在保存空闲时执行。删除没有回收站，请先备份。

GUI 回归使用 debug：`just ui` 或 `python3 tools/test-ui-debug.py --test tab_actions -- --nocapture`。这个入口也绕过固定测试工具内部的 release 构建；直接外层 cargo test 为 debug 并不能保证 App 是 debug。详见 [UI 调试](docs/ui-development.md)。

评论按完整线程卡片展示：灰色原文摘要点击定位、全部消息显示作者和创建时间；当前卡片有黄色顶部标识和内联回复框。当前单机作者为“你” / “Agent”，历史消息缺失时间显示“时间未知”。AI 请求在对应卡片显示排队 / 等待 / 失败 / 完成；工作时显示动态图标与“Agent 正在处理…”。成功回复后标记“已处理”，新增回复恢复待处理，但不自动设为人工“已解决”。只有正文实际改变时显示修改对比展开/收起图标，hover有操作含义提示，可查看最近一次修改前后与 revision；无改动的解释回复不展示 diff，也不增加正文版本号。记录随文档保存，重启可查看。

常用工具栏与评论操作使用统一 SVG 图标（发送、解决/重开、编辑/阅读、修改对比等），悬停显示含义。绿色表示已处理，紫色表示 Agent 状态，黄色正文标记仅保留在待处理评论范围；解决或处理完成后清除。

## 配置 MiniMax

默认模型 **`minimax-m3`**，默认接口：

```text
https://api.minimaxi.com/v1/chat/completions
```

凭据读取顺序：`AGENT_DOCS_API_KEY` → `MINIMAX_API_KEY` → `~/.zshrc` 中 `MINIMAX_API_KEY` 的字面量赋值。只解析赋值，不执行 / source shell 文件。

推荐只在本地当前终端设置，不把真实 key 写进仓库、README、截图或 issue。例如 zsh 中隐藏输入：

```sh
read -rs 'MINIMAX_API_KEY?MiniMax API key: '; echo
export MINIMAX_API_KEY
cargo run --locked -p docgenie-desktop
unset MINIMAX_API_KEY
```

### 授权与隐私

- **默认关闭 Agent 自动修改**。本地评论不需要网络或密钥。
- 新评论勾选「问 AI」是单条授权；Preferences 中开启「默认启用 Agent 自动修改」会使随后发送的评论 / 回复交给 Agent，也可能处理当前线程。只在接受外发时开启。
- 外发上下文是**实际选中的文字 / Markdown 源码范围与该线程**，不会自动扩大为完整段落；不发送其它文档，不发送图片字节。
- 评论绑定精确选区，模型只能替换该范围。输出必须通过 schema、文档 UUID、revision、round 与原文一致性检查才能应用。无法安全映射的复杂语法选区会拒绝，不偷偷扩范围。
- 恢复线程本身不产生请求；但已保存的 Preferences 默认自动修改开关会在加载时恢复，应在处理敏感材料前检查。
- 停止只让本地忽略迟到结果，**不能撤回已经发出的请求**。模型调用可能产生费用。
- 不记录 API key 或完整请求 / 响应。文档与评论未加密；OS 安全凭据存储尚未实现。

开发环境覆盖：

| 环境变量 | 用途 |
| --- | --- |
| `AGENT_DOCS_HOME` | vault 根目录，必须是绝对路径；同时隔离 Preferences |
| `AGENT_DOCS_API_KEY` | 优先读取的 API key |
| `MINIMAX_API_KEY` | MiniMax API key |
| `AGENT_DOCS_ENDPOINT` | 完整 chat completions URL；HTTPS 或本地 HTTP，拒绝凭据 / fragment / redirects |
| `AGENT_DOCS_MODEL` | 模型名称；不会静默切换模型 |

## 文档存储与兼容

项目已从 Agent Docs 改名为 **DocGenie**。为了不丢失已有用户数据，旧存储路径和 `AGENT_DOCS_*` 环境变量保持兼容，没有自动迁移。

```text
~/.agent-docs/
├── 文档.md                    # UTF-8 Markdown 正文
├── 项目/
│   └── 路线图.md
├── assets/<UUID>.png           # 本地图片附件
└── .state/
    ├── <id>.json               # 评论、revision、正文一致性副本与撤销历史
    └── ui.json                 # 标签 / 目录 / 页签布局，不含正文
```

Preferences 默认位于 macOS `~/Library/Application Support/agent-docs/preferences.json`；设置 `AGENT_DOCS_HOME` 后位于该目录下的 `preferences.json`。

- 保存使用临时文件与原子 rename；发现外部不同内容时拒绝覆盖，不自动合并。
- 正文与元数据不是单文件事务；恢复不一致时以 Markdown 为准，不复活过期锚点。
- 普通关闭会等待保存；强制终止 / 崩溃可能丢失最近未落盘输入。
- 删除正文或撤销图片引用不清理附件，以免破坏共享引用；附件垃圾回收尚未实现。
- 需要完整备份时复制整个 vault，而不只是 `.md`。内容不加密。

## 架构

```text
Makepad + Octoscript 原生 UI
  文件树 / 标签 / Markdown 编辑阅读 / 大纲 / 评论
                         │ 受限命令
                         ▼
                   Rust App controller
                 ┌───────┼─────────┐
                 ▼       ▼         ▼
          document-core  project-store  MiniMax adapter
          revision /     vault /        当前块 + 线程
          threads / undo 自动保存       严格结果校验
```

| 路径 | 职责 |
| --- | --- |
| `apps/desktop/` | 原生 UI、编辑器、评论交互、Agent adapter、Preferences |
| `crates/document-core/` | 不依赖 UI / 网络 / provider 的文档与评论模型，修改 / 拒绝 / 撤销 / 冲突检查 |
| `crates/project-store/` | 文件树、路径校验、Markdown / 元数据 / 附件保存 |
| `runtime.lock.json` | 固定 Makepad 版本 |
| `tools/setup-runtime.py` | 获取与验证独立 runtime |
| `docs/` | 架构、实施状态、验证与 UI 调研 |

使用固定 Makepad `script_mod!` / makepad-script 的 Octoscript UI dialect；没有宣称接入独立 canonical workflow VM，也没有 Rinx、小程序、AppCard 或商店宿主。

## 开发与验证

先运行 runtime setup，再执行：

```sh
cargo fmt -p document-core -p project-store -p docgenie-desktop -- --check
cargo test --locked -p document-core -p project-store
cargo clippy --locked -p document-core -p project-store -p docgenie-desktop --all-targets -- -D warnings
cargo check --locked -p docgenie-desktop

# 需要 macOS 图形会话；独立隐藏实例与临时 vault
python3 tools/test-ui-debug.py -- --nocapture
```

`just setup`、`just run`、`just ci`、`just ui` 提供相应快捷入口。不要执行 `cargo fmt --all`：它会进入固定依赖的 workspace；格式化只针对本项目三个 package。

常规测试使用 Mock HTTP，不调用真实模型。真实 MiniMax 检查默认 `ignore`，需自行配置 key、接受联网及费用后显式运行：

```sh
cargo test --locked -p docgenie-desktop --bin docgenie-desktop live_minimax_paragraph_edit -- --ignored
python3 tools/test-ui-debug.py --test live -- --ignored --nocapture
```

编译、业务状态、像素检查、人工交互与真实服务验证分别记录，见 [验证记录](docs/validation.md)。

## 已知边界与后续计划

- 支持普通正文跨段落拖选、精确评论及编辑替换；尚不支持全部复杂结构的跨块端点、完整富文本、多人协作、云同步或外部文档导入 UI。
- 呈现式编辑目前支持同一格式 run 内修改、完整 inline 容器跨界替换；部分跨格式边界和解码实体的模糊修改会拒绝并提示，避免破坏 Markdown。列表的活动块尚未保留完整项目符号布局；RTL / 复杂换行与 IME 仍需专项验收。
- 表格支持单元格文字与 inline 样式编辑，编辑模式右键单元格可上下新增/删除行、左右新增/删除列，支持撤销与保存。结构操作暂限带首尾 `|` 且列数一致的表格，禁止删表头或最后一列；暂不支持移动/复制/排序或单元格原始 `|` / 换行。编辑模式普通点击编辑链接文字，⌘/Ctrl+单击打开；阅读模式单击打开。
- 已有完整线程卡片、活动内联回复、真实 Agent 状态和最近一轮选区修改对比；尚未做评论附件 / 分享、逐词 inline diff 或完整版本历史。
- 中文 IME、系统截图粘贴 / 浏览器跳转的完整人工验收、键盘无障碍、长文性能及其它 OS 仍需验证。
- 没有签名 `.app` / 安装包、OS keychain、回收站、外部编辑冲突合并或自动附件清理。
- UI 正在分阶段打磨；下一步包括条目级更多菜单、长标题换行和轻量目录入口。

进一步阅读：[实施状态](docs/implementation.md) · [架构](docs/architecture.md) · [验证](docs/validation.md) · [UI/UX 设计路线](docs/ui-ux-plan.md)。

## 许可证与致谢

项目采用 [Apache-2.0](LICENSE)。Makepad 等依赖保留各自许可证，派生 renderer 的上游声明见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。感谢 Makepad 与 OctoSense 的原生 UI / Octoscript 工作。

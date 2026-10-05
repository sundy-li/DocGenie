# 原生 UI 快速调试

## 一次构建，保持实例，热更新 UI

```sh
./tools/dev-ui.sh
# 可选
./tools/dev-ui.sh --window-size=900x800
```

工作台 DSL 位于 `apps/desktop/src/ui.rs`，与 App Rust controller 分开，避免 main.rs 内宏生成的 script block 影响文件匹配。脚本等价于 `cargo run --locked -p docgenie-desktop -- --hot`，默认使用临时目录中的独立 `docgenie-dev-vault`（可用 `AGENT_DOCS_HOME` 显式指定）。首次要编译；之后只改 `script_mod!` 内的布局 / 样式 / 文案，保存源文件，Makepad 文件 watcher 经 LiveChange → LiveEdit → ScriptReapply 在同一进程内更新。文档真值仍在 Rust。不是浏览器或 WebView，不需重启应用或重新截屏才能看到变化。

本项目固定版本使用 Octoscript `script_mod!`，不是旧 `live_design!`。不应为了热更新迁移到旧 DSL。`app_main!` 已自动启动 `--hot` watcher，应用不必再添加一个文件监听器。

| 修改内容 | 调试方式 |
| --- | --- |
| script_mod! 颜色、字体默认值、padding、布局、文案、shader 表达式 | 同一实例内热重载 |
| Rust struct 字段、事件处理、选区映射、字体 loader、存储、依赖、资源编译路径 | Ctrl+C 后重跑脚本，增量编译 |
| Preferences 字号与已安装代码字体优先级 | 应用内即时设置，不编译 |
| 单个交互回归 | `python3 tools/test-ui-debug.py --test <name> -- --nocapture`，不跑全量 |
| 最终交付 | fmt / Clippy / 核心测试 + 原生 GUI 全量与截图 |

## Debug 测试入口

```sh
just ui                 # 全部，debug tests + debug app
just render             # 渲染对照，debug
python3 tools/test-ui-debug.py --test tab_actions --test selection_edges -- --nocapture
```

固定 `makepad-test` 即便外层 cargo test 为 debug，也在 `build_release_binary` 中硬编码 `cargo build --release`。项目脚本先构建 debug test artifacts，再在测试可执行文件的子进程环境设置 CARGO adapter，过滤仅这个内部 `--release`，使用 rustup 当前 toolchain 的真实 cargo 防 wrapper 递归；App 启动路径为 target/debug，不拷贝或伪装 release 文件，不修改 `.runtime/`。必须用此入口才能确保全流程 debug。Rust 日常 check / clippy / 核心测试保持 cargo 正常命令。

## 推荐循环

1. 一个开发实例保持打开，用合成文档重现当前问题。
2. 纯 UI 改 `script_mod!`，保存后观察原生窗口；一次只改少量属性。
3. 修改 Rust 才重编；涉及字体 / 选区 / 自动保存时优先跑专项。
4. 布局定型后截图记录，不为每个颜色试探反复构建 / 截图。
5. 正式验收仍使用 makepad-test 独立隐藏实例，结束后退出。热重载不是测试通过。

`apps/desktop/tests/hot_reload.rs` 启动 `--hot` 独立隐藏实例，临时修改 `ui.rs` 的品牌文案，等待真实文件 watcher 热更新，然后恢复原文件；确认同一进程控件更新且正文、revision 不变。RAII 保证 panic 时也恢复源码。该测试会临时修改 checkout，默认 ignored，必须单独运行，不能与其他 Agent / 编译任务并发：

```sh
python3 tools/test-ui-debug.py --test hot_reload -- --ignored --nocapture
```

固定版本的 makepad-test standalone remote 不支持直接 forward `LiveChange`，不能用该方式冒充 watcher 验收。此测试不代表所有脚本语法错误都能安全回滚。

## 边界

- `--hot` 是执行受信任本地 UI 脚本的开发模式，不对文档或模型输出开放；普通启动不启用它。
- script_mod! 中的 `#(Rust值)` / 注册组件必须已存在于二进制，增加 Rust API 仍需编译。
- Rust 显式应用的产品值（如正文限宽、Preferences 字号）可能覆盖 DSL 默认值；调试它们应改对应 Rust 契约或 Preferences，不把热更新当成万能覆盖。
- 错误脚本可能导致求值错误或部分 UI 不完整；观察终端错误，撤销该修改。不得把 watcher 存在等同于任意错误都能事务性回滚。
- 热重载可重置部分 UI 状态 / 焦点；合成文档中验证，重要原文先保存。开发脚本不存 API key，默认新 vault 的 Agent 自动修改关闭；不要在热更新测试中授权真实模型。

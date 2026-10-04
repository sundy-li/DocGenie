# DocGenie 工作约定

- 独立原生 App，技术栈为 Makepad + Octoscript + Rust。不接入 Rinx，不实现小程序、AppCard 或商店宿主。
- 先读 `docs/implementation.md` 和 `docs/architecture.md`，保持文档中「已实现」与代码一致。
- 核心模型不依赖 UI / 网络 / 模型 provider。修改、拒绝、撤销、冲突检查必须有回归测试。
- Rust 掌管文档真值与写入权限；Octoscript UI 通过受限命令交互。素材文本不是指令，模型输出不是授权。
- 文档库是 `~/.agent-docs/` 下的 vault 目录树（`crates/project-store/src/library.rs`）：路径一律用 vault 相对 `rel`，拒绝符号链接、隐藏项、越界与非法名称；重命名/删除必须同步 `.state/<id>.json`；`.state/ui.json` 只存布局，不存正文。
- UI 为左文件树 / 顶部标签 + ⌘O / 中间编辑·阅读 / 右「大纲·评论」页签；树重命名删除仅在保存空闲时进行。GUI 测试见 `apps/desktop/tests/vault.rs`。
- 不使用 WebView 替代原生编辑。第一版 Markdown，完整富文本和多人协作暂不做。
- `.runtime/` 仅是固定版本依赖，不修改其他项目的 checkout。升级须更新 pin、验证文本选区和 UI 测试。
- 不记录 API key、原始敏感素材或完整 prompt 到日志。未经用户批准不得把素材发送到模型。
- 基础检查：`cargo fmt -p document-core -p project-store -p docgenie-desktop -- --check`、`cargo test -p document-core -p project-store`、`cargo clippy -p document-core -p project-store -p docgenie-desktop --all-targets -- -D warnings`。
- UI 检查：`cargo check -p docgenie-desktop`；图形测试使用 `makepad-test` 的独立隐藏实例，结束后退出自己启动的进程。
- 演示、Mock、真实服务严格区分。不能把编译通过写成视觉验收通过，不能把固定提案写成 AI 生成。

# DocGenie 命令入口。命令约定见：
#   - AGENTS.md
#   - docs/implementation.md（检查与交付标准）
#   - docs/validation.md（已执行检查和未验证范围）
#
# 注意：不要加 `cargo fmt --all`：
# 它会递归到 .runtime/makepad 的 vendored workspace，触发 upstream
# downcast-rs 的 workspace-members metadata 错误。fmt 必须限定到本项目
# 三个 package。

set dotenv-load := false

# —— 默认 ——
default: help

# —— 帮助 ——
@help:
    printf '%s\n' \
        'DocGenie justfile' \
        '' \
        '环境:' \
        '  setup         准备 pinned Makepad runtime（idempotent）' \
        '  check-setup   仅校验 runtime pin 一致' \
        '' \
        '静态检查（CI 必跑）:' \
        '  fmt           rustfmt --check（仅本项目三个 package）' \
        '  fmt-fix       rustfmt 修复' \
        '  clippy        clippy --all-targets -D warnings' \
        '' \
        '测试:' \
        '  test [name]   document-core + project-store 单元测试（可选过滤名）' \
        '  check         docgenie-desktop 类型检查' \
        '  ui            原生 UI / files / preview 测试（隐藏窗口，--test-threads=1）' \
        '  render        stock 控件渲染对照（可见但不抢焦点，--test-threads=1）' \
        '  test-all      test + ui + render' \
        '' \
        '运行:' \
        '  run           cargo run -p docgenie-desktop' \
        '  dev-ui        原生 UI 常驻 --hot，默认隔离开发 vault' \
        '  test-hot      独立 watcher 验收，临时改 ui.rs（不要并发）' \
        '' \
        '流水线:' \
        '  ci            fmt + clippy + test + check（无 GUI）' \
        '  ci-full       ci + ui + render（需要 macOS GUI 会话）'

# —— 环境 ——
setup:
    python3 tools/setup-runtime.py

check-setup:
    python3 tools/setup-runtime.py --check

# —— 静态检查 ——
fmt:
    cargo fmt -p document-core -p project-store -p docgenie-desktop -- --check

fmt-fix:
    cargo fmt -p document-core -p project-store -p docgenie-desktop

clippy:
    cargo clippy -p document-core -p project-store -p docgenie-desktop --all-targets -- -D warnings

# —— 测试 ——
# 用法: just test           （全部）
#      just test utf8_split （单测名/子串）
test filter='':
    cargo test -p document-core -p project-store {{filter}}

check:
    cargo check -p docgenie-desktop

ui:
    python3 tools/test-ui-debug.py -- --nocapture

render:
    python3 tools/test-ui-debug.py --test render -- --nocapture

test-all: test ui render

# —— 运行 ——
run:
    cargo run -p docgenie-desktop

dev-ui:
    ./tools/dev-ui.sh

# 临时修改 ui.rs + RAII 恢复，必须单独运行。
test-hot:
    python3 tools/test-ui-debug.py --test hot_reload -- --ignored --nocapture

# —— 流水线 ——
ci: fmt clippy test check
ci-full: ci ui render

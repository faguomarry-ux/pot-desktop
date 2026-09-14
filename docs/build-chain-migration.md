# 原生编译链迁移记录

记录日期：2026-09-13。此仓库基于 [pot-app/pot-desktop](https://github.com/pot-app/pot-desktop)，保留原项目 GPL-3.0 许可证和作者信息。本文记录本分支的迁移决策、构建步骤与验证边界；服务端接口修复见[翻译服务修复记录](service-repair-record.md)。

## 1. 起因与目标

在 Ubuntu 26.04 执行 `pnpm tauri build` 时，Cargo 编译 `soup2-sys 0.2.0` 失败，`pkg-config` 找不到 `libsoup-2.4.pc`。旧项目依赖 Tauri 1 所在的 WebKitGTK 4.0 / libsoup 2 编译链，新宿主使用 WebKitGTK 4.1 / libsoup 3。

这不是仅配置 `PKG_CONFIG_PATH` 就能解决的问题：libsoup 2 与 3、WebKitGTK 4.0 与 4.1 的 API/ABI 不同。没有将新库伪装成旧库，也没有混入 Ubuntu 22.04 软件源。解决方式是升级到 Tauri 2，并迁移前后端接口。

整个构建在宿主系统运行，不使用 Docker、Flatpak、Snap。Ubuntu **Universe** 是 apt 软件源；macOS **Universal** 是包含两种 CPU 架构的应用；两者均不是 Linux 通用二进制格式。

## 2. 当前编译链

```text
React / JSX / TypeScript / Tailwind
          │ pnpm install --frozen-lockfile
          ▼
Vite 5 构建静态资源 → dist/
          │ Tauri CLI 2 → beforeBuildCommand
          ▼
Cargo / Rust → tauri-build 2 → 嵌入前端资源与 capabilities
          │
          ├─ Linux: GTK 3 + WebKitGTK 4.1 + libsoup 3
          ├─ macOS: WKWebView + 系统框架 + OCR 辅助程序
          └─ Windows: WebView2（保留源码，当前新 CI 不构建）
          ▼
Tauri bundler → DEB / RPM / DMG
```

| 层级 | 迁移后选择 | 可复现来源 |
| --- | --- | --- |
| Node.js | 24 系列；本机 24.21.0 | `.node-version` |
| pnpm | 本机与 CI 12.4.1 | CI 固定版本；`pnpm-lock.yaml` |
| Rust | 本机与 CI 1.98.1 | CI 固定工具链；`src-tauri/Cargo.lock` |
| Tauri | Rust 2.11.5，JS API 2.11.1，CLI 2.11.4（本次迁移锁定结果） | Cargo 与 pnpm 锁文件 |
| WebKitGTK | 4.1 API | 宿主 `libwebkit2gtk-4.1-dev` |
| libsoup | 3.0 API | 宿主 pkg-config 与动态库 |
| Linux CI | Ubuntu 24.04 x64 / ARM64 | 原生 runner，作为比本机 26.04 更早的构建基线 |

系统包仍由发行版软件源维护，因此这不是逐字节完全可复现构建。更新依赖时应同时提交对应锁文件。

## 3. 迁移内容与对应文件

| 模块 | 修改 | 文件入口 |
| --- | --- | --- |
| Rust 框架 | Tauri 1 升级到 2，引入 v2 插件 | `src-tauri/Cargo.toml`、`src-tauri/src/main.rs` |
| 配置 | 迁移 app、bundle、plugins、平台覆盖配置 | `src-tauri/tauri*.conf.json` |
| 权限 | 为各窗口声明 v2 capabilities，HTTP URL 包含非默认端口 | `src-tauri/capabilities/migrated.json` |
| 前端 API | 文件、HTTP、通知、剪贴板、更新、Store 等迁至插件 | `src/` 中相关 imports |
| HTTP 兼容 | 保留 Body、query、response.data、文本/二进制、原始请求体对象语义 | `src/utils/tauri-http.js` |
| 文件兼容 | 旧 baseDir、目录项路径及文件监听事件适配 | `src/utils/tauri-fs.js` |
| Store | 异步加载、共享存储、reload 与空值默认值处理 | `src/utils/store.js`、`src/hooks/useConfig.jsx` |
| 桌面窗口 | 空显示器信息回退，避免隐藏窗口 panic | `src-tauri/src/window.rs` |
| GNOME 快捷键 | Wayland 下注册 GNOME 自定义快捷键，单实例转发 action | `src-tauri/src/gnome_hotkey.rs`、`hotkey.rs`、`main.rs` |
| 输入框 | 最小高度、显示后焦点、中文输入法 Enter 保护 | `src/window/Translate/components/SourceArea/index.jsx` |

保留 `com.pot-app.desktop` 标识以继续读取已有设置和历史数据。因此本分支与上游安装实例共享用户配置，不应同时运行。

## 4. Ubuntu 原生构建步骤

先安装 Node.js 24、pnpm 12.4.1 和 Rust 1.98.1，执行：

```bash
git clone https://github.com/faguomarry-ux/pot-desktop.git
cd pot-desktop
bash .scripts/install-linux-deps.sh
pnpm install --frozen-lockfile
pnpm test:adapters
cargo test --locked --manifest-path src-tauri/Cargo.toml gnome_hotkey
pnpm tauri build --bundles deb,rpm -- --locked
```

依赖脚本安装编译器、pkg-config、GTK/WebKit 开发包、AppIndicator、OpenSSL、X11/DBus/Clang、Tesseract 与 RPM 元数据工具。没有 sudo 权限时，可由管理员先安装依赖，再用普通用户编译。不要用 sudo 运行 pnpm/Cargo。

验证链接：

```bash
pkg-config --modversion gtk+-3.0 webkit2gtk-4.1 libsoup-3.0
ldd src-tauri/target/release/pot
```

本机默认 `pnpm tauri build` 仍只生成 DEB；要同时生成 RPM，显式指定 `--bundles deb,rpm`。安装同版本重编译包必须覆盖安装：

```bash
sudo apt install --reinstall ./src-tauri/target/release/bundle/deb/pot_3.0.7_amd64.deb
```

指定 `--target` 后，产物路径会增加架构目录，例如 `src-tauri/target/aarch64-unknown-linux-gnu/release/bundle/`。Linux ARM 在本文指 ARM64 / aarch64，不包含 32 位 armv7。

## 5. RPM 与 macOS

RPM 的运行依赖使用 RPM 发行版命名（例如 `xdotool`、`libXrandr`、`libxcb`、`tesseract`），不能照搬 Debian 的 `libxdo-dev`、`libxrandr2`。Tauri 还会生成框架依赖；发行版提供的 WebKitGTK 包名与 glibc 版本仍须兼容。能生成 RPM 不等于已在所有 Fedora/openSUSE 系统上安装验证。

DMG 必须在 macOS 构建：

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
chmod +x src-tauri/resources/ocr-*-apple-darwin
pnpm tauri build --target universal-apple-darwin --bundles app,dmg -- --locked
```

Universal 会先编译两种架构再合并。原有两个 OCR 辅助程序一并保留，并在打包前赋予执行权限。CI 使用 ad hoc 签名，不包含 Apple Developer 证书签名或公证；正式分发需要另外配置证书和公证凭据，不能把凭据提交到仓库。

## 6. 验证范围

已在 Ubuntu 26.04.1 amd64 完成原生构建、动态链接检查和 DEB 安装依赖解析。15 项 JS 测试覆盖 HTTP/文件兼容及 Bing；两项 GNOME 设置测试覆盖注册和清除行为。实际桌面验证了托盘输入窗口、窗口重复打开、单实例 action 转发和测试 PRIMARY 选区读取。用户也确认划词快捷键能获取文本。

真实 WebKit 测试确认输入框非零高度、Bing 中英双向请求成功，最终打包前端的译文 textarea 显示“你好，世界”。临时测试配置与真实用户配置分开，测试数据使用普通示例句子，不提交用户设置或日志。

ARM64、macOS 和 Universal 需要各自 CI 构建及实机验证；当前 Linux 主机无法证明这些平台运行正常。GNOME 原生 Wayland 应用选区仍取决于与 XWayland 的同步，截图 OCR 未做完整桌面矩阵回归。仍存在上游 screenshots 0.7.2 将来兼容性警告和 shell.open 弃用警告。

自动构建、产物下载与发布方式见 [CI 与发布说明](ci-packaging.md)。

## 7. 2026-09-14：macOS CI 编译修复

首次 CI 的 Linux DEB/RPM 构建成功，但 macOS 在 `src-tauri/src/tray.rs` 报 E0308。
`TrayIcon::set_tooltip` 在 Tauri 2 中接收 `Option<S>`，原来的平台条件分支仍传入
`&String`，因此 Linux 编译无法发现该错误。现改为 `Some(format!(...))`，同时覆盖
macOS 与 Windows 的这处调用。

应用代码警告一并处理：

- `Listener` 只在非 macOS 的截图事件分支使用，导入添加同样的条件编译。
- 托盘“查看日志”由弃用的 `tauri-plugin-shell::open` 改为
  `tauri-plugin-opener::open_path`，并注册 Rust 插件；打开失败记录日志，不再 unwrap 退出。
- `Cargo.lock` 增加 `tauri-plugin-opener 2.5.5`。此功能由 Rust 托盘事件调用，无须额外开放前端路径权限。

仍保留的上游警告：`screenshots 0.7.2` 的两个 Wayland D-Bus `method_call`
依赖 never type 回退到 `()`。这是将来 Rust 兼容性提示，当前固定工具链仍能编译。
诊断命令应在 `src-tauri` 目录中执行：

```bash
cargo report future-incompatibilities --id 1
```

报告编号随本地 Cargo 记录变化。后续可评估上游修复或最小补丁；此次没有使用
`allow` 或全局 RUSTFLAGS 隐藏警告，也没有在缺少截图实机回归的情况下升级整个截图 API。
修复后 macOS 各架构是否成功，以新提交对应的 Actions 作业为准。


## 8. 2026-09-14：DMG 完成后 lipo 找不到应用

后续 macOS CI 已进入打包后的检查步骤，但 `lipo` 找不到
`bundle/macos/pot.app/Contents/MacOS/pot`。检查当前 CLI 2.11.4 的
[Tauri bundler 源码](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.4/crates/tauri-bundler/src/bundle.rs)
确认：如果只请求 DMG，打包器会在生成磁盘映像后清理中间 `.app`。
因此这次错误属于工作流的产物保留与检查顺序问题，不是上次的 Rust 类型错误。

工作流改用 `--bundles app,dmg`，显式保留 `.app`。检查仍要求主程序存在且非空，
并用 `lipo -verify_arch` 验证 ARM64、x64 或 Universal 对应的实际架构。
上传的下载产物仍为 DMG；没有跳过失败检查，也没有用其他架构的文件替代。
Linux 构建命令不变。本机只验证工作流语法，macOS 实际打包结果以新 CI 为准。

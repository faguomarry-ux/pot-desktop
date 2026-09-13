# Ubuntu 26.04 原生构建

项目使用 Tauri 2、GTK 3、WebKitGTK 4.1 和 libsoup 3。直接在宿主系统构建与运行，不需要容器、Flatpak 或 Snap。

## 准备

使用 Rust stable、Node.js 24 和 pnpm。项目的 `.node-version` 指定 Node.js 24。

```bash
bash .scripts/install-linux-deps.sh
pnpm install --frozen-lockfile
```

如果 apt 提示找不到依赖，检查 Ubuntu 的 Universe 仓库是否启用：

```bash
sudo add-apt-repository universe
bash .scripts/install-linux-deps.sh
```

Universe 是软件仓库，不是应用沙盒。旧版 Tauri 1 需要的 `libwebkit2gtk-4.0-dev` 不属于此构建链；不要把 libsoup 3 的 `.pc` 文件或动态库链接伪装为 libsoup 2，也不要混入 Ubuntu 22.04 的软件源。

## 构建和运行

```bash
pnpm tauri build
# 开发模式
pnpm tauri dev
```

Linux 默认只打包原生 Debian 包，不要求发布签名私钥。产物位于：

- `src-tauri/target/release/pot`：直接运行的可执行文件。
- `src-tauri/target/release/bundle/deb/*.deb`：由 apt 安装依赖的原生包。

```bash
sudo apt install ./src-tauri/target/release/bundle/deb/pot_*.deb
```

也可以通过 `pnpm tauri build --no-bundle` 只生成可执行文件，或通过 `--bundles appimage` 显式尝试 AppImage。AppImage 打包工具和宿主动态库仍有兼容性要求，它不能替代 WebKitGTK 依赖链的迁移。Linux 没有 macOS 那样的 universal 二进制；x86_64 和 aarch64 需要分别编译。在 Ubuntu 26.04 构建的程序不保证兼容旧发行版的 glibc。

此分支保留原来的应用标识 `com.pot-app.desktop` 和配置、缓存、历史数据库路径。第三方 `.potext` 插件仍通过兼容层使用原来的 `http.fetch` / `Body` / `response.data` 接口。

## 验证

```bash
pnpm test:adapters
cargo check --locked --manifest-path src-tauri/Cargo.toml
pnpm tauri build
```

在真实桌面中还需检查：打开设置、翻译网络请求、托盘、全局快捷键、划词、剪贴板监听、截图 OCR。Wayland 下这些桌面集成功能受窗口管理器及各依赖库支持范围影响；编译成功并不代表它们全部经过验证。

新的原生发布矩阵见 [CI 打包与发布](ci-packaging.md)。旧 Linux 容器和固定 Windows WebView2 发布流程已移除；ARM64/macOS 需查看对应 CI 结果。

## 本机验证记录

Ubuntu 26.04.1 amd64，Rust 1.98.1，Node.js 24.21.0，pnpm 12.4.1：

- `pnpm tauri build` 已生成 `pot_3.0.7_amd64.deb`。
- `cargo check --locked` 和 15 项适配器/服务测试通过。
- `ldd` 确认链接 WebKitGTK 4.1 / libsoup 3，没有缺失动态库；`apt-get -s install` 能解析安装依赖。
- 在当前桌面会话中使用临时配置启动，`/config` 返回 200，前端成功写入设置默认值。
- GNOME Wayland 下 arboard 报告不支持数据控制协议，自动回退 X11。随后完成了下述 GNOME 快捷键与测试选区回归，截图 OCR 尚未完整验证。
- 仍有上游 `screenshots 0.7.2` 的 Rust 将来兼容性警告和 shell 打开 API 的弃用警告；当前工具链构建成功。

### GNOME Wayland 运行修复

修复了隐藏窗口尚未分配显示器时 `current_monitor()` 返回空值导致的崩溃。显示器信息改为回退到可用显示器和窗口缩放比例。

GNOME Wayland 下改用 GNOME 自定义快捷键，保留其他应用的快捷键条目。启动时根据 Pot 设置注册 `pot --action selection_translate` 等命令；已运行时由单实例插件转发操作，退出后快捷键也可以启动 Pot。修改或清除快捷键仍在 Pot 设置内操作。其他桌面保留原来的快捷键后端。

回归验证：两项原生快捷键测试通过；通过 D-Bus 实际点击托盘“输入翻译”、重复打开翻译窗口及第二实例转发划词操作均未退出，测试用 PRIMARY 选区文本成功读取。测试未模拟真实硬件按键；GNOME 原生 Wayland 应用的选区读取仍取决于其与 XWayland 的选区同步。

### Clash 与翻译空白排查

在 Pot 的常规设置中启用网络代理，地址只填 `127.0.0.1`（界面已有
`http://` 前缀），端口填 Clash 的 HTTP/Mixed 端口，例如 `47436`。
无需认证时用户名和密码留空，不代理地址保留 `localhost,127.0.0.1`。
桌面快捷方式启动的程序不一定继承终端里的 `export`，应使用应用内代理设置。

划词内容已经出现在输入框，表示快捷键和取词已完成。语言检测失败或超过
5 秒时，程序继续交给翻译服务自动检测；Google 请求超过 20 秒会报错。
Google 返回 HTTP 429 时表示该次请求受服务端限制，重新编译或重复填写代理
无法解除限制。可以稍后重试或选用其他翻译服务。

Bing 已切换到 `https://edge.microsoft.com/translate/translatetext`：请求体为字符串数组，
自动检测时不传 `from`。旧 `/translate/auth` 在本机直连和代理测试均返回 404。
微软公告指出 Edge 136 之前版本的翻译在 2026-07-30 后停止服务：
https://learn.microsoft.com/en-us/deployedge/microsoft-edge-known-issues

2026-09-13 使用 Clash HTTP 代理 `127.0.0.1:47436`，通过 Pot 原生 HTTP 模块验证：
`Hello world` → `你好，世界`；`今天的天气很好。` → `The weather is great today.`。
这是当次连通性验证，在线服务的可用性仍会随服务端状态变化。

完整的迁移决策见 [编译链记录](build-chain-migration.md)，当前服务状态见 [修复记录](service-repair-record.md)。

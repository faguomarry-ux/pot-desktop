# GitHub Actions 原生打包与发布

工作流：[Native packages](../.github/workflows/package.yml)。仓库：[faguomarry-ux/pot-desktop](https://github.com/faguomarry-ux/pot-desktop)。

## 构建矩阵

| runner | Rust target | 产物 |
| --- | --- | --- |
| ubuntu-24.04 | x86_64-unknown-linux-gnu | x64 DEB、RPM |
| ubuntu-24.04-arm | aarch64-unknown-linux-gnu | ARM64 DEB、RPM |
| macos-15-intel | x86_64-apple-darwin | Intel DMG |
| macos-15 | aarch64-apple-darwin | Apple Silicon DMG |
| macos-15 | universal-apple-darwin | 双架构 Universal DMG |

Linux 使用 GitHub 原生虚拟机直接 apt 安装依赖，不使用容器或交叉编译环境。选择 24.04 是为了在保有 WebKitGTK 4.1 的前提下采用较早 glibc 基线，本机 26.04 仍可直接编译。Linux ARM64 和 x64 分别运行原生编译器；不存在一份通用于所有 Linux CPU 的 universal 包。

runner 标签依据 [GitHub 官方 runner 文档](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)。账号/仓库类型、额度和平台可用性可能影响作业排队。RPM 兼容性说明见 [Tauri RPM 文档](https://v2.tauri.app/distribute/rpm/)。

## 触发和下载

1. 推送 `main` / `master`、提交 pull request，或在 Actions 页点击 Run workflow，执行构建与 artifact 上传。
2. 在 [Actions](https://github.com/faguomarry-ux/pot-desktop/actions/workflows/package.yml) 选择一次成功的运行，下载 `linux-x64`、`linux-arm64`、`macos-x64`、`macos-arm64`、`macos-universal`。
3. artifact 是 GitHub 包装的压缩包，先解压后安装内部 DEB/RPM/DMG；保留 14 天。普通分支推送不会创建 Release。
4. 推送 `v*` 标签时，版本校验通过且全部构建成功后，发布到[本仓库 Releases](https://github.com/faguomarry-ux/pot-desktop/releases)，同时上传 SHA256SUMS。

## 版本和发布

`package.json` 与 `src-tauri/tauri.conf.json` 的 version 必须一致，标签必须为 `v<version>`。CI 不用 sed 改写多个 JSON 行，也不依赖仓库已经有历史标签。

```bash
node .scripts/check-release-version.mjs
# 确认提交中的两个 version 均已更新，例如 3.0.8，再执行：
git tag v3.0.8
git push origin v3.0.8
```

本次只按用户要求提交和推送源码，不自动创建版本标签。版本校验仅确认前端/安装包版本，Cargo 的 0.0.0 是继承上游的内部 crate 版本占位。

## 密钥与签名

分支构建无需额外 secrets。GitHub 只在标签 release 作业中授予 `contents: write`；构建作业为只读。没有向上游仓库 dispatch、推送 Homebrew 或 WinGet 的步骤。

DMG 使用 ad hoc 签名，不等于 Developer ID 签名或 Apple 公证。未经公证的下载可能被 Gatekeeper 拦截；面向公众正式分发应配置自己的 Apple Developer 证书和公证，而非关闭系统安全机制。

`createUpdaterArtifacts` 关闭，本工作流不生成更新签名和 updater 清单。应用内继承的更新逻辑不属于此工作流，当前分支的发行包请以本仓库 Release 手动安装为准。要启用独立自动更新，需要同时配置自己的签名密钥、公开验证密钥和清单地址；不能借用原作者的签名身份。

## 验证范围和故障处理

Linux runner 额外安装 `gnome-settings-daemon-common`，为内存后端的快捷键测试提供 schema，不启动 GNOME 桌面。RPM 显式使用 zstd level 3 压缩，适用于支持 zstd 的现代 RPM 系统。

Linux 作业执行 15 项离线 JS 测试、GNOME 快捷键 Rust 单元测试，并读取 DEB/RPM 的包元数据。macOS 作业检查主程序 Mach-O 架构。构建不自动执行真实桌面翻译、OCR 或系统快捷键测试；各架构仍应实机安装验证。

如某个矩阵失败，其他架构继续构建且可保留 artifact，但不会发布不完整的标签 Release。先查看失败的安装依赖、Cargo 编译或 bundle 步骤；不要将一个平台成功解释为整个矩阵通过。首次 CI 结果以 Actions 实际状态为准。

旧 Ubuntu 22 Docker action、旧 Tauri 1 updater 打包和固定 Windows WebView2 流程已移除。Windows 源码保留，当前流水线范围为 Linux 与 macOS。

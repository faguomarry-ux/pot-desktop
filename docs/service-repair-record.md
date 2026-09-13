# 翻译服务与桌面运行修复记录

日期：2026-09-13。此文区分应用缺陷、远端服务变更和代理连通性；不能把所有错误归因于 Ubuntu 或网络代理。

## 问题与修复一览

| 现象 | 检查结果 | 修复 / 当前状态 |
| --- | --- | --- |
| 托盘打开输入翻译后进程退出 | 隐藏窗口 current_monitor 返回 None，被 unwrap | 显示器回退，避免 panic |
| Wayland 快捷键不工作 | 原后端依赖 X11 全局快捷键 | GNOME 下改用系统自定义快捷键与单实例 action |
| 输入区无高度 | 窗口隐藏时 scrollHeight 为 0 | 高度至少 50px；窗口显示后聚焦 |
| 中文候选确认触发翻译 | Enter 同时被当成提交 | 忽略正在 composition 的 Enter |
| 划词已显示但翻译区空白 | 请求被前置语言检测阻断的代码路径存在 | 检测异常或超过 5 秒后继续使用服务自动检测 |
| Google 显示 SyntaxError | HTTP 429 返回 HTML，却直接解析 JSON | 保留失败响应状态与正文；成功非 JSON 响应报明确格式错误 |
| Google HTTP 429 | Clash 代理下实际收到服务端限流 | 未解除；不自动重试轰炸服务，不把解析修复说成解除限流 |
| Bing Get Token Failed | 旧 /translate/auth 经代理、直连均返回 404 | 切换到当前 /translate/translatetext 接口 |
| 某些 JSON 请求格式错误 | 兼容层仅识别 Body 实例，不识别旧对象形式 | 同时支持 `{type:'Json', payload:...}` 等旧表示 |
| 本地自定义端口请求被拒绝 | v2 HTTP scope 未覆盖非默认端口 | 显式覆盖 http/https 的端口通配模式 |

## Bing 接口迁移

旧流程是 GET `https://edge.microsoft.com/translate/auth` 获取令牌，再 POST Translator 接口。微软[已公告旧版 Edge 翻译停止服务](https://learn.microsoft.com/en-us/deployedge/microsoft-edge-known-issues)；此次实测旧令牌入口返回 404，而非缺少本地库。

新流程：

```http
POST https://edge.microsoft.com/translate/translatetext?isEnterpriseClient=false&to=zh-Hans
Content-Type: application/json

["Hello world"]
```

自动检测不传 `from`；指定源语言时传入相应语言码。读取首项 `translations[0].text`，检测结果读取 `detectedLanguage.language`。请求超时为 20 秒，HTTP 错误和不符合预期的数据结构分别报告。

实现入口是 `src/utils/microsoft-translate.js`，Bing 翻译和 Bing 语言检测共用它；不再请求旧令牌地址。不把令牌、Cookie、用户原文或实际配置写入诊断文档。

实际验证使用 Clash HTTP 代理 `127.0.0.1:47436`：

| 原文 | 目标 | Pot 原生 HTTP 返回 |
| --- | --- | --- |
| Hello world | 简体中文 | 你好，世界 |
| 今天的天气很好。 | 英文 | The weather is great today. |

最终打包前端的 SourceArea / TargetArea 也验证了原文和译文均存在，结果 textarea 高度非零。测试只说明当时接口可用，不保证第三方服务永久可用。

## 其他服务的检查边界

- Google：当前测试返回 429；`translate.google.com` 是翻译服务地址，不是 Clash 代理节点。更换编译器或重填代理不会解除同一请求出口的服务端限流。
- DeepL 免费路径：一次 HTTP 测试返回了译文；未做完整桌面端和长文本回归。
- ECDict：测试请求遇到服务端网页验证，不能当作正常词典 JSON 使用。
- Lingva：测试出现 TLS 连接中断，未验证可用。
- Yandex：返回 Session is invalid，未修复其服务端会话协议。
- Transmart：无凭据测试返回 Auth-Failed，不能宣称为可直接使用的免密服务。
- 用户报告的 HTML HTTP 405 尚未确认所属服务卡片，不将它未经证据归因到某个接口。405 表示请求方法不被接受，也可能与过时入口或重定向相关，需定位具体 URL、方法和状态链。

## 应用内代理设置

打开“常规设置 → 网络代理”，启用后填写：

| 字段 | 值 |
| --- | --- |
| 地址 | `127.0.0.1`，不重复填写界面已有的 `http://` |
| 端口 | Clash 的 HTTP/Mixed 端口；本次为 `47436` |
| 用户名 / 密码 | 未启用认证时留空 |
| 不使用代理的地址 | `localhost,127.0.0.1` |

桌面快捷方式不保证继承终端 `export` 环境变量，所以应保存应用内代理。配置修改后退出并重新启动可排除旧进程环境的影响。

## 回归测试和后续定位

```bash
pnpm test:adapters
```

15 项测试覆盖 JSON、表单、multipart、文件/监听兼容、HTML 429、非 JSON 成功响应、语言检测失败/超时、旧请求体对象、Bing 请求构造和响应校验。自动测试不访问在线翻译服务。

遇到新问题先区分：窗口是否存活、原文是否进入输入框、请求是否发出、HTTP 状态是什么、译文是否返回并显示。不要只依据“网页能打开”判断 API 一定可用。提交 issue 时提供服务名、目标语言和状态码，避免上传 API Key、代理密码或完整用户配置。

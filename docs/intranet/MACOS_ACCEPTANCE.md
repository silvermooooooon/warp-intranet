# Warp Intranet：M 系列 Mac 放行清单

**初始状态：以下均为待执行项目，不表示通过。** 每次候选构建填写提交、哈希和证据。
静态检查、Linux 编译、GitHub job 成功、原生启动、零应用外联是不同的验证级别。
构建准备见 [MACOS_ARM64_BUILD.md](MACOS_ARM64_BUILD.md)。

## 记录模板

```text
Fork commit / upstream commit:
Source + Cargo.lock SHA-256:
App archive + executable SHA-256:
macOS version / build / hardware model:
Xcode / SDK / Metal / rustc -Vv / protoc:
Target / profile / complete Cargo command / enabled features:
Builder identity + build log (no credentials):
Signing type / Team ID if applicable / notarization submission status:
Network policy + capture mechanism / start and end times:
Tests passed / failed / skipped / not run, with evidence paths:
Reviewer / date / release decision:
```

## A. 来源与构建

- [ ] 固定官方上游和 fork SHA，清洁源码或显式保存全部补丁；lockfile 不意外更新。
- [ ] LFS 资源真实存在且校验通过；vendor 含 Git 和 registry 依赖。
- [ ] 原生 arm64 Rust、Xcode/SDK、libclang、Metal、protoc 已记录；`WARP_LOCAL_SKIP_METAL` 不存在。
- [ ] Metal 编译出非空库，禁止空文件、fake compiler、替代 shader 跳过。
- [ ] 修改涉及的 crate 测试、编译、Clippy/格式检查分别记录通过/失败/未运行；
  targeted pass 不写成全工作区 pass，不为此清单自动触发全量 presubmit。
- [ ] 最终 release 二进制为 `aarch64-apple-darwin`，不是 Linux ELF 或 Rosetta x86_64。
- [ ] 最终 feature 集未恢复 AI/云登录、远程服务、自动更新、崩溃上传、遥测。
- [ ] 无 Warp/GCP/Apple 账号凭据也能完成测试包构建；正式签名与构建分离。
- [ ] 若声称“可离线构建”，在隔离 Mac、新 target、空 Cargo 下载缓存下以 vendor + `--frozen`
  重建成功；build.rs/原生工具也没有联网能力。保存实际过程，不仅保存命令。

## B. 包、签名与本地资源

- [ ] bundle ID 是 `dev.warp.WarpIntranet`，名称是 Warp Intranet，入口为 `warp-oss`；
  不冒充官方签名身份，不覆盖已有 Warp 安装/账号数据。
- [ ] `Info.plist` 语法、版本、CPU 架构、最低系统字段与 Mach-O 一致。
- [ ] 主二进制与每个嵌套可执行文件都检查架构/动态库路径。不能依赖构建机 `/opt/homebrew`、
  源码路径或未随包交付的 dylib；签名前处理依赖，签名后不再修改 bundle。
- [ ] 图标、内嵌字体/主题/shell bootstrap、需要的本地资源和许可证齐全；
  不附带 AI agent skills、遥测 SDK、自动升级器或不必要的服务端 helper。
- [ ] 若有 DockTile plug-in，检查 arm64、自己的 MainAppBundleIdentifier 及嵌套签名；
  若不带，plist 不引用缺失 plug-in。
- [ ] 签名验证通过；ad-hoc、Developer ID、是否公证如实标注，不把其中一项当作另一项。
- [ ] 若声称公证：Accepted、stapler validate、最终 ZIP/DMG 内实际 app 的核验均有证据。
- [ ] ZIP/DMG 可完整解包；校验清单在最后封装之后生成，传输后哈希一致。
- [ ] 无签名私钥、token、用户历史、个人配置或带凭据 remote 混入归档。
- [ ] `cargo-about` 排除的 private/无 license 元数据依赖已人工核对；不能只因生成清单命令成功
  就称全部第三方归属完整。记录 CI 低优化候选与最终 release 的差异。

### 不启动 app 的检查命令示例

以下仅检查操作者指定的候选包，不安装、不签名、不更改系统策略。
从最终 ZIP/DMG 解出的 app 检查，而不是只检查原始构建目录。

```bash
APP="/path/to/Warp Intranet.app"
BIN="$APP/Contents/MacOS/warp-oss"
plutil -lint "$APP/Contents/Info.plist"
plutil -extract CFBundleIdentifier raw -o - "$APP/Contents/Info.plist"
file "$BIN"
lipo -archs "$BIN"
otool -L "$BIN"
otool -l "$BIN"  # 核查 LC_BUILD_VERSION / LC_VERSION_MIN_MACOSX
codesign --verify --deep --strict --verbose=2 "$APP"
codesign --display --verbose=4 "$APP" 2>&1
shasum -a 256 "$BIN"
# 仅已公证包应通过这一项；未公证包失败是如实记录，不应绕过。
xcrun stapler validate "$APP"
```

`codesign --verify` 的成功不等于 Gatekeeper 接受。`otool -L` 也要递归检查嵌套库/插件。
GUI/安全策略核验应在新测试账号或干净受管测试 Mac 上执行，保持 Gatekeeper 与 SIP 开启。

## C. 脱离构建环境的首次启动

- [ ] 在非开发机或干净测试账号上，从实际分发渠道取得候选包，保留正常 quarantine 行为。
- [ ] 复制 app 到约定位置后，从与源码无关的工作目录启动；源码、target、vendor 不可见，
  不依赖已装 Rust、Xcode、Homebrew。
- [ ] 第一次启动前就切断互联网，而不是登录成功以后再断网；同时确认内网目标仍按测试设计可达。
- [ ] 保持默认 Gatekeeper/SIP 和组织策略；有拦截就记录签名/公证/策略原因，不删除 quarantine
  或关闭安全保护。必要时由 IT 通过批准的受管分发流程处理。
- [ ] 无账号、无缓存 token、无已下载字体或云端状态也能直接出现可用 shell；
  不出现登录/注册/激活/云 onboarding 阻塞。
- [ ] 重开、重启系统、连续新建窗口/标签、关闭最后窗口后恢复仍正常。
- [ ] 在组织实际最老与最新受支持 macOS 各测试一次；最老版本未测试就不声明最低版本支持。

## D. 终端功能（必须留存可复核结果）

- [ ] `/bin/zsh`、`/bin/bash` 本地交互命令；使用干净 shell 配置做基线，再用实际配置复测。
- [ ] 管道、重定向、退出码、工作目录、长输出、Ctrl-C、Ctrl-D 和交互式程序。
- [ ] 标签、分屏、窗口缩放、全屏、睡眠/唤醒、会话/本地历史恢复。
- [ ] 中文输入法预编辑/候选、中文与 emoji/wide character、组合字符、字体回退。
- [ ] 复制粘贴与多行粘贴确认、selection、快捷键、鼠标滚动、链接和本地文件操作。
- [ ] 全屏 TUI（如本机已有的 vim/less）能进入/退出并正确恢复终端。
- [ ] Metal 正常渲染，无空白、shader pipeline 错误、持续 beachball；记录实际 M 系列型号。
- [ ] 本地设置保存再打开生效，不需要云配置；缺失在线图片/字体时优雅使用本地资源。
- [ ] 启动日志没有连续认证/连接重试、后台任务风暴或大量网络失败弹窗。

## E. 网络负向与正向验收

**仅无网时没有成功连接，不证明没有连接尝试。** 必须结合代码/feature 审计、策略回归测试、
完整观察窗口中的连接/DNS/监听记录以及可达测试网络上的负向测试。

1. 由网络管理员提供有日志的测试出口控制；测试数据不含真实秘密。
   分别检查 IPv4、IPv6、DNS、HTTP(S)、WebSocket 和 loopback。普通 macOS 应用防火墙的
   入站设置不能当成出站阻断证明。
2. 在冷启动之前开始记录。包括启动、空闲、用户操作、睡眠唤醒、重开/重启等整个场景。
   观察窗口应覆盖本次改动涉及的定时器，记录窗口和覆盖范围；不宣称有限抓包证明永远无外联。
3. 区分 app PID、应用启动的 helper、PTY 子进程和 macOS 自身服务。DNS 可能由系统服务代发；
   只看 app 的 `lsof` 快照会漏短连接和 DNS，不能单独判为通过。
4. 在允许网络可达的隔离测试网复测，故意提供测试 server URL、proxy、保存的旧账号配置、
   可见功能入口，证明禁用策略不可通过设置/环境变量/旧数据开启。
   不向真实生产服务或未经授权目标发起请求。
5. 单独执行一次获准的 PTY 内网操作（测试 SSH/git/HTTP）。应成功且明确归因于用户命令，
   同时应用自身没有附加的认证/统计/远程配置请求。不要为“零连接”把全部 shell 网络禁掉。

- [ ] 应用和自动 helper 不尝试外部认证、Firebase、GraphQL、遥测、Sentry、更新、云配置、
  字体/素材 CDN、账户头像、AI/代理/远程 MCP 等请求。
- [ ] HTTP/WebSocket 统一策略拒绝所有目标，包括 loopback；失败返回可预测，不发生后台重试风暴。
- [ ] 无本地 HTTP/WS 管理端口、MCP OAuth callback、AI/远程终端服务被自动打开；
  必需的本地 Unix-domain IPC 单独记录，不误认成互联网连接。
- [ ] 登录、Drive/共享、AI/Agent、云会话、云设置、邀请、升级等入口不存在或清楚禁用；
  可达网络/旧配置也不能绕开策略。
- [ ] 正向 PTY 内网 SSH/git 等仍能工作；失败时明确区分组织网络限制和应用回归。
- [ ] 保存 redacted 日志、进程树/连接事件、策略测试结果；任何未覆盖出口列为风险，不写零外联认证。

### 可自动化的测试脚本分层（设计，不是已执行结果）

1. `preflight`：记录 OS/arch/toolchain；拒绝 Metal skip；检查 LFS 指针、固定 lock、feature 集。
2. `policy-regression`：HTTP GET/POST、HTTPS、WS/WSS、127.0.0.1、localhost、`::1`、proxy、
   URL override、listener bind 路径都拒绝；测试 fixture 不启动生产客户端、不访问真实端点。
3. `package-static`：实现上面的 plist/arm64/dylib/signature/hash 检查；缺依赖即失败。
4. `native-smoke`：干净账号、独立路径和受控网络下自动/人工混合完成 UI、PTY、字体和恢复场景。
5. `report`：输出每个门禁的 PASS/FAIL/NOT RUN 和证据位置；任意关键项 NOT RUN 不可输出 overall PASS。

自动测试即使通过，也不能证明所有残留服务端代码都已删除。把源代码净化范围、依赖树残留和
应用运行时外联验证分别报告；新增依赖/入口必须重新经过这些门禁。

# Warp Intranet — macOS / Apple Silicon

> **当前是待原生验收的源码候选分支，不是已验证可安装版本。**
> 原生 macOS 编译、全新用户启动、抓包和内网 SSH 验收完成之前，不应投入生产。

基于官方开源客户端 `warpdotdev/warp` 的
`65229eb524f978cd006c54424fad3a98a35c39c7`，保留完整 Git 历史及原有版权/许可证。
这是独立改版，不是 Warp 官方发行版。

## 本分支做了什么

- **默认不依赖账号启动**：跳过登录、Firebase 匿名账号和云 onboarding；不读取/恢复 Warp
  凭据，不要求外网成功后才创建终端。使用独立应用 ID `dev.warp.WarpIntranet` 和
  `~/.warp-intranet` 配置目录，不自动迁移/覆盖旧安装；不注册浏览器登录自定义 URL scheme。
- **不可被设置重新开启的应用网络策略**：`crates/network_policy` 永远拒绝应用自己的网络
  操作，没有环境变量、feature flag、localhost、内网地址、代理、重定向例外。
- **在传输之前阻断** HTTP、OAuth、SSE、WebSocket、远程图片/字体、MCP、OTLP、
  崩溃上报、本地 HTTP 控制服务及 ACP TCP。保留终端运行需要的 Unix IPC 和 PTY。
- **停止遥测收集和持久化**，不只是设置一个“不要上传”选项；关闭自动更新、远程配置、
  账号/云同步/托管 AI/共享会话的特性入口。新上游 feature flag 需要显式审核后才能编译。
- **终端优先的默认构建特性**：不默认启用 AI、云功能、MCP、语音、分类模型或 agent skills。
- **隔离上游自动化**：原厂 release/bot/sync 工作流归档到 `.github/upstream-workflows`。
  当前仅保留手动策略检查，以及手动或带 `[verify-intranet-macos]` 提交标记触发的
  公共标准 macOS arm64 构建候选流程；普通源码推送不启动构建，不发布 release。

用户在 shell 中运行的 `ssh`、`git`、`curl` 以及 shell 初始化脚本**不受这个应用策略限制**；
这是为了保留正常内网工作能力。策略也不是 macOS 系统级防火墙，不能替代组织出口控制。

## 没有声称完成的工作

**这不是“所有服务端相关源码/依赖已物理删除”的版本。** 为降低破坏终端核心的风险，
现阶段保留了不可达的云数据类型、源文件和部分依赖，用不可变策略及入口限制封住它们。
全部物理裁剪、减小依赖树，以及对未来上游代码做完整调用图审计仍是独立工程工作。
有些上游测试本来要求网络、账号或云 feature，本分支会故意不再满足这些假设。

测试边界：

| 检查 | 当前证据 |
| --- | --- |
| 不可变策略 Rust 单测 | 锁定依赖的实际 Cargo 测试已通过 2/2 |
| feature crate 单测 | 锁定 enum-iterator 1.4.1，6 通过、1 个上游原有忽略；包括 4 项内网策略测试 |
| 静态网络出口回归检查 | 已通过；只检查已列明的直接出口和关键 guard，不是形式化零联网证明 |
| HTTP/WS/asset_cache 全库单测 | 48/48 通过，含 DNS 前拒绝、代理/重定向、OAuth、SSE、loopback、凭据不读取/泄露 |
| 路径隔离单测 | Linux 路径与跨前端身份映射 13/13 通过；macOS 专用分支仍需原生验证 |
| 相关库 Clippy | network_policy/warp_features/http_client/websocket/asset_cache，所有 target/tests，`-D warnings` 通过 |
| macOS arm64 编译/打包/启动 | 尚未验证；不提供伪造的“成功”二进制 |
| macOS 断网、代理、重定向抓包及内网 SSH | 尚未验证；必须按验收清单完成 |

上述锁定依赖检查共 69 个测试通过、1 个上游原有忽略；不等于整工作区/GUI 测试。
详见 [验证记录](docs/intranet/VALIDATION.md)。工作流文件存在不代表 macOS 构建已经成功。

## 构建与验收

详见 [Apple Silicon 构建与内网交付](docs/intranet/MACOS_ARM64_BUILD.md) 和
[macOS 验收清单](docs/intranet/MACOS_ACCEPTANCE.md)。

```bash
# 不解析 Cargo 全工作区依赖的快速策略检查
rustc +1.92.0 --edition 2024 --test crates/network_policy/src/lib.rs -o /tmp/intranet-policy-tests
/tmp/intranet-policy-tests
python3 script/check-intranet-policy.py

# 依赖已准备好时运行真实 transport 回归测试
cargo +1.92.0 test --locked -p http_client intranet_tests
cargo +1.92.0 test --locked -p websocket intranet_tests
cargo +1.92.0 test --locked -p asset_cache intranet_tests
cargo +1.92.0 test --locked -p warp_features --features test-util intranet_tests
```

`script/build-intranet-macos.sh` 只接受原生 Apple Silicon macOS，固定 Rust 1.92.0 和目标
`aarch64-apple-darwin`；不替你安装依赖、不禁用 Gatekeeper、不移除隔离属性、不上传发行物。
默认是 ad-hoc 签名候选，**没有 Apple 公证**。离线构建需要事先准备完整 vendor、工具链、
Xcode/Metal 和资源，详见构建指南；只下载 GitHub source ZIP 不足以离线重建。

## 调研结论

- [warp-lite](https://github.com/terzigolu/warp-lite) 已有 macOS arm64 alpha 发行物，是值得参考的
  精简项目，但将 URL 换成无效域名不等于所有网络出口都被拒绝；未直接采用其二进制。
- [warp-offline](https://github.com/the1812/warp-offline) 提供网络策略设计参考，但已验证范围偏
  Windows，local-only 网络允许例外与本项目“应用完全不联网”的边界不同。
- 本分支直接基于上述官方 SHA 新实现更严格的 deny-all 应用策略，没有盲目移植第三方补丁。

## 许可证与发行责任

保留根目录 `LICENSE-AGPL`、`LICENSE-MIT` 及所有第三方声明。上游应用主体为 AGPL-3.0-only，
UI 框架为 MIT；本次新写的独立 `network_policy` 小 crate 为 MIT，以保留 MIT UI 框架的许可边界。
分发修改后的二进制时，应按适用许可证提供对应源码及必要声明。`cargo-about` 输出只是输入，
不是完整许可清查证明；尤其须人工复核上游 about.toml 跳过项和非 Cargo 资源许可。

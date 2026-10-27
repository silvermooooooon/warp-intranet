# 验证记录（2026-10-27）

环境：任务专用 Linux x86_64，官方 Rust/Cargo/Clippy 1.92.0。
所有 Cargo 命令使用 `--locked`；`Cargo.lock` 没有在验证中漂移。
应用运行时代码基于 `7c300571f1b96f14ca2ef6fd1f6975e1361f5bc3`，本轮后续修改为测试、文档及 CI 触发配置。

## 已通过

| 命令/范围 | 结果 |
| --- | --- |
| `cargo test --locked -p network_policy -p warp_features --features warp_features/test-util` | network_policy 2 通过；warp_features 6 通过、1 个上游原有忽略；doc-tests 0。实际 enum-iterator **1.4.1**，不是此前隔离测试的 1.5.0。 |
| `cargo test --locked -p http_client -p websocket -p asset_cache --lib --no-fail-fast` | http_client 15、websocket 26、asset_cache 7：全部通过。 |
| `cargo test --locked -p warp_core paths::tests` | 13 通过；仅当前平台适用测试，不包含 macOS 专用 cfg 分支。 |
| `cargo clippy --locked -p network_policy -p warp_features -p http_client -p websocket -p asset_cache --all-targets --tests --features warp_features/test-util -- -D warnings` | 通过。 |
| `python3 script/check-intranet-policy.py` | 关键 guard 与 17 个直接 transport 文件清单检查通过。不是全程序零联网证明。 |
| `cargo metadata --no-deps --locked --offline`、manifest/lock 边校验、YAML、`bash -n` | 通过。所测目标的锁定依赖构建已由上述 Cargo 测试进一步验证；不是整个工作区构建声明。 |

共 **69 个相关库测试通过，1 个既有忽略**。这不是整工作区测试通过声明。

### Linux 原生依赖说明

首次 transport 编译在 fontconfig 的 pkg-config 检测处失败，因为当前云环境没有开发包的
`fontconfig.pc`，但已安装真实运行库 `libfontconfig.so.1`。验证改用该上游依赖公开支持的
`RUST_FONTCONFIG_DLOPEN=1` 动态加载模式后通过；没有伪造 pkg-config 元数据、替换库或使用 mock。
此选项仅用于本轮 Linux 测试，不放进 macOS 构建工作流。

### 对上游测试的有意调整

扩大到完整库测试后，发现 4 项原测试仍要求被本分支明确删除的行为：读取 IAP 凭据并添加 header，
以及 3 项代理 CONNECT/认证/407 网络交互。它们没有被忽略或悄悄跳过，而是改为断言新契约：

- 请求构造不调用 IAP token provider；外部请求仍保留显式签名 header。
- 直接调用底层代理连接也返回 `NetworkDisabled`，监听 socket 无连接。
- 代理凭据不能恢复连接，错误不包含凭据；无效目标也不能绕过策略。

首次错误过滤器 `paths_tests` 匹配 0 项，**不计入通过数**；随后用实际模块名 `paths::tests`
重新执行了上表 13 项测试。

## 尚未建立的结论

- app crate/整个工作区的编译、GUI 单测及端到端测试尚未通过本记录认证。
- macOS arm64 AppKit/Metal 编译、打包、原生加载与首次启动需要独立 CI/测试结果。
- CI `--help` 冒烟检查即使通过，也只证明原生二进制可加载；不证明终端窗口、PTY、字体、输入、
  睡眠恢复或完全断网启动正常。
- DNS/TCP/UDP/代理抓包、干净配置/恢复配置、用户 shell 内网 SSH/Git/curl 见
  [MACOS_ACCEPTANCE.md](MACOS_ACCEPTANCE.md)，仍须执行。
- 尚无已公证或已验收的正式发行物；未声称服务模块/依赖已全部物理删除。

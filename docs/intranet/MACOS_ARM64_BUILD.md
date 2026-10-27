# Warp Intranet：Apple Silicon 构建与内网交付

适用目标：M 系列 Mac，`aarch64-apple-darwin`。上游基线：
`warpdotdev/warp@65229eb524f978cd006c54424fad3a98a35c39c7`。
调研日期：2026-10-27。

**本文是构建方案，不是构建成功报告。** 尚未在本文中认证任何 macOS 二进制、CI 结果、
最低 macOS 版本或断网运行结果。放行条件见 [验收清单](MACOS_ACCEPTANCE.md)。

## 先分清三个目标

1. **内网运行**：准备好的 `.app` 在没有互联网、Warp 账号、Warp 服务端的机器上运行。
2. **离线构建**：源码、Rust、Xcode/SDK/Metal、全部依赖和打包工具先在联网准备区收集，
   再转入隔离构建机；仅把 Git 仓库复制进去不够。
3. **完全删除云代码**：与禁用运行时出口不同。禁用的源文件/依赖仍可能留在树中；
   不得把网络拦截或隐藏 UI 描述为已完成全量物理删除。

本分支的运行策略应为：应用自身不发起 HTTP/WebSocket 请求，也不通过 localhost 接回服务端。
用户在 PTY 中主动执行的 `ssh`、`git`、`curl` 等仍需能访问获准内网服务。
这不是系统级沙箱；shell 初始化脚本、用户运行的命令、系统安全服务有各自的网络行为。

## 已有项目的参考价值

[warp-lite v0.5.9-lite](https://github.com/terzigolu/warp-lite/releases/tag/v0.5.9-lite)
已有 Apple Silicon 的 `WarpLite.dmg` / `WarpLite.app.zip`。发布者记录了 macOS release 构建、
启动和部分测试，但明确没有声称完整 hosted CI/全工作区通过；还列有未完成的 rmcp 迁移。
发行物是 **ad-hoc 签名、未经 Apple 公证**。这是可参考的打包先例，不是本分支的测试证据，
也不是零联网审计证明。其
[最小 app 打包脚本](https://github.com/terzigolu/warp-lite/blob/v0.5.9-lite/script/build-warp-lite-app.sh)
可以说明无需 Warp 私钥也能生成测试用 app；不能据此保证 Gatekeeper 放行。

## 1. 联网准备区：工具链

优先使用与目标系统相近的 Apple Silicon Mac。不要在 Linux 上把 `cargo check` 当成 macOS
AppKit/Metal 构建证明；也不要把 Rosetta 下的 x86_64 构建标为原生 arm64。

| 输入 | 基线要求与核实方法 |
| --- | --- |
| Rust | 根目录 `rust-toolchain.toml` 固定 **1.92.0**，包括 rustfmt、clippy、rust-analyzer。安装 `aarch64-apple-darwin` 标准库；保存 `rustc -Vv` 和 `cargo -V`。 |
| Xcode | 完整 Xcode，不只是 Command Line Tools；选择对应 `Contents/Developer` 并完成首次启动、许可与组件安装。保存 `xcodebuild -version`、SDK 路径/版本。上游未给出此基线已验证的唯一 Xcode 版本。 |
| Metal | `crates/warpui/build.rs` 实际调用 `xcrun metal` 与 `metallib`。Xcode 版本需要时先下载独立 Metal Toolchain，验证两个工具能工作。 |
| 原生编译工具 | Xcode clang/clang++、make、可被 bindgen 找到的 libclang；CMake、pkg-config/pkgconf、protobuf (`protoc`) 按最终依赖树准备。`remote_server/build.rs` 会生成 protobuf。 |
| Git / Git LFS | Git checkout 不是 LFS 对象内容。上游 LFS 目前包括 input classifier ONNX 模型；即使内网产品不启用模型，也要记录哪些输入被省略及理由。 |
| 打包工具 | macOS 自带 `plutil`、`codesign`、`ditto`、`hdiutil`、`sips`、`iconutil`；仅采用 cargo-bundle 路径时需额外准备上游固定的 **cargo-bundle 0.11.0**。 |

本 fork 的 `script/build-intranet-macos.sh` 不使用 cargo-bundle，不自动安装任何工具，
也不登录、发布或公证。它需要预先生成的 Cargo 第三方归属清单，准备区使用固定的
**cargo-about 0.8.4**。默认只构建 terminal feature allowlist，不启用 `gui`（该基线的这个
feature 只是增加 voice_input）、classifier 或 bundled_skills。

不要直接把上游 `script/bootstrap` 当作最小离线准备脚本：它会拉取 LFS、Homebrew、
PowerShell、Docker、gcloud、Sentry CLI 和通用 agent skills 等；跳过 gcloud 登录不等于跳过这些安装。
准备区按清单单独安装、记录版本，避免导入不必要的账号或工具。

以下命令供已批准的联网构建机执行；本次编写文档未执行安装或编译：

```bash
# 从仓库根目录执行。先由管理员完成 Xcode 安装与首次启动。
export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
xcodebuild -version
xcrun --sdk macosx --show-sdk-version
xcrun --sdk macosx --find clang
xcrun --sdk macosx --find metal
xcrun --sdk macosx --find metallib
rustup toolchain install 1.92.0 --profile minimal \
  --component rustfmt --component clippy --component rust-analyzer
rustup target add aarch64-apple-darwin --toolchain 1.92.0
rustc +1.92.0 -Vv
protoc --version
git lfs version
```

新 Xcode 若缺少 Metal，联网阶段使用 `xcodebuild -downloadComponent MetalToolchain`。
需要跨隔离边界转移组件时，按所选 Xcode 的 `xcodebuild -help` 和
[Apple 组件导出/导入说明](https://developer.apple.com/documentation/xcode/downloading-and-installing-additional-xcode-components)
准备安装包；不要假设只复制 Xcode.app 就带上了独立 Metal 组件。
归档 Xcode/SDK/组件安装介质时遵守组织和 Apple 的许可管理，不放进公开源码仓库。

**禁止发行时设置 `WARP_LOCAL_SKIP_METAL`**：该基线的构建脚本只检查变量是否存在，
设为 `0` 也会写出空 `shaders.metallib`。这只能骗过部分静态检查，不能生成可用的 GPU 终端。

## 2. 固定源码、LFS 和依赖

1. 固定实际要交付的 fork 提交，不跟随移动的 `master`。记录上游 SHA、fork SHA、
   `Cargo.lock` SHA-256、补丁及所有非默认 feature/profile/environment 选项。
2. 保留上游 remote 以取得该提交对应的 LFS 对象。在联网准备区执行 LFS fetch/checkout 和
   `git lfs fsck`；检查工作树中不再把 `version https://git-lfs.github.com/spec/v1` 指针当资源。
   若 fork 的 LFS 存储尚未同步，从已核实的上游取得对象，不把失败忽略掉。
3. 使用已固定的 Rust 和 lockfile vendor **registry 和 Git 依赖**。Cargo vendor 不是工具链、
   Homebrew bottle、SDK 或任意 build.rs 下载行为的归档器。

```bash
# 联网阶段，从根目录执行；不要覆盖已有 .cargo/config.toml。
cargo +1.92.0 vendor --locked --versioned-dirs vendor \
  > .cargo/intranet-vendor.toml
# 后续命令显式带上此配置；保留 .cargo/config.toml 的 macOS 编译配置。
cargo +1.92.0 --config .cargo/intranet-vendor.toml metadata \
  --locked --offline --format-version 1 > dependency-metadata.json
```

保存整个 `vendor/`、新生成的配置以及完整源码/资源。不要只保存 `$CARGO_HOME/registry`：
本项目还有 Git 依赖、patch 和本地工作区 crate。打包/检查工具如 cargo-bundle、cargo-about、
nextest 的安装依赖也不由工作区 vendor 自动涵盖，需独立固定与准备。

[`cargo vendor`](https://doc.rust-lang.org/cargo/commands/cargo-vendor.html) 输出 source replacement 配置；
`--frozen` 等价于 Cargo 的 `--locked --offline`。**这不是 OS 网络隔离**：依赖 build.rs、
原生编译器或外部脚本仍能自行联网。必须再做无互联网路由的干净构建。

## 3. 先做可重复构建，再宣称“离线构建”

在工具链已经就绪的 Apple Silicon 构建机上，先在线完成一次候选构建，再使用新的 target
目录和空依赖缓存验证 vendor 输入完整性。候选命令仅针对 **本 fork 的内网默认配置**：

```bash
unset WARP_LOCAL_SKIP_METAL
cargo +1.92.0 --config .cargo/intranet-vendor.toml build \
  --frozen --release -p warp --bin warp-oss --target aarch64-apple-darwin
```

最终发布时以本分支构建脚本固定的参数为准，并存入构建记录。不要加入 `--all-features`、
`cocoa_sentry`、`crash_reporting`、`autoupdate` 或 `standalone`；最后一个是 headless CLI
路径，会改变资源布局。不得回退到未经净化的上游二进制来“修复”构建。

- `warp_assets` 的 release 构建嵌入 `app/assets/bundled` 和 `app/assets/async`。
  debug 构建可能从工作树读文件，因此 debug 启动不证明 app 可独立分发。
- 上游 `script/macos/bundle` 默认是内部 dev、通用架构、签名流程。
  `--channel oss` 虽然去掉 Sentry，仍会加入 GUI/classifier features、资源/skills 和 license
  生成步骤。`--nosign` 不是完整内网净化开关。
- 内网包应明确包含需要的本地资源、许可证、可选 settings schema；不得把上游的 agent skills
  全部搬入。DockTile plug-in 若保留，需要核实自己的 bundle identifier、架构和嵌套签名。
- `.cargo/config.toml` 的 `10.14` 是上游部署配置，不是 Apple Silicon 的可运行版本承诺。
  arm64 的系统起点是 macOS 11，但此 fork 的实际最低版本必须由构建器记录 Mach-O 的
  `LC_BUILD_VERSION`，并在组织实际最老 macOS 上启动测试后再声明。
- 不承诺固定内存/磁盘下限。大工作区加 release/dSYM 可能耗费大量资源；记录峰值、剩余磁盘、
  耗时及错误。在资源紧张 CI 中先采用单并行 job，不能据此保证单个 rustc 进程不会超内存。

离线构建的合格证据：同一源码和输入在隔离 Mac、无原 `target/`、无联网 registry/Git cache
依赖的情况下完成 `--frozen` 构建；保留日志和网络策略记录。仅“第二次缓存命中”不合格。
若失败，记录缺失的具体 crate/tool/asset，在联网准备区补充、重新生成校验清单，再重做隔离验证。

### 本 fork 的打包入口

准备区先安装/生成归属清单；保留工具来源、版本和输出，连同源码转入隔离区：

```bash
cargo +1.92.0 install cargo-about --version 0.8.4 --locked
cargo +1.92.0 about generate --workspace --manifest-path Cargo.toml \
  -c about.toml -o intranet-third-party-licenses.txt about.hbs
```

在已准备的 M 系列构建机上构建候选包：

```bash
unset WARP_LOCAL_SKIP_METAL
export INTRANET_THIRD_PARTY_LICENSES="$PWD/intranet-third-party-licenses.txt"
# 联网构建（只拉取构建所需依赖；不是应用运行时联网）：
script/build-intranet-macos.sh
# 或隔离构建；vendor 与工具链必须已完整准备：
INTRANET_VENDOR_CONFIG=.cargo/intranet-vendor.toml \
  script/build-intranet-macos.sh --offline
```

产物位于新建的 `dist/intranet-<commit>-<random>/`：`Warp Intranet.app`、
`Warp-Intranet-macos-arm64.app.zip`、`BUILD-INFO.txt`、`SHA256SUMS`。
`INTRANET_OUTPUT_DIR` 可改变父目录；默认 ad-hoc 签名，`INTRANET_CODESIGN_IDENTITY`
可指定已获批准并在本机 Keychain 中的身份。指定身份不自动公证，正式分发仍需下面的签名流程。
脚本不附带 DockTile plug-in、外部 agent resources 或 settings schema；嵌入资源由 release
构建提供。若产品后来需要额外本地资源，加入并重新验证，不能靠开发机文件兜底。

脚本追加字体、bash-preexec、Alacritty 等非 Cargo notices，但 `about.toml` 仍有上游的
private/无 license 元数据 crate 排除规则。**生成成功不等于已完成全部许可证核验**：
发布前复核最终依赖与被排除条目，保留对应源码、许可证与修改说明。

## 4. 传入内网的交付包

建议分成两个包，避免最终用户安装 Xcode 或 Rust：

- **运行包**：`Warp Intranet.app` 的 ZIP/DMG、`SHA256SUMS`、版本/架构/签名说明、
  对应源码及许可证获取方式、验收记录。目标 bundle ID 为 `dev.warp.WarpIntranet`，
  可执行文件名保留 `warp-oss`。
- **重建包**：源码（含 LFS 实体）、lockfile、vendor/config、工具链和原生依赖版本/来源/哈希、
  被批准的安装介质清单、构建命令、打包步骤、许可证/第三方声明。

不要用不含 LFS 的 GitHub source ZIP 代替完整重建包。不要夹带 `.git/config` 中的凭据、
Keychain、GitHub token、签名私钥、用户 HOME、真实 shell 历史或生产配置。传输前扫描包清单。
签名身份与私钥由批准的签名机管理，不在源码、artifact 或普通日志里交换。

完成签名、公证/订书和最终封装后才计算最终文件哈希。例如，在只含待交付文件的目录中：

```bash
shasum -a 256 Warp-Intranet-macos-arm64.app.zip warp-intranet-source.tar.gz \
  warp-intranet-build-inputs.tar.gz > SHA256SUMS
# 接收端；逐条必须显示 OK。
shasum -a 256 -c SHA256SUMS
```

清单应通过独立可信渠道确认或由组织签署；与包放在一起的哈希只能校验一致性，不能独立证明来源。
解包后再核查 bundle ID、可执行文件 SHA-256、架构、签名和资源。
打包 helper 自带的清单覆盖候选 ZIP 与 BUILD-INFO；完整重建交付还需对上述源码和输入归档另作清单。

## 5. 签名、Gatekeeper 和真正断网的首次启动

无 Apple 凭据也能编译并生成 ad-hoc 签名测试包；**ad-hoc 签名不是 Developer ID，
不是 Apple 公证，也不保证下载后的首次启动通过 Gatekeeper**。
不要通过关闭 Gatekeeper/SIP、全局删除 quarantine 或跳过组织安全规则来作为交付步骤。

正式分发选择以下之一，且由组织批准：

1. 使用自己的 Developer ID Application 身份，正确签署嵌套代码与 app，启用所需的 hardened
   runtime、时间戳，在联网签名区提交 Apple 公证；收到 Accepted 后 staple 并验证。
   ZIP 不直接承载可 stapling 的 app ticket：先给 app staple，再生成最终 ZIP。
2. 若不能使用 Apple 签名/公证，明确标注“内部测试、ad-hoc/未公证”，由 IT 制定受管分发和
   批准策略；在真实受管 Mac 上验证。不能把一台开发机能运行推断成其他机器会被允许。

[Apple 的公证说明](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)
和[离线 ticket 说明](https://support.apple.com/guide/security/protecting-against-malware-sec469d47bd8/web)
是分发流程依据。公证本身需要联网；stapled ticket 为离线核验准备，但最终仍要验证组织政策与
目标系统的实际行为。macOS 自身的安全检查流量不能误记为 Warp 遥测。

## 6. 免费 GitHub macOS arm64 CI：可以尝试，不保证成功

截至调研日，[GitHub 标准 runner 文档](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
列出 `macos-15` / `macos-26` 的 arm64 标准 runner：3 核 M1、7 GB RAM、14 GB SSD。
公开仓库的标准 runner 免费；私有仓库消耗计划分钟数，超出可能收费。不要误用收费 larger runner，
也不要把“免费”扩展为所有 artifact/LFS 用量无限。

[macos-15 镜像清单](https://github.com/actions/runner-images/blob/main/images/macos/macos-15-arm64-Readme.md)
有完整 Xcode、Git LFS、Rustup、Homebrew 等；镜像自带 Rust 并非本项目固定的 1.92.0。
Metal/protoc 是否就绪必须在实际 job 中检测，缺失时在联网 job 准备。

建议独立、手动触发的最小 workflow：只读 repo 权限 → 固定 action SHA 与源码提交 → checkout LFS →
记录 runner/Xcode/SDK/工具版本 → Metal 真实编译 → fixed lockfile 构建 → 内网 app 打包 →
静态签名/架构/资源检查 → 上传测试 artifact 和校验清单。无需 Warp 账号、GCP 或 Apple 分发凭据。
不要运行继承的发布/同步/云代理 workflow，不要在此步骤发布 Release 或泄露用户配置。

这是**可行候选路线，而非已跑通结论**：上游 `ci.yml` 使用 Namespace mac profile，release 配置
也记录过 OOM；标准 runner 的资源不一定能完成本工作区。若 OOM、磁盘不足、超时，保留失败日志，
转到已批准的自有 M 系列构建机；不要无授权升级付费 runner。
CI 成功也不能替代真实 M 系列 Mac 上 Metal/UI、完全断网首次启动及 shell 内网通信验收。

本 fork 已提供 `.github/workflows/intranet-macos.yml`，但**存在 workflow 文件不表示已经执行或成功**。
它只接受手动触发，限定公开仓库、标准 `macos-15`，不自动发布。
当前为降低资源压力使用 `jobs=1`、`opt-level=1`、`debug=0`、`lto=false`、
`codegen-units=256`，因此产物是低优化验证候选，不能作为普通 release 性能结论。
它不 hydrate LFS classifier 模型，因为默认不启用相关模型 feature；这是有意省略，
不能把该 job checkout 称为完整源码重建包。添加模型或其他 LFS 输入时必须恢复相应下载/校验。

## 上游实现依据（固定基线）

- [Rust 版本](https://github.com/warpdotdev/warp/blob/65229eb524f978cd006c54424fad3a98a35c39c7/rust-toolchain.toml)
- [macOS bootstrap](https://github.com/warpdotdev/warp/blob/65229eb524f978cd006c54424fad3a98a35c39c7/script/macos/bootstrap)
  与 [Metal 安装](https://github.com/warpdotdev/warp/blob/65229eb524f978cd006c54424fad3a98a35c39c7/script/macos/install_build_deps)
- [Metal/ObjC 构建](https://github.com/warpdotdev/warp/blob/65229eb524f978cd006c54424fad3a98a35c39c7/crates/warpui/build.rs)
  与 [app build.rs](https://github.com/warpdotdev/warp/blob/65229eb524f978cd006c54424fad3a98a35c39c7/app/build.rs)
- [资源嵌入](https://github.com/warpdotdev/warp/blob/65229eb524f978cd006c54424fad3a98a35c39c7/crates/warp_assets/src/lib.rs)、
  [资源打包](https://github.com/warpdotdev/warp/blob/65229eb524f978cd006c54424fad3a98a35c39c7/script/prepare_bundled_resources)、
  [macOS bundle](https://github.com/warpdotdev/warp/blob/65229eb524f978cd006c54424fad3a98a35c39c7/script/macos/bundle)

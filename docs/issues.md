# Themis 重要问题、修改与 Trade-off

本文档记录 Release / 打包 / 桌面壳 / 工具栏等**已踩过的坑**、**当前做法**和**刻意接受的取舍**。发版或改 CI 前建议先读一遍。

相关脚本与文档：

| 主题 | 关键路径 |
|------|----------|
| 本地验包 | `scripts/build-release.sh` / `build-release.ps1` → `release-assets/` |
| 收集 + ZIP | `scripts/package-release-assets.sh` / `.ps1` |
| macOS 打补丁 | `scripts/stage-release-bundle.sh` |
| CI | `.github/workflows/release.yml` |
| ZIP 内 README 模板 | `packaging/release-assets-readme-*.md` |
| Release 页说明 | `packaging/RELEASE-INDEX.md` |

---

## 1. GitHub Release 资产：从平铺改为按平台 ZIP

### 问题

旧 Release 页把 `macos-aarch64-themis-tray`、`windows-x86_64-*` 等 **20+ 个文件平铺**在同一页，用户难以判断「该下哪些、哪些可选」。

GitHub Releases **不支持上传目录**，只能上传单个文件。

### 做法

- `package-release-assets` 在收集完平台文件后，再打 **`Themis-<platform>.zip`**
- ZIP 内是一层人类可读的文件夹名，例如 `Themis-macOS-Apple-Silicon/`
- 文件夹内用**无前缀**文件名：`themis-tray`、`themis-service`、`README.md` 等
- CI `release` job **只上传** `Themis-*.zip` + `README.md`（`RELEASE-INDEX.md` 副本），不再平铺所有二进制

### Trade-off

| 取舍 | 说明 |
|------|------|
| ✅ 下载体验 | 每平台只下一个 ZIP，结构清晰 |
| ⚠️ 多解压一步 | 用户需 unzip，不能直接点单个 exe（Windows 仍可在解压后双击） |
| ⚠️ 历史 Release | `v0.5.3` 及更早仍是旧平铺格式；新 tag 起才生效 |
| ⚠️ 本地目录 | 本地还会生成 `release-assets/macos-aarch64/` 中间目录，易与 ZIP 内文件夹混淆——**以 ZIP 解压结果为准** |

### 本地验证

```bash
./scripts/build-release.sh
ls release-assets/macos-aarch64/          # 与 ZIP 内文件一致
unzip -l release-assets/Themis-macos-aarch64.zip
```

---

## 2. Release 包与 `themis.sh tray` 开发版 UI 不一致

### 问题

仅 `cargo build -p themis-tray` 得到的 exe **不会嵌入最新 Vite 前端**；菜单栏/浮层可能是旧 UI（缺按钮、布局不对）。用户反馈「Release 和 dev 不一样」多源于此。

### 做法

- Release **必须**走 `npm run tauri build`（`beforeBuildCommand: npm run build`）
- CI / 本地脚本在 tauri build 前 **`cargo clean -p themis-tray`**，避免增量编译留下旧资源
- 配置合并使用**文件路径**：`--config scripts/tauri-release-build.json`（见 §4）

### Trade-off

| 取舍 | 说明 |
|------|------|
| ✅ 与 dev 一致 | 浮层 UI 与 `./scripts/themis.sh tray` 同源（release 为 embedded dist） |
| ⚠️ 构建更慢 | 每次 release 全量编 tray + 前端；`--skip-installer` 可省 dmg/NSIS 但仍需 tauri build |
| ⚠️ dev 热更新 | 开发用 `tauri dev` / Vite HMR；Release exe 无热更新，需重新打包 |

---

## 3. macOS 便携版找不到 `themis-service`

### 问题

Tauri 默认只打 `Themis.app`，**不会**把 `themis-service` 放进 `.app` 或便携目录；tray 启动 service 失败。

### 做法

- `stage-release-bundle.sh`：把 `themis-service`、`themis-cli` **复制进** `Themis.app/Contents/MacOS/`
- 便携包里的 `themis-tray` **优先从 .app 内复制**（保证与安装版同源）
- `find_service_binary()` 增加 `.app/Contents/MacOS` 及同目录候选路径

### Trade-off

| 取舍 | 说明 |
|------|------|
| ✅ 便携版可用 | 三个二进制 + README 同目录即可跑 |
| ⚠️ 非标准 .app 布局 | Apple 期望 .app 内只有一个主 executable；我们故意多塞 sidecar |
| ⚠️ Windows | NSIS 安装包与便携目录逻辑不同，需各自验证 |

---

## 4. Windows CI：`tauri build --config` 解析失败

### 问题

Workflow 曾用 `--config "$(cat tauri-release-build.json)"`。在 Windows runner 上 JSON 被 shell 拆成多参数，Tauri 只收到 `{`，报错 `failed to parse config '{' as JSON`。

### 做法

改为传**配置文件路径**：

```bash
npm run tauri build -- ... --config ../../scripts/tauri-release-build.json
```

与 `build-release.ps1` 一致。

### Trade-off

| 取舍 | 说明 |
|------|------|
| ✅ 跨平台稳定 | bash / pwsh 均可用 |
| ⚠️ 合并配置 | 仅覆盖 `build.beforeBuildCommand`；其余仍以 `tauri.conf.json` 为准 |

---

## 5. macOS Gatekeeper：「Themis.app 已损坏」

### 问题

从 GitHub / Safari 下载的 `.dmg` / `.app` 带 **quarantine** 属性；未 **Apple 公证（notarization）** 的应用会被 Gatekeeper 拦截，提示「已损坏，无法打开」——**不是文件损坏**。

### 做法（当前）

- Release 流水线：**ad-hoc 签名**（`codesign --sign -`）+ dmg 内附 `请先阅读-安装说明.txt`
- 文档与脚本：`xattr -cr`、`fix-macos-app-quarantine.sh`
- **推荐用户用便携版**（ZIP 内 `themis-tray` + `xattr -cr .`），避开 `.app` 安装路径

### Trade-off

| 取舍 | 说明 |
|------|------|
| ✅ 零 Apple 开发者成本 | 无需付费账号 + notarize 流水线 |
| ❌ 安装体验 | `.dmg` 路径对非技术用户不友好 |
| 🔮 若要做公证 | 需 Apple Developer ID、CI 密钥链、`notarytool`、 stapler；成本高但可根治 |

---

## 6. ZIP 内 README 与用户所见不一致

### 问题

- README 模板曾写「本地 `build-release.sh` 生成」「去 Release 页单独下 tray + service」——与 **GitHub ZIP 一包齐全** 不符
- `.env.example` 在 macOS Finder 中**默认隐藏**（点开头的文件），用户以为包内缺少配置模板

### 做法

- 重写 `packaging/release-assets-readme-macos.md` / `-windows.md`：面向 **GitHub ZIP 解压后的文件夹**
- 打包时同时复制 **`env.example`**（可见文件名）与 **`.env.example`**
- Release 总览：`packaging/RELEASE-INDEX.md`

### Trade-off

| 取舍 | 说明 |
|------|------|
| ✅ 文档与包一致 | 改模板 → 重新 `package-release-assets` 或完整 build |
| ⚠️ 双份 env 模板 | `env.example` 与 `.env.example` 内容相同，略冗余 |
| ⚠️ 已下载旧包 | 旧 ZIP 内 README 仍可能是旧文案，需等新 Release |

---

## 7. DMG 文件名版本与 Git tag 不一致

### 问题

Release tag 可能是 `v0.5.x`，但 dmg 仍名如 `Themis_0.1.0_aarch64.dmg`——来自 `tauri.conf.json` 的 `"version": "0.1.0"`，**未与 git tag 同步**。

### 现状

已知问题，**不影响运行**；仅文件名/关于对话框版本号可能误导。

### 可选后续

发 tag 前用脚本把 `tauri.conf.json` version 与 tag 对齐，或 CI 里 sed 替换。未做是为了避免每次发版改多处配置。

---

## 8. 浮层工具栏：两行 + `⋯` 溢出菜单

### 问题

窗口变窄时工具栏按钮被裁切；曾出现「还有空位却显示 `⋯`」「点 `⋯` 无反应」「菜单对比度差」「tooltip 挡住菜单」等。

### 做法

- 最多 **2 行**布局；空间不足时按优先级移入 **`⋯` 菜单**
- **永不溢出**：捕捉、字幕/导出、清空、隐藏、浮、钉子、尺寸条等
- **溢出顺序**（先移走 = 优先级低）：scroll-latest → 字/透/中文 → 退出 → 配置 → 诊断
- 极窄窗口 **DESPERATE_OVERFLOW**：清空、导出等最后兜底
- 菜单 **`position: fixed` + 挂到 `document.body`**，避免被 `overflow: hidden` 裁切
- `⋯` 上不显示 tooltip；菜单内项保留 tooltip（靠左显示）

### Trade-off

| 取舍 | 说明 |
|------|------|
| ✅ 窄窗可用 | 核心操作始终可见 |
| ⚠️ 复杂度 | 独立 `toolbar-overflow.js` + 与 `tooltips.js` / drag 区域协作 |
| ⚠️ 极端窄窗 | 仍可能把部分次要按钮收进菜单，需用户点 `⋯` |

---

## 9. macOS 浮层：系统标题栏与拖拽

### 问题

Release 构建出现**意外系统标题栏**；或整片 header 不可拖拽窗口。

### 做法

- `tauri.conf.json`：`titleBarStyle: "Overlay"`
- `macos_window.rs`：`ensure_overlay_frameless()` 隐藏标题栏
- **拖拽**：brand / spacer 等区域可拖；**不要**给整个 `#drag-handle` 绑 `data-tip`（曾导致拖拽失效）
- `main.js` 里 `-webkit-app-region: drag` 排除范围收窄，不含整个 `.header-toolbar`

### Trade-off

| 取舍 | 说明 |
|------|------|
| ✅ 无边框浮层 | 与产品设计一致 |
| ⚠️ 平台差异 | Windows 无 titleBarStyle；行为以 macOS 为主验证 |
| ⚠️ 可点区域 | 按钮密集处必须显式 `no-drag`，否则难拖窗 |

---

## 10. CI Release 与本地打包对齐清单

发 tag 前建议本地确认：

1. **`./scripts/build-release.sh`**（或 Windows 等价脚本）成功
2. **`release-assets/Themis-*.zip`** 解压后文件夹名、文件列表正确
3. **`README.md`** 内容与 `packaging/release-assets-readme-*.md` 一致
4. 存在 **`env.example`** 与 **`.env.example`**
5. 便携启动：`chmod +x` + `xattr -cr .` + `./themis-tray`（macOS）
6. **`themis-cli doctor`** 通过（配置好 `.env` 后）
7. 浮层 UI 与 dev 一致（确认走了 `tauri build` 而非仅 `cargo build -p themis-tray`）

CI 成功标志：三个 matrix job（Windows + 两个 macOS）绿 → `release` job 上传 3 个 ZIP + README。

---

## 11. 刻意未做 / 后续可考虑

| 项 | 原因 |
|----|------|
| Apple **notarization** | 需开发者账号与 CI 密钥；当前用文档 + 便携版缓解 |
| **自动同步** tauri version 与 git tag | 低优先级；dmg 文件名 cosmetic |
| Release 同时保留平铺 + ZIP | 避免 Release 页再次变乱；只保留 ZIP |
| 单文件「全平台」安装包 | 体积与签名策略不同；维持分平台 ZIP |

---

*最后更新：与 Release ZIP 重组、Windows `--config` 修复、README 对齐、工具栏 overflow 等改动同步。*

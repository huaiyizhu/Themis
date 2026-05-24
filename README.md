# Themis

**项目名称：** Themis（忒弥斯）

实时捕获 **系统音频输出**（电脑正在播放的声音），经本地 `themis-service` 送入 **Azure Speech** 做流式听写，由托盘浮层 `themis-tray` 展示字幕。不是麦克风录音；与物理扬声器、系统音量条无直接关系。

---

## 工作原理

```
┌─────────────────┐     WASAPI loopback      ┌──────────────────┐
│ 系统正在播放的   │  (默认播放设备上的混音)   │  themis-service  │
│ 应用音频输出     │ ───────────────────────► │  重采样 16 kHz   │
└─────────────────┘                         │  Azure STT       │
                                            └────────┬─────────┘
                                                     │ gRPC
                                            ┌────────▼─────────┐
                                            │  themis-tray     │
                                            │  浮层 + 热键     │
                                            └──────────────────┘
```

| 组件 | 作用 |
|------|------|
| `themis-service` | 后台：抓系统输出 → 识别 → gRPC 推送字幕 |
| `themis-tray` | 托盘图标、浮层 UI、`Ctrl+Shift+T` / `Ctrl+Shift+D` 快捷键 |
| `themis-cli` | 安装服务、诊断、`status` / `doctor` |

更细的架构见 [docs/architecture.md](docs/architecture.md)，平台差异见 [docs/platform-notes.md](docs/platform-notes.md)。

---

## 环境要求

| 项目 | 版本 |
|------|------|
| Windows | 10+（当前主要开发平台） |
| macOS | 12+（托盘可用；系统音频需虚拟声卡，见平台说明） |
| Rust | stable（见 `rust-toolchain.toml`） |
| Node.js | 20+（仅托盘 Tauri 前端） |

---

## 第一次使用

### 1. Azure Speech 资源

1. 在 [Azure Portal](https://portal.azure.com/#create/Microsoft.CognitiveServicesSpeechServices) 创建 **Speech** 资源。
2. 记下 **Key** 与 **Region**（如 `eastus`）。
3. 复制环境变量模板并填写：

```powershell
copy .env.example .env
# 编辑 .env，填入 AZURE_SPEECH_KEY、AZURE_SPEECH_REGION
```

未配置 Key 时会自动进入 **Mock 识别**（仅用于 UI 联调，无真实听写）。

### 2. 一键脚本（推荐）

在项目根目录用 PowerShell，或双击根目录下的 **`dev.cmd` / `restart.cmd` / `tray.cmd`**（内部调用同一套脚本）：

| 命令 | 作用 |
|------|------|
| `.\scripts\themis.ps1 dev` | **编译 + 后台启动** `themis-service`（日常最常用） |
| `.\scripts\themis.ps1 restart` | **停止 → 重新编译 → 再启动**（改完 Rust 代码或 `.env` 后用） |
| `.\scripts\themis.ps1 tray` | 编译并启动服务 + 启动托盘 UI（单窗口开发） |
| `.\scripts\themis.ps1 stop` | 停止后台服务 |
| `.\scripts\themis.ps1 status` | 查看进程是否在跑、二进制是否已编译 |
| `.\scripts\themis.ps1 doctor` | 运行 `themis-cli doctor`（检查 Azure 连通性等） |
| `.\scripts\themis.ps1 probe` | **音频采集自检**（8 秒，不依赖 Azure，见下文） |
| `.\scripts\themis.ps1 build` | 仅编译服务 |
| `.\scripts\themis.ps1 build -Release` | Release 编译（加 `-Release` 适用于任意子命令） |

macOS / Linux：

```bash
chmod +x scripts/themis.sh
./scripts/themis.sh dev
./scripts/themis.sh restart
./scripts/themis.sh tray
```

**典型流程**

```powershell
# 1) 首次或改 .env / Rust 后
.\scripts\themis.ps1 restart

# 2) 只开后台服务
.\scripts\themis.ps1 dev

# 3) 再开托盘（也可只跑 tray，会自动尝试拉起已编译的服务）
.\scripts\themis.ps1 tray
```

服务在**后台无窗口**运行；日志在 `%LOCALAPPDATA%\Themis\logs`（Windows），不在那个黑窗口里。

### 3. 手动运行（两个终端）

```powershell
# 终端 1 — 后台服务（保持运行）
cargo run -p themis-service

# 终端 2 — 托盘
cd apps\themis-tray
npm install
npm run tauri dev
```

浮层应显示 `Status: idle — …`。若只有 **Service offline**，说明服务未启动或未监听 gRPC。

---

## 使用说明

| 操作 | Windows | macOS |
|------|---------|-------|
| 开始/停止采集 | `Ctrl+Shift+T` | `Cmd+Shift+T` |
| 延迟诊断窗口 | `Ctrl+Shift+D` | `Cmd+Shift+D` |
| 浮层透明度 − / + | `Ctrl+Shift+[` / `]` | `Cmd+Shift+[` / `]` |
| 切换浮层风格 | `Ctrl+Shift+S` | `Cmd+Shift+S` |
| 背景自适应对比 | `Ctrl+Shift+A` | `Cmd+Shift+A` |
| 退出托盘应用 | `Ctrl+Shift+Q` | `Cmd+Shift+Q` |
| 显示/隐藏浮层 | 左键托盘图标 | 同左 |
| 移动浮层 | 拖动标题栏 | 同左 |
| 调整大小 | 拖动窗口边缘/角 | 同左 |

浮层**始终置顶**。风格预设：`dark-glass`、`light-glass`、`high-contrast-dark`、`high-contrast-light`、`outline`。**自适应**（`Ctrl+Shift+A`）会采样浮层下方的桌面亮度，自动在深浅面板间切换（Windows）。

**Insights 侧栏**：每句最终转写后会提取**关键词**、**术语解释**（如 RAG、NBA）和**问题初步回答**；右侧显示。未配置 LLM 时使用内置启发式；配置 `FOUNDRY_*` 后使用 Azure OpenAI 增强。

**诊断窗口**会显示当前浮层文字、最近短语的延迟分解（**Buffer** ≈ REST 分块累积时长、**Azure** = 网络 + 识别、**STT wall** = 多语言并行时的墙钟时间、**E2E est.** ≈ 从语音结束到文字就绪的估计、**UI** = 服务发出到浮层显示的间隔）。托盘菜单也可选 **Diagnostics**。

开始采集后，字幕在浮层中**逐句累积**（最终结果追加，过程中显示灰色 partial）。

---

## 配置说明（`.env`）

完整模板见 [.env.example](.env.example)。

| 变量 | 必填 | 说明 |
|------|------|------|
| `AZURE_SPEECH_KEY` | 是* | Speech 资源密钥 |
| `AZURE_SPEECH_REGION` | 是* | 区域，如 `eastus` |
| `AZURE_SPEECH_LANGUAGE` | 建议 | **`auto`**（默认，中英自动）\| `en-US` \| `zh-CN` \| `en-US,zh-CN` |
| `AZURE_SPEECH_MODE` | 否 | `streaming`（整句流式）或 `rest`（默认，**2 秒**分块） |
| `THEMIS_AUDIO_CAPTURE_MODE` | 否 | **Windows**：`auto`（默认，优先进程 loopback）\| `process` \| `endpoint` |
| `THEMIS_AUDIO_OUTPUT_DEVICE` | 否 | **仅 endpoint 模式**：播放设备友好名子串或 endpoint ID |
| `THEMIS_AUDIO_GAIN_MAX` | 否 | 自动增益上限，默认 `16` |
| `THEMIS_GRPC_PORT` | 否 | 默认 `50051` |
| `THEMIS_LOG_LEVEL` | 否 | 默认 `info` |
| `THEMIS_USE_MOCK_SPEECH` | 否 | `true` 强制 Mock，不连 Azure |

\* 缺 Key/Region 时自动 Mock。

修改 `.env` 后请执行：

```powershell
.\scripts\themis.ps1 restart
```

---

## 音频采集（需求 1）— Windows

目标：**只要系统里有应用在播放声音**（YouTube、会议、音乐等），Themis 就要能抓到**数字音频**，用于后续转写。与是否插耳机、物理扬声器无关；并尽量在**系统静音 / 音量很低**时仍能采到。

### 两种采集方式

| 模式 | 环境变量 | 行为 |
|------|----------|------|
| **进程 loopback**（默认） | `THEMIS_AUDIO_CAPTURE_MODE=auto` 或 `process` | 枚举正在播放音频的应用（Chrome、Edge 等），对每个会话做 **Application Loopback**，在多数机器上**不受主音量静音影响** |
| **端点 loopback**（备用） | `endpoint` | 抓取默认播放设备上的混音；部分声卡驱动在**静音时会变成全零**，不推荐单独使用 |

`auto`：有活跃音频会话 → 用 **process**；否则回退 **endpoint**。

### 采集自检（请先跑这个）

播放 YouTube 或任意声音（可故意把 Windows **静音** 做验证），在项目根目录：

```powershell
.\scripts\themis.ps1 probe
```

或：

```powershell
cargo run -p themis-cli -- audio-probe --seconds 8
```

输出示例（正常）：

```text
mode:     process
sessions: 2
frames:   1000+
peak:     >200  (越大越好，>2000 很强)
OK: capture pipeline is receiving audio.
```

| 结果 | 含义 |
|------|------|
| `OK` + `peak > 200` | 采集正常，可继续接 Azure 转写 |
| `WARN` + `peak < 200` | 有帧但信号弱，转写可能差 |
| `FAIL` + `frames = 0` | 未采到任何数据，检查是否有应用在出声、输出设备路由 |

### 运行时状态（浮层 / gRPC）

开始采集后，**Status 行**会显示诊断信息，例如：

```text
capturing | capture=process sessions=2 peak=12000 frames=800 signal=strong
```

| 字段 | 说明 |
|------|------|
| `capture=process` | 当前使用的采集后端 |
| `sessions` | 检测到的活跃音频会话数 |
| `peak` | 近期 PCM 峰值（0–32767） |
| `frames` | 已收到的音频帧计数 |
| `signal` | `silent` / `quiet` / `ok` / `strong` |

也可用：`cargo run -p themis-cli -- status`（需先 `themis-service` 在跑且已开始 capture）。

### 限制说明（诚实说明）

- **进程 loopback** 需要 Windows 10 20H1+（你当前系统满足）。
- 极少数声卡驱动在静音时，**端点** loopback 仍会全零；`auto` 模式会优先避开这一点。
- 若应用把声音送到**非默认**播放设备，请把 Windows 默认输出改到该设备，或设置 `THEMIS_AUDIO_OUTPUT_DEVICE`（仅 `endpoint` 模式）。

---

## 常见问题

### 浮层显示 Service offline

- 先运行 `.\scripts\themis.ps1 dev` 或 `restart`，或 `.\scripts\themis.ps1 status` 确认进程在跑。
- 检查端口是否与 `.env` 中 `THEMIS_GRPC_PORT` 一致。

### 能采集但几乎没有字幕 / 只有零星单词

先确认采集：`.\scripts\themis.ps1 probe` 必须 `OK` 且 `peak > 200`。

1. 使用 **`AZURE_SPEECH_LANGUAGE=auto`**（默认）自动中英识别；或手动指定单一语言。
2. 确认 `.env` 已保存并 **`.\scripts\themis.ps1 restart`**。
3. 确认 `THEMIS_AUDIO_CAPTURE_MODE=auto`。
4. 尝试 `AZURE_SPEECH_MODE=streaming`（默认）；若 WebSocket 失败可暂用 `rest`。
5. 用 `.\scripts\themis.ps1 doctor` 检查 Azure 密钥与区域。

### 想用 Mock 测 UI

在 `.env` 设置 `THEMIS_USE_MOCK_SPEECH=true`，然后 `restart`。

### 编译时提示无法覆盖 themis-service.exe

服务仍在运行。先 `.\scripts\themis.ps1 stop` 再 `build` / `restart`。

---

## 从源码完整构建

```bash
# Rust
cargo build --release -p themis-service -p themis-cli

# 托盘
cd apps/themis-tray
npm install
npm run tauri build    # 发布安装包
# 或
npm run tauri dev      # 开发
```

Release 二进制：`target/release/themis-service.exe`（Windows）。

---

## 安装为系统服务（可选）

**Windows（管理员）** — 详见 [packaging/windows/themis-service.md](packaging/windows/themis-service.md)：

```powershell
themis-cli service install
themis-cli service start
```

**macOS LaunchAgent：**

```bash
themis-cli agent install
themis-cli agent start
```

日常开发用 `scripts/themis.ps1` 即可，无需管理员。

---

## 仓库结构

```
crates/themis-core      # 配置、状态机、音频帧
crates/themis-audio     # 系统音频输出采集（Windows WASAPI）
crates/themis-azure     # Azure Speech（流式 / REST / Mock）
crates/themis-ipc       # gRPC
crates/themis-service   # 后台服务入口
crates/themis-cli       # CLI / 服务安装
apps/themis-tray        # Tauri 托盘 + 浮层
scripts/themis.ps1      # Windows 一键脚本
scripts/themis.sh       # macOS/Linux 一键脚本
docs/                   # 架构与平台说明
packaging/              # 服务安装模板
```

---

## 诊断命令

```bash
cargo run -p themis-cli -- doctor
cargo run -p themis-cli -- status
# 或
.\scripts\themis.ps1 doctor
```

---

## iOS 路线图

v0.1 **不包含** iOS 系统内录；受平台限制，后续可能通过 ReplayKit 等方案。见 [docs/platform-notes.md](docs/platform-notes.md)。

---

## License

MIT

# Themis

**项目名称：** Themis（忒弥斯）

实时捕获 **系统音频输出**（电脑正在播放的声音），经本地 `themis-service` 送入 **Azure Speech** 做流式听写，由托盘浮层 `themis-tray` 展示字幕。不是麦克风录音；与物理扬声器、系统音量条无直接关系。

---

## 工作原理

```
┌─────────────────┐     WASAPI loopback      ┌──────────────────────────────┐
│ 系统正在播放的   │  (进程/端点 loopback)     │  themis-service              │
│ 应用音频输出     │ ───────────────────────► │  重采样 16 kHz → Azure STT   │
└─────────────────┘                         │  → Insights 分析 (可选 LLM)  │
                                            └────────┬─────────────────────┘
                                                     │ gRPC (字幕 + insights_json)
                                            ┌────────▼─────────┐
                                            │  themis-tray     │
                                            │  字幕 + 侧栏洞察  │
                                            └──────────────────┘
```

| 组件 | 作用 |
|------|------|
| `themis-service` | 后台：抓系统输出 → 听写 → **分析关键词/术语/问题** → gRPC 推送 |
| `themis-analysis` | 启发式词表 + 可选 Azure OpenAI，生成结构化 Insights |
| `themis-tray` | 托盘图标、浮层 UI（左侧字幕 + 右侧 Insights 侧栏） |
| `themis-cli` | 安装服务、诊断、`status` / `doctor` |

更细的架构见 [docs/architecture.md](docs/architecture.md)，平台差异见 [docs/platform-notes.md](docs/platform-notes.md)。

---

## 环境要求

| 项目 | 版本 |
|------|------|
| Windows | 10+ |
| macOS | 14.2+（推荐 Process Tap 抓系统播放声；见下文） |
| Rust | stable（见 `rust-toolchain.toml`） |
| Node.js | 20+（仅托盘 Tauri 前端） |

**macOS 额外依赖**

| 工具 | 用途 |
|------|------|
| [Xcode Command Line Tools](https://developer.apple.com/xcode/) | `xcode-select --install` — Tauri / 本地链接 |
| [Homebrew](https://brew.sh/)（推荐） | 安装 Node 20、`rustup` 等 |
| [BlackHole](https://existential.audio/blackhole/)（可选） | 仅在使用 `THEMIS_AUDIO_CAPTURE_MODE=input` 且需虚拟环回时 |

---

## 第一次使用

### 1. Azure Speech 资源

1. 在 [Azure Portal](https://portal.azure.com/#create/Microsoft.CognitiveServicesSpeechServices) 创建 **Speech** 资源。
2. 记下 **Key** 与 **Region**（如 `eastus`）。
3. 复制环境变量模板并填写：

```powershell
# Windows
copy .env.example .env
```

```bash
# macOS / Linux
cp .env.example .env
```

编辑 `.env`，填入 `AZURE_SPEECH_KEY`、`AZURE_SPEECH_REGION`。

未配置 Key 时会自动进入 **Mock 识别**（仅用于 UI 联调，无真实听写）。

### 2. 一键脚本（推荐）

#### Windows

在项目根目录用 PowerShell，或双击 **`dev.cmd` / `restart.cmd` / `tray.cmd`**：

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

```powershell
# 典型流程
.\scripts\themis.ps1 restart   # 改 .env / Rust 后
.\scripts\themis.ps1 dev       # 只开后台服务
.\scripts\themis.ps1 tray      # 服务 + 托盘（会自动编译并拉起服务）
```

日志：`%LOCALAPPDATA%\Themis\logs`

#### macOS（MacBook）

默认使用 **Process Tap**（无需 BlackHole）。详见 [docs/platform-notes.md](docs/platform-notes.md)。

```bash
chmod +x scripts/themis.sh dev.sh restart.sh tray.sh
./scripts/themis.sh dev      # 或 ./dev.sh
./scripts/themis.sh restart  # 或 ./restart.sh
./scripts/themis.sh tray     # 或 ./tray.sh — 首次会自动生成 Tauri 所需的 icon.icns
```

| 命令 | 作用 |
|------|------|
| `./scripts/themis.sh dev` / `./dev.sh` | 编译 + 后台启动服务 |
| `./scripts/themis.sh restart` / `./restart.sh` | 停止 → 编译 → 启动 |
| `./scripts/themis.sh tray` / `./tray.sh` | 服务 + Tauri 托盘（前台） |
| `./scripts/themis.sh stop` | 停止 `themis-service` |
| `./scripts/themis.sh status` | 进程与 `.env` 状态 |
| `./scripts/themis.sh doctor` | Azure / gRPC 自检 |
| `./scripts/themis.sh probe` | 音频采集自检（8 秒） |
| `./scripts/themis.sh icons` | 仅生成 `icon.icns`（`tray` 会自动调用） |
| `./scripts/themis.sh build -r` | Release 编译（`-r` / `--release`） |

日志：`~/Library/Logs/Themis`  
数据：`~/Library/Application Support/Themis`

macOS 默认 `THEMIS_AUDIO_CAPTURE_MODE=auto` 使用 **Process Tap** 采集系统播放；检测到 Zoom/Teams 等通话 app 时会**同时采集麦克风**。`THEMIS_AUDIO_OUTPUT_DEVICE` 仅 Windows 有效。

### 3. 手动运行（两个终端）

**Windows**

```powershell
cargo run -p themis-service
cd apps\themis-tray
npm install
npm run tauri dev
```

**macOS**

```bash
cargo run -p themis-service
cd apps/themis-tray
npm install
npm run tauri dev   # 若缺 icon.icns：./scripts/themis.sh icons
```

浮层应显示 `Status: idle — …`。若只有 **Service offline**，说明服务未启动或未监听 gRPC。

---

## 使用说明

| 操作 | Windows | macOS |
|------|---------|-------|
| 开始/停止采集 | `Ctrl+Shift+T` | `Cmd+Shift+T` |
| **唤醒/置顶浮层（居中⅓屏）** | `Ctrl+Shift+O` | `Cmd+Shift+O` |
| 延迟诊断窗口 | `Ctrl+Shift+D` | `Cmd+Shift+D` |
| 浮层透明度 − / + | `Ctrl+Shift+[` / `]` | `Cmd+Shift+[` / `]` |
| 切换浮层风格 | `Ctrl+Shift+S` | `Cmd+Shift+S` |
| 背景自适应对比 | `Ctrl+Shift+A` | `Cmd+Shift+A` |
| 显示/隐藏字幕区 | `Ctrl+Shift+H` | `Cmd+Shift+H` |
| 迷你浮标模式 | `Ctrl+Shift+M` | `Cmd+Shift+M` |
| 退出托盘应用 | `Ctrl+Shift+Q` | `Cmd+Shift+Q` |
| 显示/隐藏浮层 | 左键托盘图标 | 同左 |
| 移动浮层 | 拖动标题栏 | 同左 |
| 调整大小 | 拖动窗口边缘/角 | 同左 |

浮层**始终置顶**。风格预设：`dark-glass`、`light-glass`、`high-contrast-dark`、`high-contrast-light`、`outline`。**自适应**（`Ctrl+Shift+A` / `Cmd+Shift+A`）会采样浮层下方的桌面亮度并自动切换深浅面板（**仅 Windows**；macOS 上快捷键存在但暂不采样桌面）。

**Insights 侧栏**（浮层右侧）：对**每一句最终听写结果**做关键词、术语解释与问题初答，详见下文 [Insights 洞察](#insights-洞察关键词--术语--问答)。**诊断窗口**（`Ctrl+Shift+D`）可查看 STT 延迟与启发式/LLM 分析拆分，见 [延迟诊断](#延迟诊断)。

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
| `THEMIS_STT_FIXUP` | 否 | 默认 `true`：STT 后术语纠错（如 Reg→RAG） |
| `AZURE_SPEECH_CORRECTIONS` | 否 | 额外纠错对，如 `Reg:RAG,某词:正确词` |
| `THEMIS_AUDIO_CAPTURE_MODE` | 否 | **Windows**：`auto`（默认；检测到通话 app 时自动双路）\| `process` \| `endpoint` \| `call` \| `dual` |
| | | **macOS**：`auto`（默认；检测到通话 app 时 process tap + 麦克风）\| `process_tap` \| `input` \| `call` \| `dual` |
| `THEMIS_AUDIO_INPUT_DEVICE` | 否 | **双路/输入模式**：麦克风设备名子串（如 `Jabra`）；macOS 在 `input` 或 dual 时生效 |
| `THEMIS_AUDIO_OUTPUT_DEVICE` | 否 | **仅 endpoint 模式**：播放设备友好名子串或 endpoint ID |
| `THEMIS_AUDIO_GAIN_MAX` | 否 | 自动增益上限，默认 `16` |
| `THEMIS_GRPC_PORT` | 否 | 默认 `50051` |
| `THEMIS_LOG_LEVEL` | 否 | 默认 `info` |
| `THEMIS_USE_MOCK_SPEECH` | 否 | `true` 强制 Mock，不连 Azure |
| `THEMIS_ANALYSIS_ENABLED` | 否 | 默认 `true`；`false` 关闭 Insights 分析 |
| `FOUNDRY_ENDPOINT` | 否 | Azure OpenAI 终结点（增强术语/问答，见 Insights 章节） |
| `FOUNDRY_API_KEY` | 否 | Azure OpenAI API Key |
| `FOUNDRY_DEPLOYMENT` | 否 | 部署名，默认 `gpt-4o-mini` |

\* 缺 Key/Region 时自动 Mock。Insights **不依赖** Foundry：未配置时仍可用内置启发式与词表。

修改 `.env` 后请执行 `.\scripts\themis.ps1 restart`（Windows）或 `./scripts/themis.sh restart`（macOS）。

---

## Insights 洞察（关键词 / 术语 / 问答）

### 做什么、不做什么

| 能力 | 说明 |
|------|------|
| ✅ | 从**听写文本**里抽关键词、技术术语、问句，并在浮层**右侧**给出简短解释或初答 |
| ✅ | 内置词表覆盖常见缩写与 AI 词（RAG、embedding、GPU…），**无需大模型**即可用 |
| ✅ | 可选接入 **Azure OpenAI**，对词表外内容、复杂问句做更强解释 |
| ❌ | **不识别画面字幕**（不做 OCR）；视频里只显示文字、没有旁白时，Insights 不会出现该字幕 |

### 端到端机理

```
Azure STT 输出 is_final 句子
        │
        ▼
┌───────────────────┐     先推送 gRPC（仅 text，insights 为空）
│ themis-service    │ ──► 浮层立刻显示该句字幕
│ CaptureEngine     │
└─────────┬─────────┘
          │ 异步 analyze(text)
          ▼
┌───────────────────┐
│ themis-analysis   │
│ ① 启发式 (必跑)   │  词表 + 正则问句 + 英文词扫描
│ ② LLM (可选)    │  若配置 FOUNDRY_*，合并 JSON 结果
└─────────┬─────────┘
          │ 再推送 gRPC（同一句 text + insights_json）
          ▼
     浮层右侧 Insights 更新；句末可带关键词小标签
```

相关代码：

| 路径 | 职责 |
|------|------|
| [`crates/themis-service/src/engine.rs`](crates/themis-service/src/engine.rs) | 每句 `is_final` 先出字幕后调 `create_analyzer()` |
| [`crates/themis-analysis/src/heuristic.rs`](crates/themis-analysis/src/heuristic.rs) | 规则引擎：问句、英文词、查词表 |
| [`crates/themis-analysis/src/glossary.rs`](crates/themis-analysis/src/glossary.rs) | **内置术语表**（可编辑） |
| [`crates/themis-analysis/src/llm.rs`](crates/themis-analysis/src/llm.rs) | Azure OpenAI Chat Completions（JSON 输出） |
| [`crates/themis-ipc/proto/themis.proto`](crates/themis-ipc/proto/themis.proto) | 字段 `insights_json` |
| [`apps/themis-tray/main.js`](apps/themis-tray/main.js) | 解析 JSON，渲染 Keywords / Terms / Questions |

### 两层分析：启发式 vs 大模型

| 层级 | 何时启用 | 成本 | 典型能力 |
|------|----------|------|----------|
| **启发式** | 默认始终开启（`THEMIS_ANALYSIS_ENABLED=true`） | 无 API 费用 | 词表术语、大写缩写、英文技术词作 Keywords；识别问句并给模板化初答 |
| **Azure OpenAI** | `.env` 配置 `FOUNDRY_ENDPOINT` + `FOUNDRY_API_KEY` + `FOUNDRY_DEPLOYMENT` | 按 token 计费 | 词表外的术语、更长解释、开放域问题的 2–3 句初答 |

合并策略：先跑启发式，再在 **12 秒超时**内等待 LLM；两者结果按「术语 / 关键词 / 问题」去重合并（见 [`factory.rs`](crates/themis-analysis/src/factory.rs)）。

**未连大模型时侧栏为空？** 常见原因曾是：① 词表无该词；② 问句无 `?`/`？`（如「embedding 是什么」）。当前启发式已支持 **「X是什么 / 什么是X / what is X」** 无标点问句，并对 **小写英文技术词** 查表。

### 内置术语表（如何扩展）

文件：**[`crates/themis-analysis/src/glossary.rs`](crates/themis-analysis/src/glossary.rs)**

```rust
// 键：小写、不区分大小写匹配；值：(显示名, 解释)
("embedding", ("embedding", "嵌入向量：把文本映射到高维向量…")),
```

修改后执行 `.\scripts\themis.ps1 restart`（需重新编译 `themis-service`）。

### 启发式规则摘要

1. **词表命中**：句中出现词表键（如 `embedding`、`RAG`）→ 写入 **Terms** + **Keywords**。  
2. **大写缩写**：正则 `\b[A-Z]{2,8}\b`（NBA、API…）→ 查表。  
3. **英文词**：`\b[A-Za-z][A-Za-z0-9_-]{2,}\b` → 查表并列入 Keywords。  
4. **问句**（不要求句末问号）：  
   - `embedding 是什么` / `啥是 RAG`  
   - `什么是 embedding`  
   - `what is RAG`  
   - 以及带 `?` / `？` 的整句  
5. **问题初答**：若问句主语在词表中，直接用词表解释；否则给简短模板句；配置 LLM 后可被更完整回答覆盖。

### 浮层 UI

| 区域 | 内容 |
|------|------|
| 左侧 | 听写正文；最终句可带灰色 **Keywords** 小标签 |
| 右侧 **Insights** | **Keywords** 标签云、**Terms** 卡片（术语 + 解释）、**Questions** 卡片（问题 + 初答） |

默认浮层宽度约 520px，便于并排阅读。

### 配置示例（仅启发式）

无需额外 Key，保持默认即可：

```env
THEMIS_ANALYSIS_ENABLED=true
# 不填 FOUNDRY_* 亦可
```

### 配置示例（启发式 + Azure OpenAI）

在 Azure 门户创建 **Azure OpenAI** 资源，部署聊天模型（如 `gpt-4o-mini`）：

```env
FOUNDRY_ENDPOINT=https://你的资源名.openai.azure.com
FOUNDRY_API_KEY=你的_key
FOUNDRY_DEPLOYMENT=gpt-4o-mini
THEMIS_ANALYSIS_ENABLED=true
```

修改后：`.\scripts\themis.ps1 restart`。

### 延迟诊断

**诊断窗口**（`Ctrl+Shift+D` 或托盘 **Diagnostics**）用于看清 **STT** 与 **分析** 各自在做什么：

- **Pipeline**：音频 → Azure STT（字幕文本）→ 启发式（词表/正则，本地即时）→ 可选 LLM（`FOUNDRY_*`）→ 合并后写入浮层 Insights。
- **STT 延迟表**：Buffer / Azure / E2E / UI（见下表）。
- **Latest phrase — analysis split**：同一句的最终听写，分别展示启发式、LLM、合并三路的关键词 / 术语 / 问答。
- **Analysis history**：每句的启发式与 LLM 产出摘要、`llm_status`（`ok` / `empty` / `disabled` / `error`）及耗时。

| 指标 | 含义 |
|------|------|
| **Buffer** | REST 模式下每块音频累积时长（默认约 **2s**） |
| **Azure** | 单次 Azure STT HTTP 往返 |
| **E2E est.** | 估计从「话说完」到「字幕就绪」≈ Buffer + Azure |
| **UI** | 服务发出 → 浮层收到 |

---

## 音频采集

### macOS（Process Tap，推荐，无需 BlackHole）

**macOS 14.2+**（含你当前的 26.x）可用 Apple **Core Audio Process Tap** 直接抓系统播放声。`.env` 保持默认即可：

```env
THEMIS_AUDIO_CAPTURE_MODE=auto
```

1. `./scripts/themis.sh restart` → `./scripts/themis.sh tray` → **Cmd+Shift+T** 开始采集  
2. 首次会提示 **系统音频录制** 权限，请允许  
3. 状态行应显示 `process_tap`，播放 YouTube 时 `peak` / `frames` 上升  

自检：

```bash
./scripts/themis.sh probe
```

若 tap 不可用，会自动回退到**默认输入设备**（麦克风）；此时可设 `THEMIS_AUDIO_CAPTURE_MODE=input`，或可选安装 [BlackHole](https://existential.audio/blackhole/)（见 [docs/platform-notes.md](docs/platform-notes.md)）。

### Windows（WASAPI loopback）

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

- 先运行开发脚本启动服务：`.\scripts\themis.ps1 dev`（Windows）或 `./scripts/themis.sh dev`（macOS），或用 `restart` / `status` 确认进程在跑。
- 检查端口是否与 `.env` 中 `THEMIS_GRPC_PORT` 一致。

### macOS：Tauri 报错缺少 icon.icns

```bash
./scripts/themis.sh icons
# 或
./scripts/prepare-macos-icons.sh
```

### macOS：probe 失败 / peak 为 0

- 确认正在播放声音，且 **输出** 已路由到 BlackHole。
- **输入** 选 BlackHole；在 **隐私与安全性 → 麦克风** 中允许终端或 Themis。
- 运行 `./scripts/themis.sh probe` 查看 `detail` 中的设备名。

### 听写把技术词听错（如 RAG → Reg）

Azure 听中文视频里的英文缩写时，常把 **RAG** 听成 **Reg**（发音接近）。Themis 默认在 STT 结果上做一次**术语纠错**（[`transcript_fixup.rs`](crates/themis-azure/src/transcript_fixup.rs)）：

| 机制 | 说明 |
|------|------|
| **内置替换** | `Reg` / `REG` → `RAG`（词边界）；句中出现 AI、知识、资料等上下文时，`Reg，` 也会纠正 |
| **自定义** | `.env` 中 `AZURE_SPEECH_CORRECTIONS=误听:正确,Reg:RAG`（逗号分隔） |
| **关闭** | `THEMIS_STT_FIXUP=false` |

修改后 `.\scripts\themis.ps1 restart`。纠错后的文本会进入字幕与 Insights，词表里的 **RAG** 才能被正确解释。

进一步从上游减少误听（可选，需 Azure 侧配置）：

- 视频原声尽量清晰；`AZURE_SPEECH_LANGUAGE=auto` 或明确 `en-US,zh-CN`。
- 专有名词很多时，可在 Azure 门户为该 Speech 资源配置 **Custom Speech / 短语列表**（当前 REST 分块接口未直接传 phrase list，故以本地纠错为主）。

自定义纠错示例见 [.env.example](.env.example) 中 `AZURE_SPEECH_CORRECTIONS`（复制到 `.env` 后去掉注释）。

### 换别的语音识别模型会更好吗？

**有可能更好**，取决于场景：中文视频里夹英文术语、口音、背景 BGM 等。Themis **当前只接 Azure Speech**；换模型需要新做适配（接口在 `themis-azure`）。

| 方案 | 适合场景 | 优点 | 缺点 / 备注 |
|------|----------|------|-------------|
| **Azure Speech（现状）** | 已用 Azure、要低延迟云端 | 与项目集成完整；`auto` 中英并行；可配 Custom Speech | 英文缩写偶发误听 → 用 `AZURE_SPEECH_CORRECTIONS` |
| **Azure Custom Speech** | 固定领域词多（RAG、产品名） | 用你自己的音频+文本训练，专名词更稳 | 需标注数据与训练成本；集成比标准 STT 复杂 |
| **Azure Speech `streaming`** | 要更快出字 | 边说边出 partial | 本项目里曾不稳定，默认 `rest` |
| **OpenAI Whisper**（API 或本地） | 离线/成本敏感、多语言混合 | 术语与口音往往不错；本地无上传 | 非真正流式；延迟与算力需自管；需新写 `themis-whisper` |
| **Deepgram** | 英文或实时流式 | 流式 API 成熟、延迟低 | 中文+英混需实测；付费 API |
| **Google Cloud Speech-to-Text v2** | 已用 GCP | 短语/模型选择多 | 需新适配；国内访问视网络而定 |
| **AssemblyAI** | 英文播客/会议 | 标点与说话人体验好 | 中文场景需自行验证 |

**实用建议（不换代码的前提下）：**

1. 继续 **Azure + `AZURE_SPEECH_LANGUAGE=auto`**，并打开 **`AZURE_SPEECH_CORRECTIONS`**（见 `.env.example`）。  
2. 英文术语多的频道，可试 **`en-US,zh-CN`** 或暂时 **`en-US`** 对比效果。  
3. 若 Reg→RAG 仍多，优先 **纠错表 + glossary**，比立刻换云厂商划算。

**若你愿意换 Whisper / Deepgram 等**：可以说目标语言与延迟要求，可在 `themis-azure` 旁增加可选 backend（工作量中等，需改 `create_recognizer` 与配置项）。

### 能采集但几乎没有字幕 / 只有零星单词

先确认采集：`probe` 必须 `OK` 且 `peak > 200`（Windows：`.\scripts\themis.ps1 probe`；macOS：`./scripts/themis.sh probe`）。

1. 使用 **`AZURE_SPEECH_LANGUAGE=auto`**（默认）自动中英识别；或手动指定单一语言。
2. 确认 `.env` 已保存并 **`.\scripts\themis.ps1 restart`**。
3. 确认 `THEMIS_AUDIO_CAPTURE_MODE=auto`。
4. 尝试 `AZURE_SPEECH_MODE=streaming`（默认）；若 WebSocket 失败可暂用 `rest`。
5. 用 `.\scripts\themis.ps1 doctor` 检查 Azure 密钥与区域。

### Insights 侧栏没有内容 / 只有字幕

1. 确认听写的是**最终句**（灰色 partial 不会触发分析）。  
2. 说的内容是否在**词表**里，或是否为可识别的**问句**（见 [Insights 洞察](#insights-洞察关键词--术语--问答)）。画面字幕不会自动进入 Insights。  
3. 确认 `THEMIS_ANALYSIS_ENABLED` 未设为 `false`；改 `.env` 后 **`restart`**。  
4. 需要更强解释时配置 `FOUNDRY_*`（Azure OpenAI），不是 Speech Key。  
5. 若刚改过 `glossary.rs` / 分析逻辑，必须重新编译服务（`restart` 脚本会编译）。

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

Release 二进制：

- Windows：`target/release/themis-service.exe`
- macOS / Linux：`target/release/themis-service`

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
crates/themis-analysis  # Insights：启发式词表 + 可选 Azure OpenAI
crates/themis-ipc       # gRPC
crates/themis-service   # 后台服务入口
crates/themis-cli       # CLI / 服务安装
apps/themis-tray        # Tauri 托盘 + 浮层
scripts/themis.ps1           # Windows 一键脚本
scripts/themis.sh            # macOS/Linux 一键脚本
scripts/prepare-macos-icons.sh
dev.sh / restart.sh / tray.sh  # macOS 快捷入口（调用 themis.sh）
dev.cmd / restart.cmd / tray.cmd  # Windows 快捷入口
docs/                   # 架构与平台说明
packaging/              # 服务安装模板
```

---

## 诊断命令

```bash
cargo run -p themis-cli -- doctor
cargo run -p themis-cli -- status
cargo run -p themis-cli -- audio-probe --seconds 8   # Windows / macOS
# 或
./scripts/themis.sh doctor
./scripts/themis.sh probe
```

---

## iOS 路线图

v0.1 **不包含** iOS 系统内录；受平台限制，后续可能通过 ReplayKit 等方案。见 [docs/platform-notes.md](docs/platform-notes.md)。

---

## License

MIT

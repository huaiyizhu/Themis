# Themis 使用说明（Windows）

**Themis（忒弥斯）** 在电脑**播放声音**时实时生成字幕，并在浮层中展示术语解释、问题初答与会话摘要。适合网课、播客、技术视频，以及 Zoom / Teams 等线上会议。

> 默认采集**系统播放声**（浏览器、会议里对方的声音），不是全程录麦克风。检测到 Zoom、Teams 等通话软件时会**自动同时采集麦克风**，以便转写你说的话。

---

## 一、产品能做什么

| 能力 | 说明 |
|------|------|
| **实时字幕** | 抓取系统播放音频 → Azure Speech 听写 → 浮层逐句显示（灰色 partial、黑色 final） |
| **Insights** | 每句 final 后分析 **关键词 / 术语解释 / 问题初答**（内置词表 + 可选 Azure OpenAI） |
| **全文总结** | 根据当前会话内容周期性刷新摘要（配 LLM 后效果更好） |
| **延迟诊断** | 独立窗口查看 STT / 启发式 / LLM 各阶段耗时 |
| **迷你浮标** | 缩成圆形桌面浮标，不占屏幕空间 |
| **始终置顶** | 可调透明度、主题、字号；支持居中约 ⅔ 屏唤醒 |

**不适合：** 只有画面字幕、没有旁白的内容（不做 OCR）；完全离线（需 Azure Speech，Mock 仅用于体验 UI）。

---

## 二、快速开始

1. 将同目录 **`windows-x86_64-env.example`**（或 `*-env.example`）**复制为 `.env`**（与 `themis-tray.exe` 同一文件夹，无其他扩展名）。
2. 编辑 `.env`：至少填写 **`AZURE_SPEECH_KEY`**、**`AZURE_SPEECH_REGION`**；Insights 建议填写 **`FOUNDRY_*`**（见下文配置说明）。
3. 双击 **`*-themis-tray.exe`** 启动；`themis-service.exe` 会自动拉起。
4. 按 **`Ctrl+Shift+T`** 开始采集，或右键系统托盘图标选择 Toggle capture。
5. 运行 **`.\themis-cli.exe doctor`** 确认 Speech 与 foundry 均已 configured。

> **未创建 `.env` 或未填 Speech Key 时，听写为 Mock 模式**（浮层出现 `[mock] …` 假字幕），不能用于真实场景。

便携使用时：**tray、service、cli 与 `.env` 须在同一目录**。

---

## 三、Release 包文件说明

| 文件 | 用途 |
|------|------|
| `*-README.md` | 本说明（请先阅读） |
| `*-env.example` | 配置模板 → 复制为 `.env` |
| `*-themis-tray.exe` | 主程序：托盘 + 字幕浮层 |
| `*-themis-service.exe` | 后台：音频采集 + Azure STT |
| `*-themis-cli.exe` | 诊断工具（可选） |
| `*-setup.exe` | NSIS 安装程序（可选） |

---

## 四、浮层布局

```
┌──────────────────────────────────────────────┐
│ 标题栏 · 捕捉/诊断/中文/清空 · 透/字/浮/尺寸…  │
├──────────────────────────────────────────────┤
│ 全文总结（Session Summary）                   │
├────────────────────┬─────────────────────────┤
│ Questions          │ Terms / Keywords        │
├────────────────────┴─────────────────────────┤
│ 实时字幕（Transcript）                        │
└──────────────────────────────────────────────┘
```

- 状态行：采集模式、延迟、配置交叉检查（STT / LLM .env vs 服务是否一致）
- 拖动标题栏可移动窗口；拖动分隔条可调整 Questions/Terms 宽度与字幕区高度

---

## 五、标题栏按钮说明

### 第一行（主要操作）

| 按钮 | 作用 |
|------|------|
| **捕捉** | 开始/停止系统音频采集与听写（`Ctrl+Shift+T`） |
| **诊断** | 打开/关闭延迟诊断窗口，查看 STT、启发式、LLM 三路结果（`Ctrl+Shift+D`） |
| **中文** | 切换界面与 Insights 文案的中/英显示 |
| **清空** | 清空当前字幕、总结与 Insights，从零继续监听 |

### 第二行（显示与窗口）

| 控件 | 作用 |
|------|------|
| **透 − / +** | 降低 / 提高浮层透明度（`Ctrl+Shift+[` / `]`） |
| **字 − / + / ↺** | 缩小 / 放大 / 重置字号（`Ctrl+Shift+-` / `=` / `0`） |
| **浮** | 迷你浮标模式：缩成圆形图标，点击恢复（`Ctrl+Shift+M`） |
| **尺寸** | 预设窗口大小（紧凑 / 标准 / 宽屏等） |
| **隐藏** | 隐藏浮层窗口，采集可继续；托盘或 `Ctrl+Shift+O` 可再次打开 |
| **退出** | 完全退出 Themis（`Ctrl+Shift+Q`） |

### 其他区域

| 区域 | 作用 |
|------|------|
| **全文总结** | 会话级摘要，约每 20 秒刷新（需 LLM 配置时内容更充实） |
| **Questions** | 识别到的问句及初答；可点击卡片固定 |
| **Terms / Keywords** | 术语解释与关键词标签 |
| **实时字幕 ▾** | 折叠/展开字幕区（`Ctrl+Shift+H` 切换显示） |
| **主题徽章** | 当前主题；`Ctrl+Shift+S` 切换样式，`Ctrl+Shift+A` 自适应对比度 |

---

## 六、快捷键（Windows：`Ctrl+Shift+` + 键）

| 操作 | 键 | 说明 |
|------|-----|------|
| 开始/停止采集 | `T` | 核心操作 |
| 唤醒/置顶浮层 | `O` | 居中约 ⅔ 屏宽 |
| 诊断窗口 | `D` | STT / 分析拆分 |
| 显示/隐藏字幕区 | `H` | 只藏字幕条 |
| 迷你浮标 | `M` | 圆形桌面浮标 |
| 透明度 − / + | `[` / `]` | 同「透 − / +」 |
| 字号 − / + / 重置 | `-` / `=` / `0` | 同「字 − / + / ↺」 |
| 切换浮层风格 | `S` | 主题样式 |
| 自适应对比度 | `A` | 根据背景调整 |
| 退出 | `Q` | 停止托盘与采集 |

**系统托盘：** 右键 → Toggle capture / Diagnostics / Quit；左键 → 显示/隐藏浮层。

---

## 七、`.env` 配置说明

将 `*-env.example` 复制为 `.env` 后，按下列说明填写。改 `.env` 后须**完全退出 Themis 再打开**（仅关浮层不够）。

### 7.1 Azure Speech（字幕，必填）

| 变量 | 必填 | 说明 |
|------|------|------|
| `AZURE_SPEECH_KEY` | **是** | Speech 资源密钥；缺则 Mock 假字幕 |
| `AZURE_SPEECH_REGION` | **是** | 区域，如 `eastus` |
| `AZURE_SPEECH_LANGUAGE` | 建议 | 默认 `auto`（中英）；可 `en-US`、`zh-CN` |
| `AZURE_SPEECH_MODE` | 否 | `rest`（默认）或 `streaming` |

### 7.2 Azure OpenAI（Insights，强烈建议）

| 变量 | 必填 | 说明 |
|------|------|------|
| `FOUNDRY_ENDPOINT` | 建议 | Azure OpenAI 终结点 URL |
| `FOUNDRY_API_KEY` | 建议 | Azure OpenAI Key（**不是** Speech Key） |
| `FOUNDRY_DEPLOYMENT` | 建议 | 部署名，如 `gpt-4o-mini` |
| `THEMIS_ANALYSIS_ENABLED` | 否 | 默认 `true`；`false` 关闭全部 Insights |
| `THEMIS_INSIGHT_DWELL_SECS` | 否 | 已弃用（术语/问题卡片不再自动消失，保留配置项兼容旧 `.env`） |
| `THEMIS_SESSION_SUMMARY_INTERVAL_SECS` | 否 | 全文总结刷新间隔，默认 20 秒 |

| 能力 | 无 `FOUNDRY_*` | 有 `FOUNDRY_*` |
|------|----------------|----------------|
| 实时字幕 | ✅（需 Speech Key） | ✅ |
| 关键词 / 术语 | 仅内置词表 | 开放域术语 + 更丰富解释 |
| 问题初答 | 简单模板 | 更完整的 2–3 句回答 |
| 全文总结 | 弱或空白 | 周期性会话摘要 |

### 7.3 听写纠错

| 变量 | 说明 |
|------|------|
| `THEMIS_STT_FIXUP` | 默认 `true`，内置 Reg→RAG 等 |
| `AZURE_SPEECH_CORRECTIONS` | 自定义 `听错:正确` 对；**含空格须加双引号**，否则其后变量加载失败 |

### 7.4 音频采集（Windows）

| 变量 | 说明 |
|------|------|
| `THEMIS_AUDIO_CAPTURE_MODE` | `auto`（默认）/ `process` / `endpoint` / `call` / `dual` |
| `THEMIS_AUDIO_OUTPUT_DEVICE` | `endpoint` 模式下指定播放设备名子串 |
| `THEMIS_AUDIO_INPUT_DEVICE` | 双路/通话模式下麦克风设备名子串 |
| `THEMIS_AUDIO_GAIN_MAX` | 自动增益上限，默认 16 |

`auto` 检测到 Zoom、Teams 等通话 app 时会**输出 + 麦克风双路混音**。

### 7.5 其他

| 变量 | 说明 |
|------|------|
| `THEMIS_GRPC_PORT` | gRPC 端口，默认 `50051` |
| `THEMIS_LOG_LEVEL` | 日志级别，默认 `info` |
| `THEMIS_USE_MOCK_SPEECH` | 缺 Key 时默认自动 Mock；可显式设为 `true` |

---

## 八、命令行（可选）

在 exe 所在目录打开 PowerShell：

```powershell
.\themis-cli.exe doctor
.\themis-cli.exe audio-probe --seconds 8
.\themis-cli.exe status
```

日志：`%LOCALAPPDATA%\Themis\logs`

---

## 九、常见问题

**只有 `[mock]` 假字幕**  
→ `.env` 是否在 exe 旁；`doctor` 是否显示 STT 非 mock；Speech Key/Region 是否正确。

**Insights 为空**  
→ 配置 `FOUNDRY_*`；确认有 **final** 句（非灰色临时字）且正在播放**有声音**的内容（不识别画面 OCR）。

**Service offline**  
→ `themis-service.exe` 与 tray 同目录；重启 tray。

**改 `.env` 不生效**  
→ 完全退出 Themis 再开。

**托盘两个图标**  
→ 完全退出后只启动一次。

更多文档：GitHub 仓库 README。

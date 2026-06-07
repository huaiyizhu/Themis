# Themis 使用说明（macOS）

**Themis（忒弥斯）** 在 Mac **播放声音**时实时生成字幕，并在浮层中展示术语解释、问题初答与会话摘要。

> 默认采集系统播放；检测到 Zoom、Teams 等时会自动加麦克风。首次运行需授予**系统音频录制**权限。

---

## 一、产品能做什么

与 Windows 版相同：实时字幕、Insights（关键词/术语/问题）、全文总结、诊断窗口、迷你浮标、可调透明度与字号。

**不适合：** 仅画面字幕（无 OCR）；完全离线。

---

## 二、快速开始

1. 将 **`*-env.example`** 复制为 **`.env`**（与 `themis-tray` 同目录）。
2. 填写 `AZURE_SPEECH_KEY`、`AZURE_SPEECH_REGION`；Insights 建议填 `FOUNDRY_*`。
3. 运行 `./themis-tray`；菜单栏出现图标后按 **`Ctrl+Shift+T`**（macOS 上为 **Cmd+Shift+T**）开始采集。
4. `./themis-cli doctor` 验证配置。

未配 `.env` → Mock 假字幕，不能真实听写。

---

## 三、浮层按钮与快捷键

布局与按钮含义与 Windows 版相同（见 Windows README 第四、五节）。

macOS 快捷键使用 **Cmd+Shift+** 代替 Ctrl+Shift+：

| 操作 | 键 |
|------|-----|
| 开始/停止采集 | `T` |
| 唤醒浮层 | `O` |
| 诊断 | `D` |
| 隐藏字幕区 | `H` |
| 迷你浮标 | `M` |
| 透明度 − / + | `[` / `]` |
| 字号 − / + / 重置 | `-` / `=` / `0` |
| 退出 | `Q` |

---

## 四、`.env` 配置说明

字段含义与 Windows 版 **`windows-x86_64-env.example`** 内注释一致。macOS 差异：

| 变量 | 说明 |
|------|------|
| `THEMIS_AUDIO_CAPTURE_MODE` | `auto` / `process_tap` / `input` / `call` / `dual` |
| `THEMIS_AUDIO_OUTPUT_DEVICE` | macOS 上忽略 |

Process Tap 不可用时可用 `input` 模式并指定 `THEMIS_AUDIO_INPUT_DEVICE`。

---

## 五、常见问题

**双击 Themis.app 提示「已损坏，无法打开」** → Safari 下载触发了隔离属性。执行 `xattr -cr /Applications/Themis.app` 后打开；或改用 Release 页的 **便携版**（`macos-aarch64-themis-tray` + `themis-service` 同目录）。详见 Release README 或 dmg 内「请先阅读-安装说明.txt」。

**没有字幕** → 检查 `.env`、系统音频录制权限、`./themis-cli audio-probe`。

**Insights 空** → 配置 `FOUNDRY_*`；`doctor` 显示 foundry configured。

详见 GitHub README。

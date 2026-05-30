# Themis Release 下载说明

**Themis（忒弥斯）** — 在电脑播放声音时实时显示字幕，并在浮层中提供术语解释、问题初答与会话摘要。适合网课、播客、技术视频与 Zoom / Teams 会议。

---

## 快速开始

1. 下载与你系统**匹配前缀**的全部文件到**同一文件夹**（如 `windows-x86_64-*`）。
2. 打开 **平台 README**（见下表），将 **`*-env.example`** 复制为 **`.env`** 并填入密钥。
3. 运行 `*-themis-tray` / `*-themis-tray.exe`；用 `*-themis-cli doctor` 确认配置。

> **未创建 `.env` 时听写为 Mock（假字幕），不能用于真实场景。**

---

## 平台用户手册（含产品介绍、按钮说明、`.env` 字段详解）

| 系统 | 文件名 |
|------|--------|
| Windows 64 位 | `windows-x86_64-README.md` |
| macOS Apple Silicon | `macos-aarch64-README.md` |
| macOS Intel | `macos-x86_64-README.md` |

配置模板：同前缀的 **`*-env.example`** → 复制为 **`.env`**。

---

## 你需要准备

1. **Azure Speech**（`AZURE_SPEECH_*`）→ 实时字幕  
2. **Azure OpenAI**（`FOUNDRY_*`）→ Insights 核心（术语 / 问题 / 总结），强烈建议一并配置  

源码与开发者文档：GitHub 仓库 README。

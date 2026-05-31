# Themis 本地 Release 包（Windows x64）

本目录由 `.\scripts\build-release.ps1` 生成，所有文件放在**同一文件夹**即可使用。

---

## 文件说明与运行方式

| 文件 | 作用 | 如何运行 |
|------|------|----------|
| **themis-tray.exe** | 主程序：系统托盘图标 + 字幕/Insights 浮层（UI 内嵌在 exe 中） | 双击启动；须由 `build-release.ps1` 经 **`tauri build`** 生成，勿单独 `cargo build` 托盘 |
| **themis-service.exe** | 后台服务：音频采集、Azure 听写、Insights（gRPC），**无控制台窗口** | 由 tray 自动拉起；日志见 `%LOCALAPPDATA%\Themis\logs` |
| **themis-cli.exe** | 命令行诊断（可选） | 在 PowerShell 中：`.\themis-cli.exe doctor` 检查 Speech / Foundry 是否已配置 |
| **.env.example** | 配置字段说明与示例（可选参考） | 不必手动复制；见下文「配置方式」 |
| **README.md** | 本说明 | — |
| **\*-setup.exe**（若有） | NSIS 安装程序 | 双击安装到 Program Files；便携使用可忽略 |

**推荐启动：**

```powershell
cd <本目录>
.\themis-tray.exe
```

按 **Ctrl+Shift+T** 开始/停止采集；**Ctrl+Shift+O** 唤醒浮层。

---

## 配置方式（不必先复制 `.env`）

1. 启动 **themis-tray.exe** 后，看浮层底部**配置检查**行：会标出未配置的 **STT（Azure Speech）**、**LLM（FOUNDRY_*）** 等。
2. 点击标题栏 **「配置」** 按钮，在窗口中填写 `AZURE_SPEECH_KEY`、`AZURE_SPEECH_REGION` 以及（建议）`FOUNDRY_*`，点击 **保存并重新加载**——程序会在本目录**自动创建/更新 `.env`** 并重启服务。
3. 也可手动：`copy .env.example .env` 后用记事本编辑；改完后完全退出 Themis 再打开，或于配置窗口点「重新加载」。

**至少配置（真实听写）：** `AZURE_SPEECH_KEY`、`AZURE_SPEECH_REGION`  
**Insights 建议：** `FOUNDRY_ENDPOINT`、`FOUNDRY_API_KEY`、`FOUNDRY_DEPLOYMENT`

未配置 Speech 时为 **Mock** 假字幕，仅用于体验 UI。

---

## 常见问题

- **「不一致，请 restart 服务」**：先 `taskkill /IM themis-service.exe /F`，再只从本目录启动 `themis-tray.exe`；勿在跑完 `build-release.ps1` 的同一终端里启动（可能残留 `THEMIS_USE_MOCK_SPEECH`）。
- **doctor 检查**：`.\themis-cli.exe doctor`

完整功能说明见 GitHub 仓库 README。

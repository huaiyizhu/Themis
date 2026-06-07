# Themis Windows 版（64 位）

你从 GitHub Releases 下载 **`Themis-windows-x86_64.zip`**，解压后进入 **本文件夹**（`Themis-Windows/`）。

本文件夹内文件**已齐全**，无需再去 Release 页单独下载其他附件。

---

## 本文件夹里有什么

| 文件 | 是否必需 | 说明 |
|------|----------|------|
| **themis-tray.exe** | ✅ | 主程序：托盘图标 + 字幕/Insights 浮层（**推荐用这个**） |
| **themis-service.exe** | ✅ | 后台：音频采集与 Azure 听写（tray 自动拉起） |
| **themis-cli.exe** | 可选 | 命令行诊断：`.\themis-cli.exe doctor` |
| **\*-setup.exe** | 可选 | NSIS 安装程序；与便携版二选一即可 |
| **env.example** | 参考 | 配置模板 |
| **.env.example** | 参考 | 同上（部分工具会隐藏点开头的文件） |
| **README.md** | — | 本说明 |

---

## 快速开始（便携版，推荐）

```powershell
cd Themis-Windows
.\themis-tray.exe
```

按 **Ctrl+Shift+T** 开始/停止采集；**Ctrl+Shift+O** 唤醒浮层。

---

## 或使用安装包（可选）

双击 **`*-setup.exe`** 安装到 Program Files。便携版（上一节）无需安装。

---

## 配置 Azure

**不必先手动建 `.env`：**

1. 启动 **themis-tray.exe** 后，看浮层底部配置检查
2. 点 **「配置」**，填写 Speech / Foundry 字段，**保存并重新加载**

也可手动：

```powershell
copy env.example .env
# 编辑 AZURE_SPEECH_KEY、AZURE_SPEECH_REGION；Insights 建议 FOUNDRY_*
```

未配置 Speech 时为 Mock 假字幕。

---

## 常见问题

- **「不一致，请 restart 服务」**：`taskkill /IM themis-service.exe /F` 后只从本目录启动 `themis-tray.exe`
- 诊断：`.\themis-cli.exe doctor`
- 完整文档：GitHub 仓库 README

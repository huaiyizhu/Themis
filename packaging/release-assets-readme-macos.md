# Themis macOS 版

你从 GitHub Releases 下载 **`Themis-macos-aarch64.zip`**（Apple Silicon）或 **`Themis-macos-x86_64.zip`**（Intel），解压后进入 **本文件夹**（`Themis-macOS-Apple-Silicon/` 或 `Themis-macOS-Intel/`）。

本文件夹内文件**已齐全**，无需再去 Release 页单独下载其他附件。

---

## 本文件夹里有什么

| 文件 | 是否必需 | 说明 |
|------|----------|------|
| **themis-tray** | ✅ | 主程序：菜单栏 + 字幕/Insights 浮层（**推荐用这个**） |
| **themis-service** | ✅ | 后台：音频采集与 Azure 听写（通常由 tray 自动拉起） |
| **themis-cli** | 可选 | 命令行诊断：`./themis-cli doctor` |
| **\*.dmg** | 可选 | 安装镜像；与便携版二选一即可 |
| **env.example** | 参考 | 配置模板（可见文件名） |
| **.env.example** | 参考 | 同上；Finder 默认**不显示**以 `.` 开头的文件 |
| **README.md** | — | 本说明 |

> **看不到 `.env.example`？** 在 Finder 按 **Cmd+Shift+.** 显示隐藏文件，或直接用 **`env.example`**（内容与 `.env.example` 相同）。

---

## 快速开始（便携版，推荐）

```bash
cd Themis-macOS-Apple-Silicon   # 或 Themis-macOS-Intel
chmod +x themis-tray themis-service
xattr -cr .                     # 解除 Safari 下载隔离
./themis-tray
```

按 **Cmd+Shift+T** 开始/停止采集。首次需授予**系统音频录制**权限。

若正在跑开发版 `./scripts/themis.sh tray`，先退出并：`pkill -x themis-tray; pkill -x themis-service`

---

## 或使用 .dmg 安装（可选）

1. 双击本文件夹内的 **`Themis_*.dmg`**
2. 将 **Themis.app** 拖入「应用程序」
3. 若提示 **「已损坏，无法打开」**（Gatekeeper，不是文件坏了）：

```bash
xattr -cr /Applications/Themis.app
open /Applications/Themis.app
```

或在 Finder **右键 Themis.app → 打开 → 再次点打开**。

便携版（上一节）通常更简单，不必处理 `.app` 隔离。

---

## 配置 Azure

**不必先手动建 `.env`：**

1. 启动 **themis-tray** 后，看浮层底部配置检查（未配置项会标红）
2. 点 **「配置」**，填写 Speech / Foundry 字段，**保存并重新加载**（自动写入本目录 `.env`）

也可手动：

```bash
cp env.example .env    # 或 cp .env.example .env
# 编辑 AZURE_SPEECH_KEY、AZURE_SPEECH_REGION；Insights 建议配置 FOUNDRY_*
```

未配置时为 Mock 假字幕。

---

## 常见问题

- 配置改了仍不对：完全退出后重新 `./themis-tray`
- 诊断：`./themis-cli doctor`
- 完整文档：GitHub 仓库 README

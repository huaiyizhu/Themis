# Themis Release 下载说明

**Themis（忒弥斯）** — 在电脑播放声音时实时显示字幕，并在浮层中提供术语解释、问题初答与会话摘要。

[GitHub Releases 页面](https://github.com/huaiyizhu/Themis/releases)

---

## 下载哪个文件？（按操作系统）

每个 Release 提供 **3 个 ZIP**，解压后是一个文件夹，**只下载与你电脑匹配的一个即可**：

| 你的系统 | 下载这个 ZIP | 解压后的文件夹 |
|----------|--------------|----------------|
| **Windows 64 位** | `Themis-windows-x86_64.zip` | `Themis-Windows/` |
| **macOS Apple Silicon**（M1 / M2 / M3 / M4） | `Themis-macos-aarch64.zip` | `Themis-macOS-Apple-Silicon/` |
| **macOS Intel**（x86_64） | `Themis-macos-x86_64.zip` | `Themis-macOS-Intel/` |

另附 **`README.md`**（本说明的副本）。

> 不再需要在一长串文件里找 `macos-aarch64-themis-tray` 等带前缀的文件名。

---

## 文件夹里有什么？

| 文件 | 是否必需 | 作用 |
|------|----------|------|
| **themis-tray** / **themis-tray.exe** | ✅ 必需 | 主程序：菜单栏/托盘 + 字幕浮层 |
| **themis-service** / **themis-service.exe** | ✅ 必需 | 后台：音频采集 + Azure 听写（通常由 tray 自动拉起） |
| **README.md** | 建议阅读 | 本平台安装与按钮说明 |
| **.env.example** | 参考 | 配置模板；也可启动后在 tray 里点「配置」 |
| **themis-cli** / **themis-cli.exe** | 可选 | 命令行诊断：`doctor`、`audio-probe` |
| **\*-setup.exe**（Windows） | 可选 | NSIS 安装包，与便携版二选一 |
| **\*.dmg**（macOS） | 可选 | 安装镜像；便携版 `./themis-tray` 更简单 |

---

## 快速开始

### Windows

1. 下载并解压 **`Themis-windows-x86_64.zip`**
2. 进入 **`Themis-Windows/`** 文件夹
3. 双击 **`themis-tray.exe`**（或 `*-setup.exe` 安装）
4. 按 **`Ctrl+Shift+T`** 开始/停止采集

### macOS（Apple Silicon 示例）

1. 下载并解压 **`Themis-macos-aarch64.zip`**
2. 进入 **`Themis-macOS-Apple-Silicon/`**
3. 终端执行：

```bash
cd Themis-macOS-Apple-Silicon
chmod +x themis-tray themis-service
xattr -cr .          # 解除 Safari 下载隔离
./themis-tray
```

4. 按 **`Cmd+Shift+T`** 开始/停止采集

> **提示「Themis.app 已损坏」？** 见文件夹内 `README.md` 第一节，或执行 `xattr -cr /Applications/Themis.app`。

---

## 配置 Azure

1. 复制 **`.env.example`** 为 **`.env`**，填入 `AZURE_SPEECH_KEY`、`AZURE_SPEECH_REGION`
2. Insights 建议配置 **`FOUNDRY_*`**
3. 或在 tray 浮层点 **「配置」** 保存后自动写入 `.env`

未配置时为 Mock 假字幕，不能用于真实听写。

---

## 你需要准备

1. **Azure Speech** → 实时字幕  
2. **Azure OpenAI（Foundry）** → 术语 / 问题 / 会话摘要（强烈建议）

源码与开发者文档：GitHub 仓库 README。

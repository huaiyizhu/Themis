# Themis 本地 Release 包（macOS）

本目录由 `./scripts/build-release.sh` 生成，所有文件放在**同一文件夹**即可使用。

---

## ⚠️ 从 GitHub / Safari 下载后提示「Themis.app 已损坏」？

这是 **macOS Gatekeeper** 对未 Apple 公证应用的常见提示，**不是文件损坏**。

**任选一种方式：**

```bash
# 方法一：解除隔离（推荐）
xattr -cr /Applications/Themis.app
open /Applications/Themis.app

# 方法二：便携版（同一 Release 页下载 themis-tray + themis-service 到同一文件夹）
cd <下载文件夹>
chmod +x themis-tray themis-service
xattr -cr .
./themis-tray
```

或在 Finder 中 **右键 Themis.app → 打开 → 再次点打开**。

仓库内也可运行：`./scripts/fix-macos-app-quarantine.sh /Applications/Themis.app`

---

## 文件说明与运行方式

| 文件 | 作用 | 如何运行 |
|------|------|----------|
| **themis-tray** | 主程序：菜单栏图标 + 字幕/Insights 浮层 | 终端：`chmod +x themis-tray && ./themis-tray`；或双击（若已签名/允许） |
| **themis-service** | 后台服务：音频采集、Azure 听写、Insights（gRPC） | 通常由 tray 自动拉起；也可先 `./themis-service` 再开 tray |
| **themis-cli** | 命令行诊断（可选） | `./themis-cli doctor` |
| **.env.example** | 配置说明与示例（可选参考） | 不必手动复制；见下文 |
| **README.md** | 本说明 | — |
| **\*.dmg**（若有） | 安装镜像 | 打开 dmg 拖入「应用程序」 |

**推荐启动：**

```bash
cd <本目录>
# 若正在跑 ./scripts/themis.sh tray（开发版），先退出并停止后台 service：
pkill -x themis-tray 2>/dev/null; pkill -x themis-service 2>/dev/null
chmod +x themis-tray themis-service themis-cli
./themis-tray
```

按 **Cmd+Shift+T** 开始/停止采集。首次需授予**系统音频录制**权限。

---

## 配置方式（不必先复制 `.env`）

1. 启动 **themis-tray** 后，查看浮层底部配置检查：未配置项会标红并提示。
2. 点击 **「配置」**，填写 Azure Speech / Foundry 字段，**保存并重新加载**（自动写入本目录 `.env`）。
3. 或：`cp .env.example .env` 后编辑。

**至少：** `AZURE_SPEECH_KEY`、`AZURE_SPEECH_REGION`  
**Insights 建议：** `FOUNDRY_*`

未配置时为 Mock 假字幕。

---

## 常见问题

- 配置变更后若状态仍不对：退出 Themis 后重新 `./themis-tray`。
- `./themis-cli doctor` 验证配置。

完整说明见 GitHub 仓库 README。

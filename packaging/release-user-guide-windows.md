# Themis 使用说明（Windows）

## 首次使用（3 步）

1. **复制配置模板**：将同目录下的 *-env.example 复制为 **.env**（与 	hemis-tray.exe 同一文件夹）。
2. **填写密钥**：编辑 .env，至少设置 AZURE_SPEECH_KEY 与 AZURE_SPEECH_REGION；Insights 建议再填 LLM_*（或旧名 FOUNDRY_*）。
3. **启动**：双击 *-themis-tray.exe。	hemis-service 会自动拉起。

验证：在该目录运行 *-themis-cli.exe doctor，应看到 Speech 与 llm 状态。

---

## 没有 .env 会怎样？

| 情况 | 行为 |
|------|------|
| 无 .env 或未填 Speech Key | **Mock 听写** — 浮层会出现 [mock] … 假字幕，不是真实转写 |
| 未填 LLM | 字幕可用（有 Speech Key 时）；Insights 仅内置词表，术语/问题/总结很弱 |

**结论：直接运行 exe 而不配置 .env，默认就是 Mock STT。**

---

## 文件说明

| 文件 | 用途 |
|------|------|
| *-themis-tray.exe | 主程序（托盘 + 浮层） |
| *-themis-service.exe | 后台服务（采集 + Azure STT） |
| *-themis-cli.exe | 诊断：doctor、udio-probe |
| *-env.example | 复制为 .env 后编辑 |
| *-setup.exe | 可选：安装到本机 |

便携使用时，**tray / service / cli 与 .env 放在同一目录**。

---

## 常用快捷键

| 操作 | 快捷键 |
|------|--------|
| 开始/停止采集 | Ctrl+Shift+T |
| 唤醒浮层 | Ctrl+Shift+O |
| 诊断窗口 | Ctrl+Shift+D |

改 .env 后须**完全退出 Themis 再打开**（仅关浮层不够）。

---

## 常见问题

**只有 mock 字幕** → 检查 .env 是否在 exe 旁；doctor 是否显示 STT 非 mock。

**Insights 为空** → 配置 LLM_*（或 FOUNDRY_*）；确认有 final 句且正在播放有声音的内容。

**Service offline** → 	hemis-service.exe 须与 tray 同目录。

更多说明见 GitHub 仓库 README。

# Themis 使用说明（macOS）

## 首次使用（3 步）

1. 将同目录 *-env.example **复制为 .env**（与 	hemis-tray 同目录）。
2. 编辑 .env：必填 AZURE_SPEECH_KEY、AZURE_SPEECH_REGION；Insights 建议填 LLM_*（或 FOUNDRY_*）。
3. 运行 	hemis-tray；首次需授予**系统音频录制**权限。

验证：./themis-cli doctor

---

## 没有 .env 会怎样？

无 Speech Key → **Mock 听写**（[mock] … 假字幕）。无 LLM → Insights 仅词表启发式。

**直接运行而不配置 .env，默认不是真实听写。**

---

## 文件

| 文件 | 用途 |
|------|------|
| 	hemis-tray | 主程序 |
| 	hemis-service | 后台服务 |
| 	hemis-cli | 诊断 |
| *-env.example | 复制为 .env |
| *.dmg | 可选安装包 |

快捷键与 Windows 相同（Ctrl+Shift+T 等）。改 .env 后完全退出再开。

详见 GitHub README。

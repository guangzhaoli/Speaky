# Speaky

跨平台语音输入工具，实时将语音转换为文字。按下快捷键，说话，文字自动输入到光标位置。

![Platform](https://img.shields.io/badge/平台-Windows%20%7C%20macOS%20%7C%20Linux-blue)
![License](https://img.shields.io/badge/许可证-MIT-green)

[English](README.md) | 中文

## 功能特性

- **全局快捷键** - 按 `Alt+Space`（可自定义）随时随地开始/停止录音
- **实时转写** - 语音即时转换为文字
- **自动输入** - 转写结果自动输入到光标位置
- **LLM 润色** - 可选的 AI 后处理，自动添加标点、修正错误、优化格式
- **多服务商支持** - 支持 DeepSeek、OpenAI、Kimi、Gemini、智谱、Ollama 及任何 OpenAI 兼容 API
- **录音指示器** - 录音时显示视觉提示
- **历史记录** - 浏览和复制历史转写结果
- **深色/浅色主题** - 精美 UI，支持主题切换
- **跨平台** - 支持 Windows、macOS 和 Linux

## 截图

<!-- 在此添加截图 -->

## 安装

### 预编译版本

从 [Releases](https://github.com/guangzhaoli/Speaky/releases) 页面下载适合你平台的最新版本。

- **Windows**: 下载 `.msi` 或 `.exe` 安装包
- **macOS**: 下载 `.dmg` 文件
- **Linux**: 下载 `.AppImage` 或 `.deb` 包

### 从源码构建

前置要求：
- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/) 1.70+
- [Tauri CLI](https://tauri.app/)

```bash
# 克隆仓库
git clone https://github.com/guangzhaoli/Speaky.git
cd Speaky

# 安装依赖
npm install

# 开发模式运行
npm run tauri dev

# 生产构建
npm run tauri build
```

## 配置

### 语音识别 (ASR)

Speaky 使用[火山引擎豆包语音识别](https://www.volcengine.com/product/speech-recognition)服务。你需要：

1. 在[火山引擎控制台](https://console.volcengine.com/)注册账号
2. 开通语音识别服务
3. 创建应用并获取凭证：
   - App ID
   - Access Token
   - Secret Key（可选）

在 设置 > General > API Configuration 中填入这些信息。

### LLM 润色（可选）

在 设置 > LLM Polish 中启用 AI 文本润色：

1. 开启 "Enable LLM Post-Processing"
2. 选择处理模式：
   - **General** - 日常文本输入
   - **Code** - 保留技术术语和语法
   - **Meeting** - 正式书面风格
3. 添加并配置 API 服务商

支持的服务商：
- DeepSeek（深度求索）
- OpenAI
- Kimi（月之暗面）
- Google Gemini
- 智谱（GLM）
- Ollama（本地部署）
- 任何 OpenAI 兼容 API

## 使用方法

1. **开始录音**：按全局快捷键（默认 `Alt+Space`）或点击麦克风按钮
2. **说话**：自然说话，语音会实时转写
3. **停止录音**：松开快捷键或再次点击按钮
4. **自动输入**：转写文字自动输入到光标位置

### 快捷键

| 快捷键 | 操作 |
|--------|------|
| `Alt+Space` | 开始/停止录音（可自定义） |

## 技术栈

- **框架**: [Tauri v2](https://tauri.app/)（Rust + React）
- **前端**: React 19 + TypeScript + TailwindCSS v4
- **音频**: [cpal](https://github.com/RustAudio/cpal)（跨平台音频采集）
- **键盘模拟**: [enigo](https://github.com/enigo-rs/enigo)
- **语音识别**: 火山引擎豆包 ASR 2.0（WebSocket 二进制协议）
- **LLM**: OpenAI 兼容 API

## 项目结构

```
speaky/
├── src/                    # React 前端
│   ├── App.tsx            # 主应用组件
│   ├── main.tsx           # React 入口
│   └── style.css          # TailwindCSS 样式
├── src-tauri/             # Rust 后端
│   ├── src/
│   │   ├── lib.rs         # Tauri 应用设置
│   │   ├── commands.rs    # IPC 命令
│   │   ├── state.rs       # 应用状态
│   │   ├── audio/         # 音频采集
│   │   ├── asr/           # 语音识别
│   │   ├── input/         # 键盘模拟
│   │   └── postprocess/   # LLM 后处理
│   └── Cargo.toml         # Rust 依赖
├── package.json           # Node.js 依赖
└── tauri.conf.json        # Tauri 配置
```

## 贡献

欢迎贡献！请随时提交 Pull Request。

## 许可证

本项目基于 MIT 许可证开源 - 详见 [LICENSE](LICENSE) 文件。

## 致谢

- [Tauri](https://tauri.app/) 提供优秀的框架
- [火山引擎](https://www.volcengine.com/) 提供语音识别服务

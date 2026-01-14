# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 技术栈

**框架**
- Tauri v2 (Rust 后端 + React 前端)
- TypeScript + Vite

**后端 (Rust)**
- cpal - 跨平台音频采集
- tokio-tungstenite - WebSocket 客户端
- enigo - 键盘模拟
- parking_lot - 并发原语
- flate2 - Gzip 压缩/解压
- hmac + sha2 - HMAC-SHA256 签名

**前端 (React)**
- React 19 + TypeScript
- TailwindCSS v4 - 样式框架
- @tauri-apps/api - Tauri IPC
- @tauri-apps/plugin-clipboard-manager - 剪贴板
- @tauri-apps/plugin-global-shortcut - 全局快捷键

**ASR 服务**
- 豆包语音识别 2.0 (Volcengine BigModel)
- Seed 二进制协议 (WebSocket)

## 项目结构

```
src/
├── main.tsx      # React 入口
├── App.tsx       # 主组件
└── style.css     # TailwindCSS + 主题变量

src-tauri/        # Rust 后端
```

## 开发命令

```bash
npm run dev       # 启动 Vite 开发服务器
npm run build     # TypeScript 检查 + 生产构建
npm run tauri dev # 启动 Tauri 开发模式
```

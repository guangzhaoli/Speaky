# Speaky

A cross-platform voice input application that converts speech to text in real-time. Press a hotkey, speak, and your words are automatically typed at the cursor position.

![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue)
![License](https://img.shields.io/badge/license-MIT-green)

[English](README.md) | [中文](README_ZH.md)

## Features

- **Global Hotkey** - Press `Alt+Space` (or custom shortcut) to start/stop recording from anywhere
- **Real-time Transcription** - See your speech converted to text instantly
- **Auto-paste** - Transcribed text is automatically typed at cursor position
- **LLM Post-processing** - Optional AI polish to add punctuation, fix errors, and improve formatting
- **Multi-provider Support** - Works with DeepSeek, OpenAI, Kimi, Gemini, Zhipu, Ollama, and any OpenAI-compatible API
- **Recording Indicator** - Visual indicator shows when recording is active
- **History** - Browse and copy previous transcriptions
- **Dark/Light Theme** - Beautiful UI with theme support
- **Cross-platform** - Works on Windows, macOS, and Linux

## Screenshots

<!-- Add screenshots here -->

## Installation

### Pre-built Binaries

Download the latest release for your platform from the [Releases](https://github.com/guangzhaoli/Speaky/releases) page.

- **Windows**: Download `.msi` or `.exe` installer
- **macOS**: Download `.dmg` file
- **Linux**: Download `.AppImage` or `.deb` package

### Build from Source

Prerequisites:
- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/) 1.70+
- [Tauri CLI](https://tauri.app/)

```bash
# Clone the repository
git clone https://github.com/guangzhaoli/Speaky.git
cd Speaky

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Configuration

### ASR (Speech Recognition)

Speaky uses [Volcengine Doubao ASR](https://www.volcengine.com/product/speech-recognition) for speech recognition. You'll need to:

1. Create an account at [Volcengine Console](https://console.volcengine.com/)
2. Enable the Speech Recognition service
3. Create an application and get your credentials:
   - App ID
   - Access Token
   - Secret Key (optional)

Enter these in Settings > General > API Configuration.

### LLM Post-processing (Optional)

Enable AI-powered text polish in Settings > LLM Polish:

1. Toggle "Enable LLM Post-Processing"
2. Choose a processing mode:
   - **General** - For everyday text input
   - **Code** - Preserves technical terms and syntax
   - **Meeting** - Formal writing style
3. Add and configure an API provider

Supported providers:
- DeepSeek
- OpenAI
- Kimi (Moonshot)
- Google Gemini
- Zhipu (GLM)
- Ollama (Local)
- Any OpenAI-compatible API

## Usage

1. **Start Recording**: Press the global hotkey (default: `Alt+Space`) or click the microphone button
2. **Speak**: Talk naturally - your speech will be transcribed in real-time
3. **Stop Recording**: Release the hotkey or click the button again
4. **Auto-paste**: The transcribed text is automatically typed at your cursor position

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Alt+Space` | Start/Stop recording (customizable) |

## Tech Stack

- **Framework**: [Tauri v2](https://tauri.app/) (Rust + React)
- **Frontend**: React 19 + TypeScript + TailwindCSS v4
- **Audio**: [cpal](https://github.com/RustAudio/cpal) (cross-platform audio capture)
- **Keyboard**: [enigo](https://github.com/enigo-rs/enigo) (keyboard simulation)
- **ASR**: Volcengine Doubao ASR 2.0 (WebSocket binary protocol)
- **LLM**: OpenAI-compatible API

## Project Structure

```
speaky/
├── src/                    # React frontend
│   ├── App.tsx            # Main application component
│   ├── main.tsx           # React entry point
│   └── style.css          # TailwindCSS styles
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── lib.rs         # Tauri application setup
│   │   ├── commands.rs    # IPC commands
│   │   ├── state.rs       # Application state
│   │   ├── audio/         # Audio capture
│   │   ├── asr/           # Speech recognition
│   │   ├── input/         # Keyboard simulation
│   │   └── postprocess/   # LLM post-processing
│   └── Cargo.toml         # Rust dependencies
├── package.json           # Node.js dependencies
└── tauri.conf.json        # Tauri configuration
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [Tauri](https://tauri.app/) for the amazing framework
- [Volcengine](https://www.volcengine.com/) for the ASR service

// 应用类型定义

export type RecordingState = "idle" | "recording" | "processing";
export type SettingsTab = "general" | "asr" | "postprocess" | "history" | "config" | "logs";
export type ViewMode = "main" | "settings";
export type AsrProviderType = "doubao" | "whisper_local" | "whisper_api";
export type PostProcessMode = "General" | "Code" | "Meeting";

export interface WindowSizes {
  main: { width: number; height: number };
  settings: { width: number; height: number };
}

export interface Config {
  app_id: string;
  access_token: string;
  secret_key: string;
  shortcut: string;
  auto_type: boolean;
  auto_copy: boolean;
  auto_start: boolean;
  silent_start: boolean;
  show_indicator: boolean;
  realtime_input: boolean;
  postprocess: PostProcessConfig;
  audio_device: string;
  asr: AsrConfig;
  asr_language: string;
}

export interface AsrConfig {
  active_provider: AsrProviderType;
  doubao?: DoubaoConfig;
  whisper_local?: WhisperLocalConfig;
  whisper_api?: WhisperApiConfig;
}

export interface DoubaoConfig {
  app_id: string;
  access_token: string;
  secret_key: string;
}

export interface WhisperLocalConfig {
  model_size: string;
  language: string;
  translate_to_english: boolean;
}

export interface WhisperApiConfig {
  api_key: string;
  api_base: string;
  model: string;
  language?: string;
}

export interface WhisperModel {
  id: string;
  name: string;
  size_bytes: number;
  is_downloaded: boolean;
  is_selected: boolean;
}

export interface DownloadProgress {
  model_id: string;
  downloaded_bytes: number;
  total_bytes: number;
  percent: number;
}

export interface LlmProvider {
  id: string;
  name: string;
  api_base: string;
  api_key: string;
  model: string;
}

export interface PostProcessConfig {
  enabled: boolean;
  providers: LlmProvider[];
  active_provider_id: string;
  mode: PostProcessMode;
}

export interface HistoryEntry {
  id: string;
  text: string;
  timestamp: string;
}

export interface AudioDevice {
  name: string;
  is_default: boolean;
}

export interface LogInfo {
  path: string;
  size: number;
  enabled: boolean;
}

export interface Toast {
  id: number;
  message: string;
  type: "success" | "error" | "info";
}

// Provider 预设
export interface ProviderPreset {
  name: string;
  api_base: string;
  models: string[];
  default_model: string;
}

export const PROVIDER_PRESETS: Record<string, ProviderPreset> = {
  deepseek: {
    name: "DeepSeek",
    api_base: "https://api.deepseek.com/v1",
    models: ["deepseek-chat", "deepseek-reasoner"],
    default_model: "deepseek-chat",
  },
  openai: {
    name: "OpenAI",
    api_base: "https://api.openai.com/v1",
    models: ["gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "gpt-3.5-turbo"],
    default_model: "gpt-4o-mini",
  },
  kimi: {
    name: "Kimi (Moonshot)",
    api_base: "https://api.moonshot.cn/v1",
    models: ["moonshot-v1-8k", "moonshot-v1-32k", "moonshot-v1-128k"],
    default_model: "moonshot-v1-8k",
  },
  gemini: {
    name: "Gemini (Google)",
    api_base: "https://generativelanguage.googleapis.com/v1beta/openai",
    models: ["gemini-2.0-flash", "gemini-1.5-pro", "gemini-1.5-flash"],
    default_model: "gemini-2.0-flash",
  },
  zhipu: {
    name: "智谱 (GLM)",
    api_base: "https://open.bigmodel.cn/api/paas/v4",
    models: ["glm-4-flash", "glm-4-plus", "glm-4"],
    default_model: "glm-4-flash",
  },
  ollama: {
    name: "Ollama (Local)",
    api_base: "http://localhost:11434/v1",
    models: ["llama3", "qwen2", "mistral"],
    default_model: "llama3",
  },
};

// 默认配置
export const DEFAULT_SHORTCUT = "Alt+Space";

// 计算窗口尺寸（基于屏幕分辨率百分比）
export const calculateWindowSizes = (screenWidth: number, screenHeight: number): WindowSizes => ({
  main: {
    width: Math.max(260, Math.round(screenWidth * 0.12)),
    height: Math.max(280, Math.round(screenHeight * 0.18)),
  },
  settings: {
    width: Math.max(520, Math.round(screenWidth * 0.30)),
    height: Math.max(380, Math.round(screenHeight * 0.32)),
  },
});

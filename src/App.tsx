import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow, LogicalSize, availableMonitors } from "@tauri-apps/api/window";

// 默认快捷键
const DEFAULT_SHORTCUT = "Alt+Space";

// 计算窗口尺寸（基于屏幕分辨率百分比）
const calculateWindowSizes = (screenWidth: number, screenHeight: number) => {
  return {
    main: {
      width: Math.max(260, Math.round(screenWidth * 0.12)),
      height: Math.max(280, Math.round(screenHeight * 0.18)),
    },
    settings: {
      width: Math.max(520, Math.round(screenWidth * 0.30)),
      height: Math.max(380, Math.round(screenHeight * 0.32)),
    },
  };
};

// Icons
const MicIcon = () => (
  <svg width="28" height="28" viewBox="0 0 24 24" fill="currentColor">
    <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3z" />
    <path d="M17 11c0 2.76-2.24 5-5 5s-5-2.24-5-5H5c0 3.53 2.61 6.43 6 6.92V21h2v-3.08c3.39-.49 6-3.39 6-6.92h-2z" />
  </svg>
);

const SunIcon = () => (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <circle cx="12" cy="12" r="4" />
    <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41" />
  </svg>
);

const MoonIcon = () => (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
  </svg>
);

const SettingsIcon = () => (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
  </svg>
);

const ChevronLeftIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <polyline points="15 18 9 12 15 6" />
  </svg>
);

const ChevronDownIcon = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <polyline points="6 9 12 15 18 9" />
  </svg>
);

const ChevronUpIcon = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <polyline points="18 15 12 9 6 15" />
  </svg>
);

const CloseIcon = () => (
  <svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor">
    <path d="M1.5 1.5l7 7M8.5 1.5l-7 7" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
  </svg>
);

// Settings Icons
const GeneralIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
  </svg>
);

const PostProcessIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <path d="M12 2L2 7l10 5 10-5-10-5z" />
    <path d="M2 17l10 5 10-5" />
    <path d="M2 12l10 5 10-5" />
  </svg>
);

const HistoryIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <path d="M12 8v4l3 3" />
    <circle cx="12" cy="12" r="10" />
  </svg>
);

const ConfigFileIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
    <polyline points="14 2 14 8 20 8" />
    <line x1="16" y1="13" x2="8" y2="13" />
    <line x1="16" y1="17" x2="8" y2="17" />
    <polyline points="10 9 9 9 8 9" />
  </svg>
);

const TrashIcon = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <polyline points="3 6 5 6 21 6" />
    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
  </svg>
);

const CopyIcon = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
  </svg>
);

const LogsIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
    <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
    <line x1="8" y1="7" x2="16" y2="7" />
    <line x1="8" y1="11" x2="16" y2="11" />
    <line x1="8" y1="15" x2="12" y2="15" />
  </svg>
);

type RecordingState = "idle" | "recording" | "processing";
type SettingsTab = "general" | "postprocess" | "history" | "config" | "logs";
type ViewMode = "main" | "settings";

interface WindowSizes {
  main: { width: number; height: number };
  settings: { width: number; height: number };
}

interface Config {
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
}

interface AudioDevice {
  name: string;
  is_default: boolean;
}

interface HistoryEntry {
  id: string;
  text: string;
  timestamp: string;
}

interface LogInfo {
  path: string;
  size: number;
  enabled: boolean;
}

interface LlmProvider {
  id: string;
  name: string;
  api_base: string;
  api_key: string;
  model: string;
}

type PostProcessMode = "General" | "Code" | "Meeting";

interface PostProcessConfig {
  enabled: boolean;
  providers: LlmProvider[];
  active_provider_id: string;
  mode: PostProcessMode;
}

interface Toast {
  id: number;
  message: string;
  type: "error" | "success" | "info";
}

// 预设 Provider 模板
interface ProviderPreset {
  name: string;
  api_base: string;
  models: string[];
  default_model: string;
}

const PROVIDER_PRESETS: Record<string, ProviderPreset> = {
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

// 设置类别配置
const settingsTabs: { id: SettingsTab; label: string; icon: React.ReactNode }[] = [
  { id: "general", label: "General", icon: <GeneralIcon /> },
  { id: "postprocess", label: "LLM Polish", icon: <PostProcessIcon /> },
  { id: "history", label: "History", icon: <HistoryIcon /> },
  { id: "logs", label: "Logs", icon: <LogsIcon /> },
  { id: "config", label: "Config File", icon: <ConfigFileIcon /> },
];

export default function App() {
  const [state, setState] = useState<RecordingState>("idle");
  const [transcript, setTranscript] = useState("");
  const [viewMode, setViewMode] = useState<ViewMode>("main");
  const [isAnimating, setIsAnimating] = useState(false);
  const [settingsTab, setSettingsTab] = useState<SettingsTab>("general");
  const [isDark, setIsDark] = useState(false);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [windowSizes, setWindowSizes] = useState<WindowSizes>({
    main: { width: 260, height: 280 },
    settings: { width: 520, height: 380 },
  });
  const [config, setConfig] = useState<Config>({
    app_id: "",
    access_token: "",
    secret_key: "",
    shortcut: DEFAULT_SHORTCUT,
    auto_type: true,
    auto_copy: true,
    auto_start: false,
    silent_start: false,
    show_indicator: true,
    realtime_input: false,
    postprocess: {
      enabled: false,
      providers: [{
        id: "default",
        name: "DeepSeek",
        api_base: "https://api.deepseek.com/v1",
        api_key: "",
        model: "deepseek-chat",
      }],
      active_provider_id: "default",
      mode: "General",
    },
    audio_device: "",
  });
  const [isRecordingShortcut, setIsRecordingShortcut] = useState(false);
  const animationFrameRef = useRef<number | null>(null);

  // 音频设备列表
  const [audioDevices, setAudioDevices] = useState<AudioDevice[]>([]);

  // 历史记录
  const [historyEntries, setHistoryEntries] = useState<HistoryEntry[]>([]);

  // 配置文件内容
  const [configFileContent, setConfigFileContent] = useState("");
  const [configFilePath, setConfigFilePath] = useState("");
  const [configFileModified, setConfigFileModified] = useState(false);

  // 日志相关
  const [logInfo, setLogInfo] = useState<LogInfo>({ path: "", size: 0, enabled: true });
  const [logEntries, setLogEntries] = useState<string[]>([]);
  const logContainerRef = useRef<HTMLDivElement>(null);

  // 初始化窗口尺寸（基于屏幕分辨率）
  useEffect(() => {
    const initWindowSize = async () => {
      try {
        const monitors = await availableMonitors();
        if (monitors.length > 0) {
          const primary = monitors[0];
          const sizes = calculateWindowSizes(primary.size.width, primary.size.height);
          setWindowSizes(sizes);
          // 设置初始窗口大小
          const win = getCurrentWindow();
          await win.setSize(new LogicalSize(sizes.main.width, sizes.main.height));
        }
      } catch (e) {
        console.error("Failed to get monitor info:", e);
      }
    };
    initWindowSize();
  }, []);

  const isRecording = state === "recording";
  const isProcessing = state === "processing";
  const showSettings = viewMode === "settings";

  // 使用 useMemo 缓存设置标签配置，避免每次渲染都创建新对象
  const currentTab = useMemo(
    () => settingsTabs.find((t) => t.id === settingsTab),
    [settingsTab]
  );

  // 检测是否为 macOS（用于显示正确的快捷键提示）
  const isMacOS = useMemo(() => {
    return typeof navigator !== 'undefined' && /Mac|iPhone|iPad|iPod/.test(navigator.platform);
  }, []);

  const statusText = isRecording
    ? "正在聆听..."
    : isProcessing
      ? "识别中..."
      : isMacOS
        ? "按住 ⌥ Space 开始"
        : "按住 Alt+Space 开始";

  // 简洁Q弹缓动函数 (Ease Out Back - 轻微过冲)
  const springEase = (t: number): number => {
    const c1 = 1.70158;
    const c3 = c1 + 1;
    return 1 + c3 * Math.pow(t - 1, 3) + c1 * Math.pow(t - 1, 2);
  };

  // 窗口边缘/角落拖动调整大小
  type ResizeDirection =
    | "North" | "South" | "East" | "West"
    | "NorthEast" | "NorthWest" | "SouthEast" | "SouthWest";

  const handleResizeStart = useCallback(async (direction: ResizeDirection) => {
    const win = getCurrentWindow();
    try {
      await win.startResizeDragging(direction);
    } catch (e) {
      console.error("Failed to start resize dragging:", e);
    }
  }, []);

  // 窗口大小动画切换
  const animateWindowSize = useCallback(async (targetMode: ViewMode) => {
    if (isAnimating) return;

    const win = getCurrentWindow();
    const from = windowSizes[viewMode];
    const to = windowSizes[targetMode];

    setIsAnimating(true);
    setViewMode(targetMode);

    const duration = 300; // 更短更干脆
    const startTime = performance.now();

    const animate = async (currentTime: number) => {
      const elapsed = currentTime - startTime;
      const progress = Math.min(elapsed / duration, 1);
      const eased = springEase(progress);

      const currentWidth = Math.round(from.width + (to.width - from.width) * eased);
      const currentHeight = Math.round(from.height + (to.height - from.height) * eased);

      try {
        await win.setSize(new LogicalSize(currentWidth, currentHeight));
      } catch (e) {
        console.error("Failed to set window size:", e);
      }

      if (progress < 1) {
        animationFrameRef.current = requestAnimationFrame(animate);
      } else {
        setIsAnimating(false);
        animationFrameRef.current = null;
      }
    };

    animationFrameRef.current = requestAnimationFrame(animate);
  }, [isAnimating, viewMode, windowSizes]);

  // 清理动画帧
  useEffect(() => {
    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
    };
  }, []);

  // 切换视图模式
  const toggleViewMode = useCallback(() => {
    const newMode = viewMode === "main" ? "settings" : "main";
    animateWindowSize(newMode);
  }, [viewMode, animateWindowSize]);

  // Toast 管理
  const showToast = useCallback((message: string, type: Toast["type"] = "error") => {
    const id = Date.now();
    setToasts((prev) => [...prev, { id, message, type }]);
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 4000);
  }, []);

  const dismissToast = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  // 窗口控制
  const handleClose = async () => {
    const window = getCurrentWindow();
    await window.hide();
  };

  const handleMinimize = async () => {
    const window = getCurrentWindow();
    await window.minimize();
  };

  // 窗口拖动
  const handleDragStart = async (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest('button')) return;
    const window = getCurrentWindow();
    await window.startDragging();
  };

  // 主题初始化
  useEffect(() => {
    const saved = localStorage.getItem("theme");
    if (saved) {
      setIsDark(saved === "dark");
    } else {
      setIsDark(window.matchMedia("(prefers-color-scheme: dark)").matches);
    }
  }, []);

  useEffect(() => {
    if (isDark) {
      document.documentElement.classList.add("dark");
    } else {
      document.documentElement.classList.remove("dark");
    }
  }, [isDark]);

  const toggleTheme = () => {
    const newValue = !isDark;
    setIsDark(newValue);
    localStorage.setItem("theme", newValue ? "dark" : "light");
  };

  // 加载配置和事件监听
  useEffect(() => {
    let unlistenStarted: UnlistenFn | null = null;
    let unlistenStopped: UnlistenFn | null = null;
    let unlistenUpdate: UnlistenFn | null = null;
    let unlistenError: UnlistenFn | null = null;

    const setup = async () => {
      try {
        const savedConfig = await invoke("get_config");
        setConfig(savedConfig as Config);
      } catch (e) {
        console.error("Failed to load config:", e);
        showToast("加载配置失败");
      }

      // 加载音频设备列表
      try {
        const devices = await invoke("get_audio_devices");
        setAudioDevices(devices as AudioDevice[]);
      } catch (e) {
        console.error("Failed to load audio devices:", e);
      }

      unlistenStarted = await listen("recording-started", () => {
        setState("recording");
        setTranscript("");
      });

      unlistenStopped = await listen("recording-stopped", (event) => {
        setState("idle");
        setTranscript(event.payload as string);
      });

      unlistenUpdate = await listen("transcript-update", (event) => {
        setTranscript(event.payload as string);
      });

      unlistenError = await listen("error", (event) => {
        showToast(event.payload as string);
      });
    };

    setup();

    return () => {
      unlistenStarted?.();
      unlistenStopped?.();
      unlistenUpdate?.();
      unlistenError?.();
    };
  }, [showToast]);

  // 当切换到历史记录标签时加载历史
  useEffect(() => {
    if (settingsTab === "history") {
      loadHistory();
    }
  }, [settingsTab]);

  // 当切换到配置文件标签时加载配置文件
  useEffect(() => {
    if (settingsTab === "config") {
      loadConfigFile();
    }
  }, [settingsTab]);

  // 当切换到日志标签时加载日志
  useEffect(() => {
    if (settingsTab === "logs") {
      loadLogs();
    }
  }, [settingsTab]);

  const loadHistory = async () => {
    try {
      const entries = await invoke("get_history");
      setHistoryEntries(entries as HistoryEntry[]);
    } catch (e) {
      console.error("Failed to load history:", e);
    }
  };

  const loadConfigFile = async () => {
    try {
      const [path, content] = await Promise.all([
        invoke("get_config_file_path"),
        invoke("get_config_file_content"),
      ]);
      setConfigFilePath(path as string);
      setConfigFileContent(content as string);
      setConfigFileModified(false);
    } catch (e) {
      console.error("Failed to load config file:", e);
    }
  };

  const deleteHistoryEntry = async (id: string) => {
    try {
      await invoke("delete_history_entry", { id });
      setHistoryEntries((prev) => prev.filter((e) => e.id !== id));
      showToast("已删除", "success");
    } catch (e) {
      showToast(String(e));
    }
  };

  const clearHistory = async () => {
    try {
      await invoke("clear_history");
      setHistoryEntries([]);
      showToast("历史记录已清空", "success");
    } catch (e) {
      showToast(String(e));
    }
  };

  const saveConfigFile = async () => {
    try {
      await invoke("save_config_file_content", { content: configFileContent });
      setConfigFileModified(false);
      // 重新加载配置到 UI
      const savedConfig = await invoke("get_config");
      setConfig(savedConfig as Config);
      showToast("配置已保存", "success");
    } catch (e) {
      showToast(String(e));
    }
  };

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      showToast("已复制", "success");
    } catch (e) {
      showToast("复制失败");
    }
  };

  const loadLogs = async () => {
    try {
      const [info, entries] = await Promise.all([
        invoke("get_log_info"),
        invoke("get_logs", { maxLines: 500 }),
      ]);
      setLogInfo(info as LogInfo);
      setLogEntries(entries as string[]);
      // 滚动到底部
      setTimeout(() => {
        if (logContainerRef.current) {
          logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight;
        }
      }, 100);
    } catch (e) {
      console.error("Failed to load logs:", e);
    }
  };

  const clearLogs = async () => {
    try {
      await invoke("clear_logs");
      setLogEntries([]);
      const info = await invoke("get_log_info");
      setLogInfo(info as LogInfo);
      showToast("日志已清空", "success");
    } catch (e) {
      showToast(String(e));
    }
  };

  const toggleLogging = async (enabled: boolean) => {
    try {
      await invoke("set_logging_enabled", { enabled });
      setLogInfo((prev) => ({ ...prev, enabled }));
      showToast(enabled ? "日志已启用" : "日志已禁用", "success");
    } catch (e) {
      showToast(String(e));
    }
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  };

  const startRecording = useCallback(async () => {
    if (state !== "idle") return;
    try {
      await invoke("start_recording");
    } catch (e) {
      console.error("Failed to start recording:", e);
      showToast(String(e));
      setState("idle");
    }
  }, [state, showToast]);

  const stopRecording = useCallback(async () => {
    if (state !== "recording") return;
    setState("processing");
    try {
      await invoke("stop_recording");
    } catch (e) {
      console.error("Failed to stop recording:", e);
      showToast(String(e));
      setState("idle");
    }
  }, [state, showToast]);

  const saveConfig = async () => {
    try {
      await invoke("update_config", { config });
      showToast("设置已保存", "success");
    } catch (e) {
      console.error("Failed to save config:", e);
      showToast(String(e));
    }
  };

  const updateConfig = (key: keyof Config, value: string | boolean) => {
    setConfig((prev) => ({ ...prev, [key]: value }));
  };

  // 快捷键录入
  const handleShortcutKeyDown = (e: React.KeyboardEvent) => {
    e.preventDefault();
    e.stopPropagation();

    const parts: string[] = [];

    if (e.ctrlKey) parts.push("Ctrl");
    if (e.altKey) parts.push(isMacOS ? "Option" : "Alt");
    if (e.shiftKey) parts.push("Shift");
    if (e.metaKey) parts.push(isMacOS ? "Cmd" : "Super");

    const key = e.key;
    if (!["Control", "Alt", "Shift", "Meta"].includes(key)) {
      // 格式化按键名称
      let keyName = key;
      if (key === " ") keyName = "Space";
      else if (key.length === 1) keyName = key.toUpperCase();
      else if (key === "ArrowUp") keyName = "Up";
      else if (key === "ArrowDown") keyName = "Down";
      else if (key === "ArrowLeft") keyName = "Left";
      else if (key === "ArrowRight") keyName = "Right";

      parts.push(keyName);

      if (parts.length > 0) {
        const shortcut = parts.join("+");
        updateConfig("shortcut", shortcut);
        setIsRecordingShortcut(false);
      }
    }
  };

  // General 设置内容
  const renderGeneralSettings = () => (
    <div className="space-y-6">
      {/* 快捷键设置 */}
      <div className="space-y-3">
        <h3 className="text-xs font-medium text-text-muted uppercase tracking-wider">
          Shortcut
        </h3>
        <div className="bg-bg-secondary rounded-xl border border-border-light overflow-hidden">
          <div className="p-4">
            <label className="block text-sm text-text-primary mb-2">Press to Talk Shortcut</label>
            <div className="flex gap-2">
              <div
                tabIndex={0}
                className={`flex-1 px-3 py-2.5 text-sm rounded-lg border transition-all cursor-pointer ${
                  isRecordingShortcut
                    ? "bg-accent/10 border-accent text-accent ring-2 ring-accent/30"
                    : "bg-bg-input border-border text-text-primary hover:border-accent/50"
                }`}
                onClick={() => setIsRecordingShortcut(true)}
                onKeyDown={isRecordingShortcut ? handleShortcutKeyDown : undefined}
                onBlur={() => setIsRecordingShortcut(false)}
              >
                {isRecordingShortcut ? "Press keys..." : config.shortcut}
              </div>
              {isRecordingShortcut ? (
                <button
                  onClick={() => setIsRecordingShortcut(false)}
                  className="px-3 py-2 text-sm text-text-muted hover:text-text-primary transition-colors"
                >
                  Cancel
                </button>
              ) : (
                config.shortcut !== DEFAULT_SHORTCUT && (
                  <button
                    onClick={() => updateConfig("shortcut", DEFAULT_SHORTCUT)}
                    className="px-3 py-2 text-sm text-text-muted hover:text-text-primary transition-colors"
                    title="Reset to default"
                  >
                    Reset
                  </button>
                )
              )}
            </div>
            <p className="text-xs text-text-muted mt-2">
              Click and press your desired key combination. Default: {DEFAULT_SHORTCUT}
            </p>
          </div>
        </div>
      </div>

      {/* 麦克风设置 */}
      <div className="space-y-3">
        <h3 className="text-xs font-medium text-text-muted uppercase tracking-wider">
          Audio Input
        </h3>
        <div className="bg-bg-secondary rounded-xl border border-border-light overflow-hidden">
          <div className="p-4">
            <label className="block text-sm text-text-primary mb-2">Microphone</label>
            <select
              value={config.audio_device}
              onChange={(e) => updateConfig("audio_device", e.target.value)}
              className="w-full px-3 py-2.5 text-sm border border-border rounded-lg focus:outline-none focus:border-accent transition-colors bg-bg-input text-text-primary"
              style={{ colorScheme: 'dark' }}
            >
              {audioDevices.map((device, index) => (
                <option
                  key={index}
                  value={device.name}
                  className="bg-bg-secondary text-text-primary"
                >
                  {device.name === "" ? "System Default" : device.name}
                  {device.is_default && device.name !== "" ? " (Default)" : ""}
                </option>
              ))}
            </select>
            <p className="text-xs text-text-muted mt-2">
              Select the microphone to use for recording
            </p>
          </div>
        </div>
      </div>

      {/* 启动设置 */}
      <div className="space-y-3">
        <h3 className="text-xs font-medium text-text-muted uppercase tracking-wider">
          Startup
        </h3>
        <div className="bg-bg-secondary rounded-xl border border-border-light overflow-hidden">
          <label className="flex items-center justify-between p-4 border-b border-border-light cursor-pointer hover:bg-bg-tertiary transition-colors">
            <div>
              <span className="text-sm text-text-primary font-medium">Launch at Login</span>
              <p className="text-xs text-text-muted mt-1">Start Speaky when you log in</p>
            </div>
            <div className="relative shrink-0 ml-4">
              <input
                type="checkbox"
                checked={config.auto_start}
                onChange={(e) => updateConfig("auto_start", e.target.checked)}
                className="sr-only peer"
              />
              <div className="w-11 h-6 bg-bg-tertiary rounded-full peer peer-checked:bg-accent transition-colors" />
              <div className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow-sm transition-transform peer-checked:translate-x-5" />
            </div>
          </label>
          <label className={`flex items-center justify-between p-4 cursor-pointer transition-colors ${
            config.auto_start ? "hover:bg-bg-tertiary" : "opacity-50 cursor-not-allowed"
          }`}>
            <div>
              <span className="text-sm text-text-primary font-medium">Start Minimized</span>
              <p className="text-xs text-text-muted mt-1">Hide window on startup, run in background</p>
            </div>
            <div className="relative shrink-0 ml-4">
              <input
                type="checkbox"
                checked={config.silent_start}
                onChange={(e) => updateConfig("silent_start", e.target.checked)}
                disabled={!config.auto_start}
                className="sr-only peer"
              />
              <div className="w-11 h-6 bg-bg-tertiary rounded-full peer peer-checked:bg-accent transition-colors" />
              <div className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow-sm transition-transform peer-checked:translate-x-5" />
            </div>
          </label>
        </div>
      </div>

      {/* API 配置区块 */}
      <div className="space-y-3">
        <h3 className="text-xs font-medium text-text-muted uppercase tracking-wider">
          API Configuration
        </h3>
        <div className="bg-bg-secondary rounded-xl border border-border-light overflow-hidden">
          <div className="p-4 border-b border-border-light">
            <label className="block text-sm text-text-primary mb-2">App ID</label>
            <input
              type="text"
              value={config.app_id}
              onChange={(e) => updateConfig("app_id", e.target.value)}
              placeholder="Enter Doubao App ID"
              className="w-full px-3 py-2.5 text-sm bg-bg-input border border-border rounded-lg text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent focus:ring-1 focus:ring-accent/30 transition-all"
            />
          </div>
          <div className="p-4 border-b border-border-light">
            <label className="block text-sm text-text-primary mb-2">Access Token</label>
            <input
              type="password"
              value={config.access_token}
              onChange={(e) => updateConfig("access_token", e.target.value)}
              placeholder="Enter Access Token"
              className="w-full px-3 py-2.5 text-sm bg-bg-input border border-border rounded-lg text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent focus:ring-1 focus:ring-accent/30 transition-all"
            />
          </div>
          <div className="p-4">
            <label className="block text-sm text-text-primary mb-2">Secret Key (Optional)</label>
            <input
              type="password"
              value={config.secret_key}
              onChange={(e) => updateConfig("secret_key", e.target.value)}
              placeholder="Enter Secret Key"
              className="w-full px-3 py-2.5 text-sm bg-bg-input border border-border rounded-lg text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent focus:ring-1 focus:ring-accent/30 transition-all"
            />
          </div>
        </div>
      </div>

      {/* 行为设置区块 */}
      <div className="space-y-3">
        <h3 className="text-xs font-medium text-text-muted uppercase tracking-wider">
          Behavior
        </h3>
        <div className="bg-bg-secondary rounded-xl border border-border-light overflow-hidden">
          <label className="flex items-center justify-between p-4 border-b border-border-light cursor-pointer hover:bg-bg-tertiary transition-colors">
            <div>
              <span className="text-sm text-text-primary font-medium">Auto Copy</span>
              <p className="text-xs text-text-muted mt-1">Copy transcription to clipboard automatically</p>
            </div>
            <div className="relative shrink-0 ml-4">
              <input
                type="checkbox"
                checked={config.auto_copy}
                onChange={(e) => updateConfig("auto_copy", e.target.checked)}
                className="sr-only peer"
              />
              <div className="w-11 h-6 bg-bg-tertiary rounded-full peer peer-checked:bg-accent transition-colors" />
              <div className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow-sm transition-transform peer-checked:translate-x-5" />
            </div>
          </label>
          <label className="flex items-center justify-between p-4 cursor-pointer hover:bg-bg-tertiary transition-colors border-b border-border-light">
            <div>
              <span className="text-sm text-text-primary font-medium">Auto Paste</span>
              <p className="text-xs text-text-muted mt-1">Paste transcription at cursor position</p>
            </div>
            <div className="relative shrink-0 ml-4">
              <input
                type="checkbox"
                checked={config.auto_type}
                onChange={(e) => updateConfig("auto_type", e.target.checked)}
                className="sr-only peer"
              />
              <div className="w-11 h-6 bg-bg-tertiary rounded-full peer peer-checked:bg-accent transition-colors" />
              <div className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow-sm transition-transform peer-checked:translate-x-5" />
            </div>
          </label>
          <label className={`flex items-center justify-between p-4 cursor-pointer transition-colors border-b border-border-light ${
            config.auto_type ? "hover:bg-bg-tertiary" : "opacity-50 cursor-not-allowed"
          }`}>
            <div>
              <span className="text-sm text-text-primary font-medium">Realtime Input</span>
              <p className="text-xs text-text-muted mt-1">Type text while speaking (experimental)</p>
            </div>
            <div className="relative shrink-0 ml-4">
              <input
                type="checkbox"
                checked={config.realtime_input}
                onChange={(e) => updateConfig("realtime_input", e.target.checked)}
                disabled={!config.auto_type}
                className="sr-only peer"
              />
              <div className="w-11 h-6 bg-bg-tertiary rounded-full peer peer-checked:bg-accent transition-colors" />
              <div className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow-sm transition-transform peer-checked:translate-x-5" />
            </div>
          </label>
          <label className="flex items-center justify-between p-4 cursor-pointer hover:bg-bg-tertiary transition-colors">
            <div>
              <span className="text-sm text-text-primary font-medium">Show Indicator</span>
              <p className="text-xs text-text-muted mt-1">Show recording indicator at screen bottom</p>
            </div>
            <div className="relative shrink-0 ml-4">
              <input
                type="checkbox"
                checked={config.show_indicator}
                onChange={(e) => updateConfig("show_indicator", e.target.checked)}
                className="sr-only peer"
              />
              <div className="w-11 h-6 bg-bg-tertiary rounded-full peer peer-checked:bg-accent transition-colors" />
              <div className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow-sm transition-transform peer-checked:translate-x-5" />
            </div>
          </label>
        </div>
      </div>

      {/* 外观设置 */}
      <div className="space-y-3">
        <h3 className="text-xs font-medium text-text-muted uppercase tracking-wider">
          Appearance
        </h3>
        <div className="bg-bg-secondary rounded-xl border border-border-light overflow-hidden">
          <label className="flex items-center justify-between p-4 cursor-pointer hover:bg-bg-tertiary transition-colors">
            <div>
              <span className="text-sm text-text-primary font-medium">Dark Mode</span>
              <p className="text-xs text-text-muted mt-1">Use dark color theme</p>
            </div>
            <div className="relative shrink-0 ml-4">
              <input
                type="checkbox"
                checked={isDark}
                onChange={toggleTheme}
                className="sr-only peer"
              />
              <div className="w-11 h-6 bg-bg-tertiary rounded-full peer peer-checked:bg-accent transition-colors" />
              <div className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow-sm transition-transform peer-checked:translate-x-5" />
            </div>
          </label>
        </div>
      </div>
    </div>
  );

  // Post-Process 设置内容
  const [showAddPreset, setShowAddPreset] = useState(false);
  const [collapsedProviders, setCollapsedProviders] = useState<Set<string>>(new Set());

  const toggleProviderCollapse = (id: string) => {
    setCollapsedProviders(prev => {
      const newSet = new Set(prev);
      if (newSet.has(id)) {
        newSet.delete(id);
      } else {
        newSet.add(id);
      }
      return newSet;
    });
  };

  const addProviderFromPreset = (presetKey: string) => {
    const preset = PROVIDER_PRESETS[presetKey];
    if (!preset) return;

    const newProvider: LlmProvider = {
      id: `${presetKey}-${Date.now()}`,
      name: preset.name,
      api_base: preset.api_base,
      api_key: "",
      model: preset.default_model,
    };

    setConfig(prev => ({
      ...prev,
      postprocess: {
        ...prev.postprocess,
        providers: [...prev.postprocess.providers, newProvider],
        active_provider_id: prev.postprocess.providers.length === 0 ? newProvider.id : prev.postprocess.active_provider_id,
      }
    }));
    setShowAddPreset(false);
  };

  const updateProvider = (index: number, updates: Partial<LlmProvider>) => {
    const newProviders = [...config.postprocess.providers];
    newProviders[index] = { ...newProviders[index], ...updates };
    setConfig(prev => ({
      ...prev,
      postprocess: { ...prev.postprocess, providers: newProviders }
    }));
  };

  const deleteProvider = (index: number) => {
    const provider = config.postprocess.providers[index];
    const newProviders = config.postprocess.providers.filter((_, i) => i !== index);
    const newActiveId = config.postprocess.active_provider_id === provider.id
      ? newProviders[0]?.id || ""
      : config.postprocess.active_provider_id;
    setConfig(prev => ({
      ...prev,
      postprocess: { ...prev.postprocess, providers: newProviders, active_provider_id: newActiveId }
    }));
  };

  const renderPostProcessSettings = () => (
    <div className="space-y-6">
      {/* 启用开关 */}
      <div className="bg-bg-secondary rounded-xl border border-border-light overflow-hidden">
        <label className="flex items-center justify-between p-4 cursor-pointer hover:bg-bg-tertiary transition-colors">
          <div>
            <span className="text-sm text-text-primary font-medium">Enable LLM Post-Processing</span>
            <p className="text-xs text-text-muted mt-1">Polish transcription with AI (add punctuation, fix errors, remove filler words)</p>
          </div>
          <div className="relative shrink-0 ml-4">
            <input
              type="checkbox"
              checked={config.postprocess.enabled}
              onChange={(e) => setConfig(prev => ({
                ...prev,
                postprocess: { ...prev.postprocess, enabled: e.target.checked }
              }))}
              className="sr-only peer"
            />
            <div className="w-11 h-6 bg-bg-tertiary rounded-full peer peer-checked:bg-accent transition-colors" />
            <div className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow-sm transition-transform peer-checked:translate-x-5" />
          </div>
        </label>
      </div>

      {config.postprocess.enabled && (
        <>
          {/* 处理模式 */}
          <div className="space-y-3">
            <h3 className="text-xs font-medium text-text-muted uppercase tracking-wider">
              Processing Mode
            </h3>
            <div className="flex gap-2">
              {(["General", "Code", "Meeting"] as PostProcessMode[]).map((mode) => (
                <button
                  key={mode}
                  onClick={() => setConfig(prev => ({
                    ...prev,
                    postprocess: { ...prev.postprocess, mode }
                  }))}
                  className={`flex-1 px-4 py-2.5 text-sm rounded-xl transition-all ${
                    config.postprocess.mode === mode
                      ? "bg-accent text-white shadow-sm"
                      : "bg-bg-secondary text-text-secondary hover:text-text-primary border border-border-light hover:border-accent/50"
                  }`}
                >
                  {mode}
                </button>
              ))}
            </div>
            <p className="text-xs text-text-muted">
              {config.postprocess.mode === "General" && "For everyday text input - adds punctuation, removes filler words"}
              {config.postprocess.mode === "Code" && "Preserves technical terms, variable names, and code syntax"}
              {config.postprocess.mode === "Meeting" && "Formal writing style suitable for meeting notes and reports"}
            </p>
          </div>

          {/* API Providers */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <h3 className="text-xs font-medium text-text-muted uppercase tracking-wider">
                API Providers
              </h3>
              <div className="relative">
                <button
                  onClick={() => setShowAddPreset(!showAddPreset)}
                  className="text-xs text-accent hover:text-accent-hover font-medium"
                >
                  + Add Provider
                </button>

                {/* 预设下拉菜单 */}
                {showAddPreset && (
                  <div className="absolute right-0 top-6 z-10 w-48 bg-bg-secondary border border-border-light rounded-xl shadow-lg overflow-hidden">
                    {Object.entries(PROVIDER_PRESETS).map(([key, preset]) => (
                      <button
                        key={key}
                        onClick={() => addProviderFromPreset(key)}
                        className="w-full px-4 py-2.5 text-left text-sm text-text-primary hover:bg-bg-tertiary transition-colors border-b border-border-light last:border-b-0"
                      >
                        {preset.name}
                      </button>
                    ))}
                    <button
                      onClick={() => {
                        const newProvider: LlmProvider = {
                          id: `custom-${Date.now()}`,
                          name: "Custom Provider",
                          api_base: "",
                          api_key: "",
                          model: "",
                        };
                        setConfig(prev => ({
                          ...prev,
                          postprocess: {
                            ...prev.postprocess,
                            providers: [...prev.postprocess.providers, newProvider],
                          }
                        }));
                        setShowAddPreset(false);
                      }}
                      className="w-full px-4 py-2.5 text-left text-sm text-accent hover:bg-bg-tertiary transition-colors"
                    >
                      Custom (OpenAI Format)
                    </button>
                  </div>
                )}
              </div>
            </div>

            {/* Provider 列表 */}
            <div className="space-y-3">
              {config.postprocess.providers.length === 0 ? (
                <div className="p-6 text-center text-text-muted bg-bg-secondary rounded-xl border border-border-light">
                  <p className="text-sm">No providers configured</p>
                  <p className="text-xs mt-1">Click "Add Provider" to get started</p>
                </div>
              ) : (
                config.postprocess.providers.map((provider, index) => {
                  const isActive = config.postprocess.active_provider_id === provider.id;
                  const isCollapsed = collapsedProviders.has(provider.id);
                  // 查找匹配的预设以获取模型列表
                  const matchingPreset = Object.values(PROVIDER_PRESETS).find(
                    p => provider.api_base === p.api_base
                  );
                  // 检查当前模型是否在预设列表中
                  const isCustomModel = matchingPreset && !matchingPreset.models.includes(provider.model);

                  return (
                    <div
                      key={provider.id}
                      className={`rounded-xl border transition-all ${
                        isActive
                          ? "border-accent bg-accent/5 shadow-sm"
                          : "border-border-light bg-bg-secondary hover:border-border"
                      }`}
                    >
                      {/* Provider 头部 */}
                      <div className={`flex items-center justify-between p-4 ${!isCollapsed ? "border-b border-border-light" : ""}`}>
                        <div className="flex items-center gap-3 flex-1">
                          <input
                            type="radio"
                            name="active_provider"
                            checked={isActive}
                            onChange={() => setConfig(prev => ({
                              ...prev,
                              postprocess: { ...prev.postprocess, active_provider_id: provider.id }
                            }))}
                            className="w-4 h-4 text-accent cursor-pointer"
                          />
                          <input
                            type="text"
                            value={provider.name}
                            onChange={(e) => updateProvider(index, { name: e.target.value })}
                            className="text-sm font-medium bg-transparent border-none text-text-primary focus:outline-none flex-1 min-w-0"
                            placeholder="Provider Name"
                          />
                        </div>
                        <div className="flex items-center gap-1">
                          <button
                            onClick={async () => {
                              try {
                                showToast("Testing connection...", "info");
                                await invoke("test_llm_connection", { provider });
                                showToast("Connection successful!", "success");
                              } catch (e) {
                                showToast(`Connection failed: ${e}`, "error");
                              }
                            }}
                            className="text-xs text-accent hover:text-accent-hover px-2 py-1 rounded hover:bg-accent/10 transition-colors"
                          >
                            Test
                          </button>
                          {config.postprocess.providers.length > 1 && (
                            <button
                              onClick={() => deleteProvider(index)}
                              className="text-xs text-red-500 hover:text-red-600 px-2 py-1 rounded hover:bg-red-500/10 transition-colors"
                            >
                              Delete
                            </button>
                          )}
                          <button
                            onClick={() => toggleProviderCollapse(provider.id)}
                            className="p-1.5 text-text-muted hover:text-text-primary hover:bg-bg-tertiary rounded transition-colors"
                            title={isCollapsed ? "Expand" : "Collapse"}
                          >
                            {isCollapsed ? <ChevronDownIcon /> : <ChevronUpIcon />}
                          </button>
                        </div>
                      </div>

                      {/* Provider 配置 - 可折叠 */}
                      {!isCollapsed && (
                        <div className="p-4 space-y-3">
                          <div>
                            <label className="block text-xs text-text-muted mb-1.5">API Base URL</label>
                            <input
                              type="text"
                              value={provider.api_base}
                              onChange={(e) => updateProvider(index, { api_base: e.target.value })}
                              placeholder="https://api.example.com/v1"
                              className="w-full px-3 py-2 text-sm bg-bg-input border border-border rounded-lg text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent transition-colors"
                            />
                          </div>

                          <div>
                            <label className="block text-xs text-text-muted mb-1.5">API Key</label>
                            <input
                              type="password"
                              value={provider.api_key}
                              onChange={(e) => updateProvider(index, { api_key: e.target.value })}
                              placeholder="sk-..."
                              className="w-full px-3 py-2 text-sm bg-bg-input border border-border rounded-lg text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent transition-colors"
                            />
                          </div>

                          <div>
                            <label className="block text-xs text-text-muted mb-1.5">Model</label>
                            {matchingPreset ? (
                              <div className="space-y-2">
                                <select
                                  value={isCustomModel ? "__custom__" : provider.model}
                                  onChange={(e) => {
                                    if (e.target.value === "__custom__") {
                                      // 切换到自定义模式，保持当前模型名
                                      if (!isCustomModel) {
                                        updateProvider(index, { model: "" });
                                      }
                                    } else {
                                      updateProvider(index, { model: e.target.value });
                                    }
                                  }}
                                  className="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:border-accent transition-colors bg-bg-input text-text-primary"
                                  style={{ colorScheme: 'dark' }}
                                >
                                  {matchingPreset.models.map((model) => (
                                    <option key={model} value={model} className="bg-bg-secondary text-text-primary">{model}</option>
                                  ))}
                                  <option value="__custom__" className="bg-bg-secondary text-text-primary">Custom model...</option>
                                </select>
                                {isCustomModel && (
                                  <input
                                    type="text"
                                    value={provider.model}
                                    onChange={(e) => updateProvider(index, { model: e.target.value })}
                                    placeholder="Enter custom model name"
                                    className="w-full px-3 py-2 text-sm bg-bg-input border border-border rounded-lg text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent transition-colors"
                                    autoFocus
                                  />
                                )}
                              </div>
                            ) : (
                              <input
                                type="text"
                                value={provider.model}
                                onChange={(e) => updateProvider(index, { model: e.target.value })}
                                placeholder="model-name"
                                className="w-full px-3 py-2 text-sm bg-bg-input border border-border rounded-lg text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent transition-colors"
                              />
                            )}
                          </div>
                        </div>
                      )}
                    </div>
                  );
                })
              )}
            </div>
          </div>
        </>
      )}
    </div>
  );

  // History 设置内容
  const renderHistorySettings = () => (
    <div className="space-y-4">
      {/* 操作栏 */}
      <div className="flex items-center justify-between">
        <p className="text-sm text-text-muted">
          {historyEntries.length} 条记录
        </p>
        {historyEntries.length > 0 && (
          <button
            onClick={clearHistory}
            className="text-xs text-red-500 hover:text-red-600 px-3 py-1.5 rounded-lg hover:bg-red-500/10 transition-colors"
          >
            Clear All
          </button>
        )}
      </div>

      {/* 历史记录列表 */}
      {historyEntries.length === 0 ? (
        <div className="p-8 text-center text-text-muted bg-bg-secondary rounded-xl border border-border-light">
          <HistoryIcon />
          <p className="text-sm mt-2">No history yet</p>
          <p className="text-xs mt-1">Your transcriptions will appear here</p>
        </div>
      ) : (
        <div className="space-y-2 max-h-[400px] overflow-y-auto">
          {historyEntries.map((entry) => (
            <div
              key={entry.id}
              className="group bg-bg-secondary rounded-xl border border-border-light p-4 hover:border-border transition-colors"
            >
              <div className="flex items-start justify-between gap-3">
                <p className="text-sm text-text-primary flex-1 break-words">
                  {entry.text}
                </p>
                <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
                  <button
                    onClick={() => copyToClipboard(entry.text)}
                    className="p-1.5 text-text-muted hover:text-text-primary hover:bg-bg-tertiary rounded transition-colors"
                    title="Copy"
                  >
                    <CopyIcon />
                  </button>
                  <button
                    onClick={() => deleteHistoryEntry(entry.id)}
                    className="p-1.5 text-text-muted hover:text-red-500 hover:bg-red-500/10 rounded transition-colors"
                    title="Delete"
                  >
                    <TrashIcon />
                  </button>
                </div>
              </div>
              <p className="text-xs text-text-muted mt-2">
                {new Date(entry.timestamp).toLocaleString()}
              </p>
            </div>
          ))}
        </div>
      )}
    </div>
  );

  // Config File 设置内容
  const renderConfigFileSettings = () => (
    <div className="space-y-4">
      {/* 文件路径 */}
      <div className="flex items-center gap-2 text-xs text-text-muted">
        <span>Path:</span>
        <code className="px-2 py-1 bg-bg-tertiary rounded text-text-secondary break-all">
          {configFilePath || "Loading..."}
        </code>
      </div>

      {/* 编辑器 */}
      <div className="relative">
        <textarea
          value={configFileContent}
          onChange={(e) => {
            setConfigFileContent(e.target.value);
            setConfigFileModified(true);
          }}
          className="w-full h-[350px] px-4 py-3 text-sm font-mono bg-bg-input border border-border rounded-xl text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent focus:ring-1 focus:ring-accent/30 transition-all resize-none"
          placeholder="# Config file content will appear here..."
          spellCheck={false}
        />
        {configFileModified && (
          <div className="absolute top-2 right-2">
            <span className="text-xs text-accent bg-accent/10 px-2 py-1 rounded">
              Modified
            </span>
          </div>
        )}
      </div>

      {/* 保存按钮 */}
      <div className="flex items-center justify-between">
        <p className="text-xs text-text-muted">
          Changes will be applied after saving
        </p>
        <div className="flex gap-2">
          <button
            onClick={loadConfigFile}
            className="px-4 py-2 text-sm text-text-secondary hover:text-text-primary bg-bg-secondary border border-border-light rounded-lg hover:border-border transition-colors"
            disabled={!configFileModified}
          >
            Revert
          </button>
          <button
            onClick={saveConfigFile}
            disabled={!configFileModified}
            className={`px-4 py-2 text-sm font-medium rounded-lg transition-all ${
              configFileModified
                ? "text-white bg-accent hover:bg-accent-hover"
                : "text-text-muted bg-bg-tertiary cursor-not-allowed"
            }`}
          >
            Save Config
          </button>
        </div>
      </div>
    </div>
  );

  // Logs 设置内容
  const renderLogsSettings = () => (
    <div className="space-y-4">
      {/* 日志控制栏 */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          {/* 启用开关 */}
          <label className="flex items-center gap-2 cursor-pointer">
            <div className="relative">
              <input
                type="checkbox"
                checked={logInfo.enabled}
                onChange={(e) => toggleLogging(e.target.checked)}
                className="sr-only peer"
              />
              <div className="w-11 h-6 bg-bg-tertiary rounded-full peer peer-checked:bg-accent transition-colors" />
              <div className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow-sm transition-transform peer-checked:translate-x-5" />
            </div>
            <span className="text-sm text-text-primary">Enable Logging</span>
          </label>
          {/* 文件大小 */}
          <span className="text-xs text-text-muted">
            Size: {formatFileSize(logInfo.size)}
          </span>
        </div>
        <div className="flex gap-2">
          <button
            onClick={loadLogs}
            className="px-3 py-1.5 text-xs text-text-secondary hover:text-text-primary bg-bg-secondary border border-border-light rounded-lg hover:border-border transition-colors"
          >
            Refresh
          </button>
          <button
            onClick={clearLogs}
            className="px-3 py-1.5 text-xs text-red-500 hover:text-red-600 bg-bg-secondary border border-border-light rounded-lg hover:border-red-500/30 transition-colors"
          >
            Clear
          </button>
        </div>
      </div>

      {/* 日志文件路径 */}
      <div className="flex items-center gap-2 text-xs text-text-muted">
        <span>Path:</span>
        <code className="px-2 py-1 bg-bg-tertiary rounded text-text-secondary break-all">
          {logInfo.path || "Loading..."}
        </code>
      </div>

      {/* 日志内容 */}
      <div
        ref={logContainerRef}
        className="h-[320px] px-4 py-3 text-xs font-mono bg-bg-input border border-border rounded-xl text-text-primary overflow-y-auto"
      >
        {logEntries.length === 0 ? (
          <div className="text-text-muted text-center py-8">
            <LogsIcon />
            <p className="mt-2">No logs yet</p>
          </div>
        ) : (
          <div className="space-y-0.5">
            {logEntries.map((line, index) => {
              // 根据日志级别设置颜色
              const isError = line.includes("[ERROR]");
              const isWarn = line.includes("[WARN]");
              const isInfo = line.includes("[INFO]");
              return (
                <div
                  key={index}
                  className={`py-0.5 ${
                    isError
                      ? "text-red-400"
                      : isWarn
                        ? "text-amber-400"
                        : isInfo
                          ? "text-text-primary"
                          : "text-text-muted"
                  }`}
                >
                  {line}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );

  return (
    <div className="w-screen h-screen flex flex-col bg-bg-primary font-['Inter','-apple-system','BlinkMacSystemFont','Segoe_UI',sans-serif] antialiased select-none overflow-hidden rounded-xl relative">
      {/* 窗口边缘拖动调整大小区域 */}
      {/* 四边 */}
      <div
        className="absolute top-0 left-2 right-2 h-1 cursor-ns-resize z-50 hover:bg-accent/20 transition-colors"
        onMouseDown={() => handleResizeStart("North")}
      />
      <div
        className="absolute bottom-0 left-2 right-2 h-1 cursor-ns-resize z-50 hover:bg-accent/20 transition-colors"
        onMouseDown={() => handleResizeStart("South")}
      />
      <div
        className="absolute left-0 top-2 bottom-2 w-1 cursor-ew-resize z-50 hover:bg-accent/20 transition-colors"
        onMouseDown={() => handleResizeStart("West")}
      />
      <div
        className="absolute right-0 top-2 bottom-2 w-1 cursor-ew-resize z-50 hover:bg-accent/20 transition-colors"
        onMouseDown={() => handleResizeStart("East")}
      />
      {/* 四角 */}
      <div
        className="absolute top-0 left-0 w-3 h-3 cursor-nwse-resize z-50 hover:bg-accent/30 transition-colors rounded-tl-xl"
        onMouseDown={() => handleResizeStart("NorthWest")}
      />
      <div
        className="absolute top-0 right-0 w-3 h-3 cursor-nesw-resize z-50 hover:bg-accent/30 transition-colors rounded-tr-xl"
        onMouseDown={() => handleResizeStart("NorthEast")}
      />
      <div
        className="absolute bottom-0 left-0 w-3 h-3 cursor-nesw-resize z-50 hover:bg-accent/30 transition-colors rounded-bl-xl"
        onMouseDown={() => handleResizeStart("SouthWest")}
      />
      <div
        className="absolute bottom-0 right-0 w-3 h-3 cursor-nwse-resize z-50 hover:bg-accent/30 transition-colors rounded-br-xl"
        onMouseDown={() => handleResizeStart("SouthEast")}
      />

      {/* Toast 通知 */}
      <div className="fixed top-12 left-1/2 -translate-x-1/2 z-50 flex flex-col gap-2 pointer-events-none">
        {toasts.map((toast) => (
          <div
            key={toast.id}
            className={`pointer-events-auto px-4 py-2.5 rounded-xl shadow-lg text-sm font-medium flex items-center gap-2 animate-slide-down ${
              toast.type === "error"
                ? "bg-red-500 text-white"
                : toast.type === "success"
                  ? "bg-accent text-white"
                  : "bg-bg-secondary text-text-primary border border-border"
            }`}
          >
            <span>{toast.message}</span>
            <button
              onClick={() => dismissToast(toast.id)}
              className="ml-1 opacity-70 hover:opacity-100 transition-opacity"
            >
              <CloseIcon />
            </button>
          </div>
        ))}
      </div>

      {/* 标题栏 */}
      <div
        className="titlebar h-11 flex items-center px-3 bg-bg-secondary border-b border-border-light shrink-0 cursor-default"
        onMouseDown={handleDragStart}
      >
        {/* macOS 窗口控制按钮 */}
        <div className="flex gap-2 z-10">
          <button
            onClick={handleClose}
            className="w-3 h-3 rounded-full bg-[#ff5f57] hover:brightness-90 transition-all group flex items-center justify-center"
            title="关闭"
          >
            <span className="opacity-0 group-hover:opacity-100 text-[#4a0002] text-[8px] font-bold">×</span>
          </button>
          <button
            onClick={handleMinimize}
            className="w-3 h-3 rounded-full bg-[#febc2e] hover:brightness-90 transition-all group flex items-center justify-center"
            title="最小化"
          >
            <span className="opacity-0 group-hover:opacity-100 text-[#985600] text-[10px] font-bold leading-none">−</span>
          </button>
          <button
            className="w-3 h-3 rounded-full bg-[#28c840] hover:brightness-90 transition-all opacity-50 cursor-default"
            title="最大化 (禁用)"
            disabled
          />
        </div>

        {/* 标题 */}
        <span className="flex-1 text-center text-[13px] font-medium text-text-primary pointer-events-none">
          {showSettings ? "Settings" : "Speaky"}
        </span>

        {/* 右侧按钮组 */}
        <div className="flex items-center gap-0.5 z-10">
          <button
            onClick={toggleTheme}
            className="p-2 rounded-lg text-icon hover:text-icon-hover hover:bg-bg-tertiary transition-colors"
            title={isDark ? "切换到浅色模式" : "切换到深色模式"}
          >
            {isDark ? <SunIcon /> : <MoonIcon />}
          </button>
          <button
            onClick={toggleViewMode}
            className={`p-2 rounded-lg transition-colors ${
              showSettings
                ? "text-accent bg-accent/10"
                : "text-icon hover:text-icon-hover hover:bg-bg-tertiary"
            }`}
            title="设置"
          >
            <SettingsIcon />
          </button>
        </div>
      </div>

      {/* 设置面板 - 左右布局 */}
      {showSettings ? (
        <div key="settings" className="flex-1 flex overflow-hidden animate-view-enter">
          {/* 左侧导航 */}
          <div className="w-48 shrink-0 bg-bg-secondary border-r border-border-light p-3 flex flex-col animate-sidebar-in">
            <div className="space-y-1">
              {settingsTabs.map((tab) => (
                <button
                  key={tab.id}
                  onClick={() => setSettingsTab(tab.id)}
                  className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-left transition-colors ${
                    settingsTab === tab.id
                      ? "bg-accent/10 text-accent"
                      : "text-text-secondary hover:bg-bg-tertiary hover:text-text-primary"
                  }`}
                >
                  <span className={settingsTab === tab.id ? "text-accent" : "text-icon"}>
                    {tab.icon}
                  </span>
                  <span className="text-sm font-medium">{tab.label}</span>
                </button>
              ))}
            </div>

            {/* 底部返回按钮 */}
            <div className="mt-auto pt-3 border-t border-border-light">
              <button
                onClick={toggleViewMode}
                className="w-full flex items-center gap-2 px-3 py-2.5 rounded-lg text-text-secondary hover:bg-bg-tertiary hover:text-text-primary transition-colors"
              >
                <ChevronLeftIcon />
                <span className="text-sm">Back</span>
              </button>
            </div>
          </div>

          {/* 右侧内容 */}
          <div className="flex-1 flex flex-col overflow-hidden animate-settings-in">
            {/* 内容头部 */}
            <div className="h-14 flex items-center justify-between px-6 border-b border-border-light shrink-0">
              <h2 className="text-lg font-semibold text-text-primary">
                {currentTab?.label}
              </h2>
              {(settingsTab === "general" || settingsTab === "postprocess") && (
                <button
                  onClick={saveConfig}
                  className="px-4 py-1.5 text-sm font-medium text-white bg-accent rounded-lg hover:bg-accent-hover active:scale-[0.98] transition-all"
                >
                  Save
                </button>
              )}
            </div>

            {/* 设置内容 */}
            <div className="flex-1 p-6 overflow-y-auto">
              {settingsTab === "general" && renderGeneralSettings()}
              {settingsTab === "postprocess" && renderPostProcessSettings()}
              {settingsTab === "history" && renderHistorySettings()}
              {settingsTab === "logs" && renderLogsSettings()}
              {settingsTab === "config" && renderConfigFileSettings()}
            </div>
          </div>
        </div>
      ) : (
        /* 主界面 */
        <div key="main" className="flex-1 flex flex-col items-center justify-center p-6 gap-6 animate-bounce-in">
          {/* 录音按钮区域 */}
          <div className="flex flex-col items-center gap-5">
            <div className="relative">
              {/* 脉冲环 */}
              {isRecording && (
                <>
                  <div className="absolute inset-0 rounded-full bg-recording/20 animate-pulse-ring" />
                  <div className="absolute inset-0 rounded-full bg-recording/10 animate-pulse-ring [animation-delay:0.5s]" />
                </>
              )}

              <button
                className={`relative w-24 h-24 rounded-full transition-all duration-200 flex items-center justify-center
                  ${isRecording
                    ? "bg-recording shadow-[0_0_0_4px_rgba(220,38,38,0.15)]"
                    : isProcessing
                      ? "bg-bg-tertiary cursor-wait"
                      : "bg-bg-secondary hover:bg-bg-tertiary border border-border hover:border-border shadow-sm hover:shadow"
                  }`}
                onMouseDown={startRecording}
                onMouseUp={stopRecording}
                onMouseLeave={() => isRecording && stopRecording()}
                disabled={isProcessing}
              >
                {!isRecording && !isProcessing && (
                  <div className="text-icon">
                    <MicIcon />
                  </div>
                )}

                {isRecording && (
                  <div className="flex items-center gap-[3px] h-8">
                    <div className="w-1 bg-white rounded-full animate-wave-1 h-[35%]" />
                    <div className="w-1 bg-white rounded-full animate-wave-2 h-[65%]" />
                    <div className="w-1 bg-white rounded-full animate-wave-3 h-full" />
                    <div className="w-1 bg-white rounded-full animate-wave-2 h-[65%]" />
                    <div className="w-1 bg-white rounded-full animate-wave-1 h-[35%]" />
                  </div>
                )}

                {isProcessing && (
                  <div className="w-6 h-6 border-2 border-text-muted border-t-transparent rounded-full animate-spin" />
                )}
              </button>
            </div>

            <p className="text-sm text-text-secondary">{statusText}</p>
          </div>

          {/* 识别结果 */}
          <div
            className={`w-full max-w-md min-h-16 px-5 py-4 bg-bg-secondary rounded-2xl border transition-all duration-200 ${
              transcript ? "border-border opacity-100" : "border-border-light opacity-60"
            }`}
          >
            <p
              className={`text-center leading-relaxed ${
                transcript ? "text-sm text-text-primary" : "text-xs text-text-muted"
              }`}
            >
              {transcript || "Transcription will appear here"}
            </p>
          </div>
        </div>
      )}
    </div>
  );
}

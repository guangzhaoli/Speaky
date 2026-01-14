import { createRoot } from "react-dom/client";
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

type IndicatorState = "recording" | "processing" | "not-configured";

function Indicator() {
  const [state, setState] = useState<IndicatorState>("recording");

  useEffect(() => {
    const setupListeners = async () => {
      const unlistenRecording = await listen("recording-started", () => {
        setState("recording");
      });

      const unlistenProcessing = await listen("recording-stopped", () => {
        setState("processing");
      });

      const unlistenNotConfigured = await listen("indicator-not-configured", () => {
        setState("not-configured");
      });

      return () => {
        unlistenRecording();
        unlistenProcessing();
        unlistenNotConfigured();
      };
    };

    const cleanup = setupListeners();
    return () => {
      cleanup.then((fn) => fn?.());
    };
  }, []);

  const isRecording = state === "recording";
  const isNotConfigured = state === "not-configured";

  return (
    <div className="w-screen h-screen flex items-center justify-center p-1">
      <div
        className={`flex items-center gap-2.5 px-4 py-2.5 rounded-full transition-all duration-300 ${
          isNotConfigured
            ? "bg-amber-500/90 text-white shadow-lg shadow-amber-500/25"
            : isRecording
              ? "bg-gradient-to-r from-sky-500 to-blue-600 text-white shadow-lg shadow-blue-500/30"
              : "bg-slate-700/90 text-slate-100 shadow-lg shadow-slate-900/30"
        }`}
      >
        {isNotConfigured ? (
          <>
            {/* 警告图标 */}
            <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
              <path strokeLinecap="round" strokeLinejoin="round" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
            <span className="text-xs font-medium whitespace-nowrap">Not Configured</span>
          </>
        ) : isRecording ? (
          <>
            {/* 录音动画 - 声波效果 */}
            <div className="flex items-center gap-0.5 h-4">
              <div className="w-1 bg-white/90 rounded-full animate-wave-1" style={{ height: '40%' }} />
              <div className="w-1 bg-white/90 rounded-full animate-wave-2" style={{ height: '70%' }} />
              <div className="w-1 bg-white/90 rounded-full animate-wave-3" style={{ height: '100%' }} />
              <div className="w-1 bg-white/90 rounded-full animate-wave-2" style={{ height: '70%' }} />
              <div className="w-1 bg-white/90 rounded-full animate-wave-1" style={{ height: '40%' }} />
            </div>
            <span className="text-xs font-medium tracking-wide">Listening</span>
          </>
        ) : (
          <>
            {/* 处理中动画 */}
            <div className="w-4 h-4 border-2 border-slate-500 border-t-white rounded-full animate-spin" />
            <span className="text-xs font-medium tracking-wide">Processing</span>
          </>
        )}
      </div>
    </div>
  );
}

// 添加自定义动画样式
const style = document.createElement("style");
style.textContent = `
  @keyframes wave-1 {
    0%, 100% { height: 40%; }
    50% { height: 80%; }
  }
  @keyframes wave-2 {
    0%, 100% { height: 70%; }
    50% { height: 40%; }
  }
  @keyframes wave-3 {
    0%, 100% { height: 100%; }
    50% { height: 50%; }
  }
  .animate-wave-1 { animation: wave-1 0.8s ease-in-out infinite; }
  .animate-wave-2 { animation: wave-2 0.8s ease-in-out infinite 0.1s; }
  .animate-wave-3 { animation: wave-3 0.8s ease-in-out infinite 0.2s; }

  html, body, #root {
    background: transparent !important;
    margin: 0;
    padding: 0;
    overflow: hidden;
  }
`;
document.head.appendChild(style);

const root = createRoot(document.getElementById("root")!);
root.render(<Indicator />);

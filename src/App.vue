<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

// 状态
const isRecording = ref(false);
const isProcessing = ref(false);
const transcript = ref("");
const showSettings = ref(false);

// 配置
const config = ref({
  app_id: "",
  access_token: "",
  secret_key: "",
  shortcut: "Alt+Space",
  auto_type: true,
  auto_copy: true,
});

// 事件监听器
let unlistenStarted: UnlistenFn | null = null;
let unlistenStopped: UnlistenFn | null = null;
let unlistenUpdate: UnlistenFn | null = null;

onMounted(async () => {
  // 加载配置
  try {
    const savedConfig = await invoke("get_config");
    config.value = savedConfig as typeof config.value;
  } catch (e) {
    console.error("Failed to load config:", e);
  }

  // 监听录音事件
  unlistenStarted = await listen("recording-started", () => {
    isRecording.value = true;
    isProcessing.value = false;
    transcript.value = "";
  });

  unlistenStopped = await listen("recording-stopped", (event) => {
    isRecording.value = false;
    isProcessing.value = false;
    transcript.value = event.payload as string;
  });

  unlistenUpdate = await listen("transcript-update", (event) => {
    transcript.value = event.payload as string;
  });
});

onUnmounted(() => {
  unlistenStarted?.();
  unlistenStopped?.();
  unlistenUpdate?.();
});

// 开始录音
async function startRecording() {
  if (isRecording.value) return;
  try {
    await invoke("start_recording");
  } catch (e) {
    console.error("Failed to start recording:", e);
    alert(`录音失败: ${e}`);
  }
}

// 停止录音
async function stopRecording() {
  if (!isRecording.value) return;
  isProcessing.value = true;
  try {
    await invoke("stop_recording");
  } catch (e) {
    console.error("Failed to stop recording:", e);
    isProcessing.value = false;
  }
}

// 保存配置
async function saveConfig() {
  try {
    await invoke("update_config", { config: config.value });
    showSettings.value = false;
    alert("配置已保存");
  } catch (e) {
    console.error("Failed to save config:", e);
    alert(`保存失败: ${e}`);
  }
}
</script>

<template>
  <main class="container">
    <h1>Audio Input</h1>

    <!-- 设置面板 -->
    <div v-if="showSettings" class="settings-panel">
      <h2>设置</h2>
      <div class="form-group">
        <label>App ID</label>
        <input v-model="config.app_id" type="text" placeholder="输入豆包 App ID" />
      </div>
      <div class="form-group">
        <label>Access Token</label>
        <input v-model="config.access_token" type="password" placeholder="输入 Access Token" />
      </div>
      <div class="form-group">
        <label>Secret Key</label>
        <input v-model="config.secret_key" type="password" placeholder="输入 Secret Key (用于 HMAC 签名)" />
      </div>
      <div class="form-group checkbox">
        <input type="checkbox" id="auto_type" v-model="config.auto_type" />
        <label for="auto_type">自动输入到焦点窗口</label>
      </div>
      <div class="form-group checkbox">
        <input type="checkbox" id="auto_copy" v-model="config.auto_copy" />
        <label for="auto_copy">自动复制到剪贴板</label>
      </div>
      <div class="button-row">
        <button @click="saveConfig" class="primary">保存</button>
        <button @click="showSettings = false">取消</button>
      </div>
    </div>

    <!-- 主界面 -->
    <div v-else class="main-panel">
      <!-- 状态指示器 -->
      <div class="status-indicator" :class="{ recording: isRecording, processing: isProcessing }">
        <div class="status-dot"></div>
        <span v-if="isRecording">录音中...</span>
        <span v-else-if="isProcessing">处理中...</span>
        <span v-else>按住 Alt+Space 开始录音</span>
      </div>

      <!-- 录音按钮 -->
      <button
        class="record-button"
        :class="{ recording: isRecording }"
        @mousedown="startRecording"
        @mouseup="stopRecording"
        @mouseleave="isRecording && stopRecording()"
      >
        <svg viewBox="0 0 24 24" width="48" height="48">
          <circle v-if="!isRecording" cx="12" cy="12" r="10" fill="currentColor" />
          <rect v-else x="6" y="6" width="12" height="12" rx="2" fill="currentColor" />
        </svg>
      </button>

      <!-- 识别结果 -->
      <div class="transcript-display" v-if="transcript">
        <p>{{ transcript }}</p>
      </div>

      <!-- 设置按钮 -->
      <button class="settings-btn" @click="showSettings = true">设置</button>
    </div>
  </main>
</template>

<style>
:root {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  font-size: 14px;
  line-height: 1.5;
  color: #333;
  background-color: #f5f5f5;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #e0e0e0;
    background-color: #1a1a1a;
  }
}

* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

.container {
  max-width: 400px;
  margin: 0 auto;
  padding: 20px;
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  align-items: center;
}

h1 {
  font-size: 24px;
  margin-bottom: 20px;
  font-weight: 600;
}

h2 {
  font-size: 18px;
  margin-bottom: 16px;
}

.main-panel,
.settings-panel {
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 20px;
}

.status-indicator {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border-radius: 20px;
  background: #e0e0e0;
  font-size: 13px;
}

@media (prefers-color-scheme: dark) {
  .status-indicator {
    background: #333;
  }
}

.status-indicator.recording {
  background: #ffebee;
  color: #c62828;
}

@media (prefers-color-scheme: dark) {
  .status-indicator.recording {
    background: #4a1a1a;
    color: #ff6b6b;
  }
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #9e9e9e;
}

.status-indicator.recording .status-dot {
  background: #c62828;
  animation: pulse 1s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.record-button {
  width: 80px;
  height: 80px;
  border-radius: 50%;
  border: none;
  background: #2196f3;
  color: white;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  box-shadow: 0 4px 12px rgba(33, 150, 243, 0.3);
}

.record-button:hover {
  transform: scale(1.05);
}

.record-button:active,
.record-button.recording {
  background: #c62828;
  box-shadow: 0 4px 12px rgba(198, 40, 40, 0.3);
}

.transcript-display {
  width: 100%;
  padding: 16px;
  background: white;
  border-radius: 8px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  max-height: 120px;
  overflow-y: auto;
}

@media (prefers-color-scheme: dark) {
  .transcript-display {
    background: #2a2a2a;
  }
}

.transcript-display p {
  word-wrap: break-word;
}

.settings-btn {
  padding: 8px 20px;
  border: 1px solid #ddd;
  border-radius: 6px;
  background: transparent;
  cursor: pointer;
  font-size: 13px;
}

@media (prefers-color-scheme: dark) {
  .settings-btn {
    border-color: #444;
    color: #e0e0e0;
  }
}

.settings-btn:hover {
  background: #f0f0f0;
}

@media (prefers-color-scheme: dark) {
  .settings-btn:hover {
    background: #333;
  }
}

.form-group {
  width: 100%;
  margin-bottom: 12px;
}

.form-group label {
  display: block;
  margin-bottom: 4px;
  font-size: 13px;
  color: #666;
}

@media (prefers-color-scheme: dark) {
  .form-group label {
    color: #aaa;
  }
}

.form-group input[type="text"],
.form-group input[type="password"] {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid #ddd;
  border-radius: 6px;
  font-size: 14px;
}

@media (prefers-color-scheme: dark) {
  .form-group input[type="text"],
  .form-group input[type="password"] {
    background: #2a2a2a;
    border-color: #444;
    color: #e0e0e0;
  }
}

.form-group.checkbox {
  display: flex;
  align-items: center;
  gap: 8px;
}

.form-group.checkbox label {
  margin: 0;
  cursor: pointer;
}

.button-row {
  display: flex;
  gap: 12px;
  width: 100%;
}

.button-row button {
  flex: 1;
  padding: 10px;
  border-radius: 6px;
  border: 1px solid #ddd;
  cursor: pointer;
  font-size: 14px;
}

@media (prefers-color-scheme: dark) {
  .button-row button {
    background: #2a2a2a;
    border-color: #444;
    color: #e0e0e0;
  }
}

.button-row button.primary {
  background: #2196f3;
  border-color: #2196f3;
  color: white;
}

.button-row button.primary:hover {
  background: #1976d2;
}
</style>

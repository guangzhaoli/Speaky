use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// 音频设备信息
#[derive(Debug, Clone, Serialize)]
pub struct AudioDevice {
    pub name: String,
    pub is_default: bool,
}

/// 获取所有可用的输入设备列表
pub fn list_audio_devices() -> Vec<AudioDevice> {
    let host = cpal::default_host();
    let default_device_name = host
        .default_input_device()
        .and_then(|d| d.name().ok());

    let mut devices = Vec::with_capacity(8); // 预分配避免多次分配

    // 添加 "系统默认" 选项
    devices.push(AudioDevice {
        name: String::new(),
        is_default: true,
    });

    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            if let Ok(name) = device.name() {
                let is_default = default_device_name.as_ref() == Some(&name);
                devices.push(AudioDevice {
                    name,
                    is_default,
                });
            }
        }
    }

    devices
}

/// 音频采集控制器
/// 使用独立线程管理 cpal::Stream，避免跨线程发送问题
pub struct AudioCaptureController {
    is_recording: Arc<AtomicBool>,
    stop_signal: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
    device_name: String,
}

impl AudioCaptureController {
    pub fn new() -> Self {
        Self {
            is_recording: Arc::new(AtomicBool::new(false)),
            stop_signal: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
            device_name: String::new(),
        }
    }

    /// 创建一个指定设备的控制器
    pub fn with_device(device_name: String) -> Self {
        Self {
            is_recording: Arc::new(AtomicBool::new(false)),
            stop_signal: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
            device_name,
        }
    }

    pub fn start_recording(&mut self, audio_sender: Sender<Vec<i16>>) -> Result<(), String> {
        if self.is_recording.load(Ordering::SeqCst) {
            return Err("Already recording".to_string());
        }

        let is_recording = self.is_recording.clone();
        let stop_signal = self.stop_signal.clone();
        let device_name = self.device_name.clone();

        // 重置停止信号
        stop_signal.store(false, Ordering::SeqCst);
        is_recording.store(true, Ordering::SeqCst);

        // 在独立线程中运行音频采集
        let handle = thread::spawn(move || {
            if let Err(e) = run_audio_capture(audio_sender, stop_signal.clone(), device_name) {
                log::error!("Audio capture error: {}", e);
            }
            is_recording.store(false, Ordering::SeqCst);
        });

        self.thread_handle = Some(handle);
        log::info!("Audio recording started");
        Ok(())
    }
}

impl Default for AudioCaptureController {
    fn default() -> Self {
        Self::new()
    }
}

/// 在当前线程运行音频采集
fn run_audio_capture(
    audio_sender: Sender<Vec<i16>>,
    stop_signal: Arc<AtomicBool>,
    device_name: String,
) -> Result<(), String> {
    let host = cpal::default_host();

    // 根据设备名称选择设备
    let device = if device_name.is_empty() {
        host.default_input_device()
            .ok_or("No input device available")?
    } else {
        host.input_devices()
            .map_err(|e| format!("Failed to enumerate devices: {}", e))?
            .find(|d| d.name().map(|n| n == device_name).unwrap_or(false))
            .ok_or_else(|| format!("Device '{}' not found", device_name))?
    };

    log::info!("Using input device: {}", device.name().unwrap_or_default());

    // 豆包 ASR 要求: 16kHz, 单声道, 16-bit PCM
    let config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(16000),
        buffer_size: cpal::BufferSize::Default,
    };

    let stop = stop_signal.clone();

    // 使用预分配缓冲区的发送策略，减少每帧的内存分配
    let stream = device
        .build_input_stream(
            &config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                if !stop.load(Ordering::Relaxed) {
                    // 预分配恰好大小的 Vec，避免过度分配
                    let mut buffer = Vec::with_capacity(data.len());
                    buffer.extend_from_slice(data);
                    let _ = audio_sender.send(buffer);
                }
            },
            |err| log::error!("Audio stream error: {}", err),
            None,
        )
        .map_err(|e| format!("Failed to build input stream: {}", e))?;

    stream
        .play()
        .map_err(|e| format!("Failed to play stream: {}", e))?;

    // 保持流活跃直到收到停止信号
    while !stop_signal.load(Ordering::SeqCst) {
        thread::sleep(std::time::Duration::from_millis(50));
    }

    Ok(())
}

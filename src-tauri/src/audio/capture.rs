use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// 音频采集控制器
/// 使用独立线程管理 cpal::Stream，避免跨线程发送问题
pub struct AudioCaptureController {
    is_recording: Arc<AtomicBool>,
    stop_signal: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl AudioCaptureController {
    pub fn new() -> Self {
        Self {
            is_recording: Arc::new(AtomicBool::new(false)),
            stop_signal: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
        }
    }

    pub fn start_recording(&mut self, audio_sender: Sender<Vec<i16>>) -> Result<(), String> {
        if self.is_recording.load(Ordering::SeqCst) {
            return Err("Already recording".to_string());
        }

        let is_recording = self.is_recording.clone();
        let stop_signal = self.stop_signal.clone();

        // 重置停止信号
        stop_signal.store(false, Ordering::SeqCst);
        is_recording.store(true, Ordering::SeqCst);

        // 在独立线程中运行音频采集
        let handle = thread::spawn(move || {
            if let Err(e) = run_audio_capture(audio_sender, stop_signal.clone()) {
                log::error!("Audio capture error: {}", e);
            }
            is_recording.store(false, Ordering::SeqCst);
        });

        self.thread_handle = Some(handle);
        log::info!("Audio recording started");
        Ok(())
    }

    pub fn stop_recording(&mut self) {
        self.stop_signal.store(true, Ordering::SeqCst);

        // 等待线程结束
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }

        self.is_recording.store(false, Ordering::SeqCst);
        log::info!("Audio recording stopped");
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
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
) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No input device available")?;

    log::info!("Using input device: {}", device.name().unwrap_or_default());

    // 豆包 ASR 要求: 16kHz, 单声道, 16-bit PCM
    let config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(16000),
        buffer_size: cpal::BufferSize::Default,
    };

    let sender = Arc::new(Mutex::new(audio_sender));
    let stop = stop_signal.clone();

    let stream = device
        .build_input_stream(
            &config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                if !stop.load(Ordering::SeqCst) {
                    let sender = sender.lock();
                    let _ = sender.send(data.to_vec());
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

    // stream 在这里自动 drop，停止录音
    Ok(())
}

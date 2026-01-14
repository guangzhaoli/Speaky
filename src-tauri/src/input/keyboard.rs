use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

pub struct KeyboardSimulator {
    enigo: Enigo,
    /// 跟踪已输入的字符数（用于实时更新）
    last_input_len: usize,
}

impl KeyboardSimulator {
    pub fn new() -> Result<Self, String> {
        let enigo = Enigo::new(&Settings::default())
            .map_err(|e| format!("Failed to create Enigo: {}", e))?;
        Ok(Self {
            enigo,
            last_input_len: 0,
        })
    }

    /// 重置输入状态（开始新的录音会话时调用）
    pub fn reset_input_state(&mut self) {
        self.last_input_len = 0;
    }

    /// 实时更新文本（删除旧文本，输入新文本）
    pub fn update_text(&mut self, new_text: &str) -> Result<(), String> {
        let new_len = new_text.chars().count();

        // 删除之前输入的字符
        if self.last_input_len > 0 {
            for _ in 0..self.last_input_len {
                self.enigo
                    .key(Key::Backspace, Direction::Click)
                    .map_err(|e| format!("Failed to press backspace: {}", e))?;
            }
            thread::sleep(Duration::from_millis(5));
        }

        // 输入新文本
        if !new_text.is_empty() {
            self.enigo
                .text(new_text)
                .map_err(|e| format!("Failed to type text: {}", e))?;
        }

        self.last_input_len = new_len;
        Ok(())
    }

    /// 完成实时输入（重置状态，不做任何操作）
    pub fn finish_realtime_input(&mut self) {
        self.last_input_len = 0;
    }

    /// 模拟键盘输入文本
    pub fn type_text(&mut self, text: &str) -> Result<(), String> {
        // 等待一小段时间确保焦点切换完成
        thread::sleep(Duration::from_millis(100));

        self.enigo
            .text(text)
            .map_err(|e| format!("Failed to type text: {}", e))
    }

    /// 模拟粘贴操作（跨平台：macOS 使用 Cmd+V，其他平台使用 Ctrl+V）
    pub fn paste(&mut self) -> Result<(), String> {
        // 短暂等待确保剪贴板内容可用
        thread::sleep(Duration::from_millis(50));

        // macOS 使用 Command 键，其他平台使用 Control 键
        #[cfg(target_os = "macos")]
        let modifier_key = Key::Meta;
        #[cfg(not(target_os = "macos"))]
        let modifier_key = Key::Control;

        // 按下修饰键
        self.enigo
            .key(modifier_key, Direction::Press)
            .map_err(|e| format!("Failed to press modifier: {}", e))?;

        thread::sleep(Duration::from_millis(10));

        // 按下 V
        self.enigo
            .key(Key::Unicode('v'), Direction::Click)
            .map_err(|e| format!("Failed to press V: {}", e))?;

        thread::sleep(Duration::from_millis(10));

        // 释放修饰键
        self.enigo
            .key(modifier_key, Direction::Release)
            .map_err(|e| format!("Failed to release modifier: {}", e))?;

        // 等待系统处理粘贴
        thread::sleep(Duration::from_millis(30));

        Ok(())
    }
}

impl Default for KeyboardSimulator {
    fn default() -> Self {
        Self::new().expect("Failed to create keyboard simulator")
    }
}

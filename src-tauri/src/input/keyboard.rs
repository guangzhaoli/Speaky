use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

pub struct KeyboardSimulator {
    enigo: Enigo,
}

impl KeyboardSimulator {
    pub fn new() -> Result<Self, String> {
        let enigo =
            Enigo::new(&Settings::default()).map_err(|e| format!("Failed to create Enigo: {}", e))?;
        Ok(Self { enigo })
    }

    /// 模拟键盘输入文本
    pub fn type_text(&mut self, text: &str) -> Result<(), String> {
        // 等待一小段时间确保焦点切换完成
        thread::sleep(Duration::from_millis(100));

        self.enigo
            .text(text)
            .map_err(|e| format!("Failed to type text: {}", e))
    }

    /// 模拟 Ctrl+V 粘贴（用于已经在剪贴板中的文本）
    pub fn paste(&mut self) -> Result<(), String> {
        // 短暂等待确保剪贴板内容可用
        thread::sleep(Duration::from_millis(30));

        // 按下 Ctrl
        self.enigo
            .key(Key::Control, Direction::Press)
            .map_err(|e| format!("Failed to press Ctrl: {}", e))?;

        // 按下 V
        self.enigo
            .key(Key::Unicode('v'), Direction::Click)
            .map_err(|e| format!("Failed to press V: {}", e))?;

        // 释放 Ctrl
        self.enigo
            .key(Key::Control, Direction::Release)
            .map_err(|e| format!("Failed to release Ctrl: {}", e))?;

        Ok(())
    }

    /// 按下并释放一个键
    #[allow(dead_code)]
    pub fn press_key(&mut self, key: Key) -> Result<(), String> {
        self.enigo
            .key(key, Direction::Click)
            .map_err(|e| format!("Failed to press key: {}", e))
    }
}

impl Default for KeyboardSimulator {
    fn default() -> Self {
        Self::new().expect("Failed to create keyboard simulator")
    }
}

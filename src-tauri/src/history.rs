use chrono::{DateTime, Local};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 历史记录条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub text: String,
    pub timestamp: DateTime<Local>,
}

/// 历史记录管理器
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct History {
    pub entries: Vec<HistoryEntry>,
}

const MAX_HISTORY_ENTRIES: usize = 100;

impl History {
    /// 获取历史文件路径
    fn history_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "speaky", "Speaky")
            .map(|dirs| dirs.data_dir().join("history.json"))
    }

    /// 从文件加载历史记录
    pub fn load() -> Self {
        if let Some(path) = Self::history_path() {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(history) = serde_json::from_str(&content) {
                        log::info!("History loaded from {:?}", path);
                        return history;
                    }
                }
            }
        }
        Self::default()
    }

    /// 保存历史记录到文件
    pub fn save(&self) -> Result<(), String> {
        let path = Self::history_path().ok_or("Failed to get history path")?;

        // 创建数据目录
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create data dir: {}", e))?;
        }

        // 使用紧凑格式减少文件大小
        let content = serde_json::to_string(self)
            .map_err(|e| format!("Failed to serialize history: {}", e))?;

        fs::write(&path, content).map_err(|e| format!("Failed to write history: {}", e))?;

        log::debug!("History saved ({} entries)", self.entries.len());
        Ok(())
    }

    /// 添加一条历史记录
    pub fn add_entry(&mut self, text: String) {
        // 跳过空白文本
        if text.trim().is_empty() {
            return;
        }

        let entry = HistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            text,
            timestamp: Local::now(),
        };
        self.entries.insert(0, entry);

        // 限制历史记录数量
        if self.entries.len() > MAX_HISTORY_ENTRIES {
            self.entries.truncate(MAX_HISTORY_ENTRIES);
        }
    }

    /// 删除一条历史记录
    pub fn delete_entry(&mut self, id: &str) -> bool {
        let original_len = self.entries.len();
        self.entries.retain(|e| e.id != id);
        self.entries.len() != original_len
    }

    /// 清空所有历史记录
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

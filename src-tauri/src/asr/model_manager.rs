//! 模型下载管理模块
//!
//! 提供模型文件下载功能，支持断点续传和进度报告。

use futures::StreamExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

use crate::asr::provider::{AsrError, DownloadProgress};

/// 下载文件到指定路径
///
/// # 参数
/// - `url`: 下载 URL
/// - `temp_path`: 临时文件路径
/// - `dest_path`: 最终目标路径
/// - `model_id`: 模型 ID（用于进度报告）
/// - `progress_tx`: 进度发送通道
/// - `cancel_flag`: 取消标志
pub async fn download_file(
    url: &str,
    temp_path: &Path,
    dest_path: &Path,
    model_id: &str,
    progress_tx: mpsc::Sender<DownloadProgress>,
    cancel_flag: Arc<AtomicBool>,
) -> Result<(), AsrError> {
    let client = reqwest::Client::new();

    // 检查已下载的大小（用于断点续传）
    let mut downloaded: u64 = if temp_path.exists() {
        std::fs::metadata(temp_path)
            .map(|m| m.len())
            .unwrap_or(0)
    } else {
        0
    };

    // 发起请求，支持 Range
    let mut request = client.get(url);
    if downloaded > 0 {
        request = request.header("Range", format!("bytes={}-", downloaded));
        log::info!("断点续传，从 {} 字节开始", downloaded);
    }

    let response = request
        .send()
        .await
        .map_err(|e| AsrError::ModelDownload(format!("请求失败: {}", e)))?;

    if !response.status().is_success() && response.status() != reqwest::StatusCode::PARTIAL_CONTENT
    {
        return Err(AsrError::ModelDownload(format!(
            "下载失败: HTTP {}",
            response.status()
        )));
    }

    // 获取总大小
    let total_size = if downloaded > 0 {
        // 断点续传，从 Content-Range 获取总大小
        response
            .headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split('/').last())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    } else {
        response.content_length().unwrap_or(0)
    };

    // 打开文件（追加模式）
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(temp_path)
        .await
        .map_err(|e| AsrError::ModelDownload(format!("打开文件失败: {}", e)))?;

    // 流式下载
    let mut stream = response.bytes_stream();
    let mut last_progress_percent: u32 = 0;

    while let Some(chunk_result) = stream.next().await {
        // 检查取消标志
        if cancel_flag.load(Ordering::SeqCst) {
            log::info!("下载已取消");
            return Err(AsrError::ModelDownload("下载已取消".into()));
        }

        let chunk = chunk_result.map_err(|e| AsrError::ModelDownload(format!("读取数据失败: {}", e)))?;

        file.write_all(&chunk)
            .await
            .map_err(|e| AsrError::ModelDownload(format!("写入文件失败: {}", e)))?;

        downloaded += chunk.len() as u64;

        // 限制进度更新频率（每 1% 更新一次）
        let current_percent = if total_size > 0 {
            ((downloaded as f32 / total_size as f32) * 100.0) as u32
        } else {
            0
        };

        if current_percent != last_progress_percent {
            last_progress_percent = current_percent;
            let _ = progress_tx
                .send(DownloadProgress {
                    model_id: model_id.to_string(),
                    downloaded_bytes: downloaded,
                    total_bytes: total_size,
                    percent: current_percent as f32,
                })
                .await;
        }
    }

    // 确保写入完成
    file.flush().await.map_err(|e| AsrError::ModelDownload(format!("刷新文件失败: {}", e)))?;
    drop(file);

    // 重命名完成的文件
    std::fs::rename(temp_path, dest_path)
        .map_err(|e| AsrError::ModelDownload(format!("重命名文件失败: {}", e)))?;

    // 发送完成进度
    let _ = progress_tx
        .send(DownloadProgress {
            model_id: model_id.to_string(),
            downloaded_bytes: total_size,
            total_bytes: total_size,
            percent: 100.0,
        })
        .await;

    log::info!("模型下载完成: {:?}", dest_path);
    Ok(())
}

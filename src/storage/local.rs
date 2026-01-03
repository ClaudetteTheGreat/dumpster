//! Local filesystem storage backend.

use super::{ByteStream, StorageBackend, StorageError, StorageObject};
use actix_web::web::{self, Bytes};
use async_trait::async_trait;
use futures::stream;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

/// Local filesystem storage backend.
pub struct LocalStorage {
    /// Base path for file storage
    base_path: PathBuf,
}

impl LocalStorage {
    /// Create a new local storage backend.
    ///
    /// The `base_path` directory will be created if it doesn't exist.
    pub fn new(base_path: PathBuf) -> Result<Self, StorageError> {
        // Create base directory if it doesn't exist
        fs::create_dir_all(&base_path)?;
        log::info!("LocalStorage initialized at {:?}", base_path);
        Ok(Self { base_path })
    }

    /// Get the full path for a file, including prefix directories.
    fn get_file_path(&self, filename: &str) -> PathBuf {
        if filename.len() < 4 {
            // Fallback for short filenames
            self.base_path.join(filename)
        } else {
            let prefix1 = &filename[0..2];
            let prefix2 = &filename[2..4];
            self.base_path.join(prefix1).join(prefix2).join(filename)
        }
    }

    /// Parse HTTP Range header.
    /// Supports formats like "bytes=0-499" or "bytes=500-"
    fn parse_range(range: &str, file_size: u64) -> Result<(u64, u64), StorageError> {
        let range = range
            .strip_prefix("bytes=")
            .ok_or_else(|| StorageError::InvalidRange("Invalid range format".into()))?;

        let parts: Vec<&str> = range.split('-').collect();
        if parts.len() != 2 {
            return Err(StorageError::InvalidRange("Invalid range format".into()));
        }

        let start: u64 = if parts[0].is_empty() {
            // Suffix range like "-500" means last 500 bytes
            let suffix: u64 = parts[1]
                .parse()
                .map_err(|_| StorageError::InvalidRange("Invalid range number".into()))?;
            file_size.saturating_sub(suffix)
        } else {
            parts[0]
                .parse()
                .map_err(|_| StorageError::InvalidRange("Invalid range number".into()))?
        };

        let end: u64 = if parts[1].is_empty() {
            file_size - 1
        } else {
            parts[1]
                .parse()
                .map_err(|_| StorageError::InvalidRange("Invalid range number".into()))?
        };

        if start > end || start >= file_size {
            return Err(StorageError::InvalidRange("Range not satisfiable".into()));
        }

        Ok((start, end.min(file_size - 1)))
    }

    /// Get MIME type from filename extension.
    fn get_mime_type(filename: &str) -> Option<String> {
        let ext = filename.rsplit('.').next()?;
        let mime = match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "ico" => "image/x-icon",
            "bmp" => "image/bmp",
            "avif" => "image/avif",
            "mp4" => "video/mp4",
            "webm" => "video/webm",
            "mkv" => "video/x-matroska",
            "avi" => "video/x-msvideo",
            "mov" => "video/quicktime",
            "mp3" => "audio/mpeg",
            "ogg" => "audio/ogg",
            "flac" => "audio/flac",
            "wav" => "audio/wav",
            "pdf" => "application/pdf",
            "zip" => "application/zip",
            "json" => "application/json",
            "txt" => "text/plain",
            "html" => "text/html",
            "css" => "text/css",
            "js" => "application/javascript",
            _ => "application/octet-stream",
        };
        Some(mime.to_string())
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn put_object(&self, data: Vec<u8>, filename: &str) -> Result<(), StorageError> {
        let path = self.get_file_path(filename);
        log::info!("LocalStorage: put_object: {:?}", path);

        // Use web::block for blocking file operations
        web::block(move || {
            // Create parent directories
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            // Write file
            fs::write(&path, data)
        })
        .await
        .map_err(|e| StorageError::Io(std::io::Error::other(e)))??;

        Ok(())
    }

    async fn get_object(
        &self,
        key: &str,
        range: Option<String>,
    ) -> Result<StorageObject, StorageError> {
        let path = self.get_file_path(key);
        log::debug!("LocalStorage: get_object: {:?}", path);

        let key_owned = key.to_string();
        let range_clone = range.clone();
        let path_clone = path.clone();

        // Use web::block for blocking file operations
        let result = web::block(
            move || -> Result<(Vec<u8>, std::fs::Metadata, Option<String>), StorageError> {
                // Get file metadata
                let metadata = fs::metadata(&path_clone)?;
                let file_size = metadata.len();

                // Handle range request
                let (start, end, content_range) = if let Some(ref range_header) = range_clone {
                    let (start, end) = LocalStorage::parse_range(range_header, file_size)?;
                    let range_str = format!("bytes {}-{}/{}", start, end, file_size);
                    (start, end, Some(range_str))
                } else {
                    (0, file_size.saturating_sub(1), None)
                };

                let bytes_to_read = (end - start + 1) as usize;

                // Read file content (with range support)
                let mut file = fs::File::open(&path_clone)?;
                if start > 0 {
                    file.seek(SeekFrom::Start(start))?;
                }

                let mut buffer = vec![0u8; bytes_to_read];
                file.read_exact(&mut buffer)?;

                Ok((buffer, metadata, content_range))
            },
        )
        .await
        .map_err(|e| StorageError::Io(std::io::Error::other(e)))??;

        let (buffer, metadata, content_range) = result;
        let content_length = buffer.len() as i64;

        // Get modification time for ETag and Last-Modified
        let modified = metadata.modified().ok();
        let e_tag = modified.map(|t: std::time::SystemTime| {
            let duration = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
            format!("\"{}\"", duration.as_secs())
        });
        let last_modified = modified.map(|t: std::time::SystemTime| {
            let datetime: chrono::DateTime<chrono::Utc> = t.into();
            datetime.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
        });

        let content_type = Self::get_mime_type(&key_owned);

        // Create streaming body
        let body: ByteStream = Box::pin(stream::once(async move { Ok(Bytes::from(buffer)) }));

        Ok(StorageObject {
            body,
            content_length: Some(content_length),
            content_type,
            e_tag,
            content_range,
            accept_ranges: Some("bytes".to_string()),
            last_modified,
        })
    }

    async fn exists(&self, filename: &str) -> Result<bool, StorageError> {
        let path = self.get_file_path(filename);
        Ok(path.exists())
    }
}

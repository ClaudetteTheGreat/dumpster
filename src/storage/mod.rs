//! Storage backend abstraction for file uploads.
//!
//! Supports multiple backends:
//! - `local`: Local filesystem storage
//! - `s3`: S3-compatible object storage (MinIO, AWS S3, etc.)

pub mod local;
pub mod s3;

use actix_web::web::Bytes;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// A boxed stream of bytes for streaming file content.
pub type ByteStream = Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>;

/// Represents a retrieved storage object with metadata.
pub struct StorageObject {
    /// Streaming body content
    pub body: ByteStream,
    /// Content length in bytes
    pub content_length: Option<i64>,
    /// MIME content type
    pub content_type: Option<String>,
    /// Entity tag for caching
    pub e_tag: Option<String>,
    /// Content range for partial responses
    pub content_range: Option<String>,
    /// Accept ranges header value
    pub accept_ranges: Option<String>,
    /// Last modified timestamp
    pub last_modified: Option<String>,
}

/// Storage operation errors.
#[derive(Debug)]
pub enum StorageError {
    /// File not found
    NotFound(String),
    /// I/O error
    Io(std::io::Error),
    /// S3 error
    S3(String),
    /// Invalid range request
    InvalidRange(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::NotFound(msg) => write!(f, "Not found: {}", msg),
            StorageError::Io(e) => write!(f, "I/O error: {}", e),
            StorageError::S3(msg) => write!(f, "S3 error: {}", msg),
            StorageError::InvalidRange(msg) => write!(f, "Invalid range: {}", msg),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        if e.kind() == std::io::ErrorKind::NotFound {
            StorageError::NotFound(e.to_string())
        } else {
            StorageError::Io(e)
        }
    }
}

/// Trait for storage backends.
///
/// All storage backends must implement this trait to provide
/// a unified interface for file storage operations.
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Store a file.
    ///
    /// Files are stored with a prefix structure based on the filename:
    /// `{filename[0:2]}/{filename[2:4]}/{filename}`
    async fn put_object(&self, data: Vec<u8>, filename: &str) -> Result<(), StorageError>;

    /// Retrieve a file.
    ///
    /// The `key` is the canonical filename (hash + extension).
    /// Optional `range` parameter supports HTTP Range requests for streaming.
    async fn get_object(
        &self,
        key: &str,
        range: Option<String>,
    ) -> Result<StorageObject, StorageError>;

    /// Check if a file exists.
    async fn exists(&self, filename: &str) -> Result<bool, StorageError>;
}

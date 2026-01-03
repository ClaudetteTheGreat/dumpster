//! S3-compatible storage backend.

use super::{ByteStream, StorageBackend, StorageError, StorageObject};
use actix_web::web::Bytes;
use async_trait::async_trait;
use futures::TryStreamExt;
use rusoto_core::Region;
use rusoto_s3::{GetObjectRequest, ListObjectsV2Request, PutObjectRequest, S3Client, S3};

/// S3-compatible storage backend.
pub struct S3Storage {
    s3: S3Client,
    bucket_name: String,
    pub pub_url: String,
}

impl S3Storage {
    /// Create a new S3 storage backend.
    pub fn new(region: Region, bucket_name: String, pub_url: String) -> S3Storage {
        log::info!("S3Storage initialized for bucket: {}", bucket_name);

        S3Storage {
            s3: S3Client::new(region),
            bucket_name,
            pub_url,
        }
    }

    /// Get the S3 key path for a filename.
    fn get_key_path(filename: &str) -> String {
        if filename.len() < 4 {
            filename.to_string()
        } else {
            let prefix1 = &filename[0..2];
            let prefix2 = &filename[2..4];
            format!("{}/{}/{}", prefix1, prefix2, filename)
        }
    }
}

#[async_trait]
impl StorageBackend for S3Storage {
    async fn put_object(&self, data: Vec<u8>, filename: &str) -> Result<(), StorageError> {
        log::info!("S3Storage: put_object: {}", filename);

        let key = Self::get_key_path(filename);
        let put_request = PutObjectRequest {
            bucket: self.bucket_name.clone(),
            key,
            body: Some(data.into()),
            ..Default::default()
        };

        self.s3
            .put_object(put_request)
            .await
            .map_err(|e| StorageError::S3(e.to_string()))?;

        Ok(())
    }

    async fn get_object(
        &self,
        key: &str,
        range: Option<String>,
    ) -> Result<StorageObject, StorageError> {
        log::debug!("S3Storage: get_object: {}", key);

        let key_path = Self::get_key_path(key);
        let request = GetObjectRequest {
            bucket: self.bucket_name.clone(),
            key: key_path,
            range,
            ..Default::default()
        };

        let output = self
            .s3
            .get_object(request)
            .await
            .map_err(|e| StorageError::S3(e.to_string()))?;

        // Convert the S3 body stream to our ByteStream type
        let body: ByteStream = match output.body {
            Some(stream) => {
                let mapped = stream.map_ok(Bytes::from).map_err(|e: std::io::Error| {
                    std::io::Error::other(e.to_string())
                });
                Box::pin(mapped)
            }
            None => {
                return Err(StorageError::NotFound("Empty body".into()));
            }
        };

        Ok(StorageObject {
            body,
            content_length: output.content_length,
            content_type: output.content_type,
            e_tag: output.e_tag,
            content_range: output.content_range,
            accept_ranges: output.accept_ranges,
            last_modified: output.last_modified,
        })
    }

    async fn exists(&self, filename: &str) -> Result<bool, StorageError> {
        log::debug!("S3Storage: exists: {}", filename);

        // Using list_objects_v2 is reportedly faster than head_object
        // https://www.peterbe.com/plog/fastest-way-to-find-out-if-a-file-exists-in-s3
        let list_request = ListObjectsV2Request {
            bucket: self.bucket_name.clone(),
            prefix: Some(filename.to_owned()),
            ..Default::default()
        };

        let result = self
            .s3
            .list_objects_v2(list_request)
            .await
            .map_err(|e| StorageError::S3(e.to_string()))?;

        let count = result.key_count.unwrap_or(0);
        Ok(count > 0)
    }
}

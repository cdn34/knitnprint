use std::{env, sync::Arc, time::Duration};

use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::{Client, config::Region, presigning::PresigningConfig, primitives::ByteStream};
use thiserror::Error;

use crate::config::Environment;

const DEFAULT_LOCAL_ENDPOINT: &str = "http://127.0.0.1:9100";
const DEFAULT_LOCAL_REGION: &str = "eu-west-1";
const DEFAULT_LOCAL_BUCKET: &str = "knitnprint-media";
const DEFAULT_LOCAL_ACCESS_KEY: &str = "knitnprint";
const DEFAULT_LOCAL_SECRET_KEY: &str = "knitnprint-local";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageProvider {
    Minio,
    AwsS3,
}

impl StorageProvider {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Minio => "minio",
            Self::AwsS3 => "aws_s3",
        }
    }
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("could not create a signed object-storage request")]
    Presign,
    #[error("object metadata could not be read")]
    Head,
    #[error("object could not be read")]
    Read,
    #[error("object body could not be collected")]
    ReadBody,
    #[error("object could not be written")]
    Write,
    #[error("object could not be deleted")]
    Delete,
}

#[derive(Debug)]
pub struct ObjectMetadata {
    pub content_length: Option<i64>,
    pub content_type: Option<String>,
}

#[derive(Debug)]
pub struct StoredObject {
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
    pub etag: Option<String>,
}

#[async_trait]
pub trait StorageBackend: Send + Sync {
    fn provider(&self) -> StorageProvider;
    fn bucket(&self) -> &str;

    async fn presign_upload(
        &self,
        key: &str,
        content_type: &str,
        content_length: i64,
        expires_in: Duration,
    ) -> Result<String, StorageError>;

    async fn presign_download(
        &self,
        key: &str,
        expires_in: Duration,
    ) -> Result<String, StorageError>;

    async fn head(&self, key: &str) -> Result<ObjectMetadata, StorageError>;
    async fn get(&self, key: &str) -> Result<StoredObject, StorageError>;
    async fn put(&self, key: &str, content_type: &str, body: Vec<u8>) -> Result<(), StorageError>;
    async fn delete(&self, key: &str) -> Result<(), StorageError>;
}

#[derive(Clone)]
pub struct ObjectStorage {
    backend: Arc<dyn StorageBackend>,
}

impl ObjectStorage {
    pub async fn from_env(environment: Environment) -> Result<Self, String> {
        let settings = StorageSettings::from_values(
            environment,
            nonempty_env("S3_ENDPOINT"),
            nonempty_env("S3_REGION"),
            nonempty_env("S3_BUCKET"),
            nonempty_env("S3_ACCESS_KEY_ID"),
            nonempty_env("S3_SECRET_ACCESS_KEY"),
        )?;
        let backend = S3CompatibleBackend::new(settings).await;
        Ok(Self {
            backend: Arc::new(backend),
        })
    }

    pub fn provider(&self) -> StorageProvider {
        self.backend.provider()
    }

    pub fn bucket(&self) -> &str {
        self.backend.bucket()
    }

    pub async fn presign_upload(
        &self,
        key: &str,
        content_type: &str,
        content_length: i64,
        expires_in: Duration,
    ) -> Result<String, StorageError> {
        self.backend
            .presign_upload(key, content_type, content_length, expires_in)
            .await
    }

    pub async fn presign_download(
        &self,
        key: &str,
        expires_in: Duration,
    ) -> Result<String, StorageError> {
        self.backend.presign_download(key, expires_in).await
    }

    pub async fn head(&self, key: &str) -> Result<ObjectMetadata, StorageError> {
        self.backend.head(key).await
    }

    pub async fn get(&self, key: &str) -> Result<StoredObject, StorageError> {
        self.backend.get(key).await
    }

    pub async fn put(
        &self,
        key: &str,
        content_type: &str,
        body: Vec<u8>,
    ) -> Result<(), StorageError> {
        self.backend.put(key, content_type, body).await
    }

    pub async fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.backend.delete(key).await
    }
}

struct S3CompatibleBackend {
    provider: StorageProvider,
    client: Client,
    bucket: String,
}

impl S3CompatibleBackend {
    async fn new(settings: StorageSettings) -> Self {
        let mut loader = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(settings.region.clone()));
        if let (Some(access_key), Some(secret_key)) =
            (settings.access_key.clone(), settings.secret_key.clone())
        {
            loader = loader.credentials_provider(Credentials::new(
                access_key,
                secret_key,
                None,
                None,
                "knitnprint-object-storage",
            ));
        }
        let shared = loader.load().await;
        let mut builder = aws_sdk_s3::config::Builder::from(&shared);
        if let Some(endpoint) = settings.endpoint {
            builder = builder.endpoint_url(endpoint).force_path_style(true);
        }
        Self {
            provider: settings.provider,
            client: Client::from_conf(builder.build()),
            bucket: settings.bucket,
        }
    }
}

#[async_trait]
impl StorageBackend for S3CompatibleBackend {
    fn provider(&self) -> StorageProvider {
        self.provider
    }

    fn bucket(&self) -> &str {
        &self.bucket
    }

    async fn presign_upload(
        &self,
        key: &str,
        content_type: &str,
        content_length: i64,
        expires_in: Duration,
    ) -> Result<String, StorageError> {
        let config = PresigningConfig::expires_in(expires_in).map_err(|_| StorageError::Presign)?;
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .content_length(content_length)
            .presigned(config)
            .await
            .map(|request| request.uri().to_string())
            .map_err(|_| StorageError::Presign)
    }

    async fn presign_download(
        &self,
        key: &str,
        expires_in: Duration,
    ) -> Result<String, StorageError> {
        let config = PresigningConfig::expires_in(expires_in).map_err(|_| StorageError::Presign)?;
        self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(config)
            .await
            .map(|request| request.uri().to_string())
            .map_err(|_| StorageError::Presign)
    }

    async fn head(&self, key: &str) -> Result<ObjectMetadata, StorageError> {
        self.client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map(|head| ObjectMetadata {
                content_length: head.content_length(),
                content_type: head.content_type().map(ToOwned::to_owned),
            })
            .map_err(|_| StorageError::Head)
    }

    async fn get(&self, key: &str) -> Result<StoredObject, StorageError> {
        let object = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|_| StorageError::Read)?;
        let content_type = object.content_type().map(ToOwned::to_owned);
        let etag = object.e_tag().map(ToOwned::to_owned);
        let bytes = object
            .body
            .collect()
            .await
            .map_err(|_| StorageError::ReadBody)?
            .into_bytes()
            .to_vec();
        Ok(StoredObject {
            bytes,
            content_type,
            etag,
        })
    }

    async fn put(&self, key: &str, content_type: &str, body: Vec<u8>) -> Result<(), StorageError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .body(ByteStream::from(body))
            .send()
            .await
            .map(|_| ())
            .map_err(|_| StorageError::Write)
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map(|_| ())
            .map_err(|_| StorageError::Delete)
    }
}

struct StorageSettings {
    provider: StorageProvider,
    endpoint: Option<String>,
    region: String,
    bucket: String,
    access_key: Option<String>,
    secret_key: Option<String>,
}

impl StorageSettings {
    fn from_values(
        environment: Environment,
        endpoint: Option<String>,
        region: Option<String>,
        bucket: Option<String>,
        access_key: Option<String>,
        secret_key: Option<String>,
    ) -> Result<Self, String> {
        if access_key.is_some() != secret_key.is_some() {
            return Err(
                "S3_ACCESS_KEY_ID and S3_SECRET_ACCESS_KEY must be configured together".into(),
            );
        }
        if environment.is_deployed() {
            if endpoint.is_some() {
                return Err(
                    "S3_ENDPOINT must be unset in staging and production so AWS S3 is used".into(),
                );
            }
            if access_key.is_some() {
                return Err(
                    "S3 access keys must be unset in staging and production; use the workload IAM role"
                        .into(),
                );
            }
            let region = region.ok_or("S3_REGION is required in staging and production")?;
            let bucket = bucket.ok_or("S3_BUCKET is required in staging and production")?;
            let required_prefix = match environment {
                Environment::Staging => "knitnprint-staging-",
                Environment::Production => "knitnprint-production-",
                Environment::Development | Environment::Test => unreachable!(),
            };
            if !bucket.starts_with(required_prefix) {
                let environment_name = required_prefix
                    .trim_start_matches("knitnprint-")
                    .trim_end_matches('-');
                return Err(format!(
                    "S3_BUCKET must start with {required_prefix} when APP_ENV is {environment_name}"
                ));
            }
            return Ok(Self {
                provider: StorageProvider::AwsS3,
                endpoint: None,
                region,
                bucket,
                access_key: None,
                secret_key: None,
            });
        }

        Ok(Self {
            provider: StorageProvider::Minio,
            endpoint: Some(endpoint.unwrap_or_else(|| DEFAULT_LOCAL_ENDPOINT.into())),
            region: region.unwrap_or_else(|| DEFAULT_LOCAL_REGION.into()),
            bucket: bucket.unwrap_or_else(|| DEFAULT_LOCAL_BUCKET.into()),
            access_key: Some(access_key.unwrap_or_else(|| DEFAULT_LOCAL_ACCESS_KEY.into())),
            secret_key: Some(secret_key.unwrap_or_else(|| DEFAULT_LOCAL_SECRET_KEY.into())),
        })
    }
}

fn nonempty_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::config::Environment;

    use super::{
        DEFAULT_LOCAL_BUCKET, DEFAULT_LOCAL_ENDPOINT, S3CompatibleBackend, StorageBackend,
        StorageProvider, StorageSettings,
    };

    #[test]
    fn development_defaults_to_minio() {
        let settings =
            StorageSettings::from_values(Environment::Development, None, None, None, None, None)
                .unwrap();
        assert_eq!(settings.provider, StorageProvider::Minio);
        assert_eq!(settings.endpoint.as_deref(), Some(DEFAULT_LOCAL_ENDPOINT));
        assert_eq!(settings.bucket, DEFAULT_LOCAL_BUCKET);
        assert!(settings.access_key.is_some());
        assert!(settings.secret_key.is_some());
    }

    #[test]
    fn staging_requires_aws_s3_settings() {
        let missing =
            StorageSettings::from_values(Environment::Staging, None, None, None, None, None)
                .err()
                .expect("deployed storage without a Region must fail");
        assert!(missing.contains("S3_REGION"));

        let settings = StorageSettings::from_values(
            Environment::Staging,
            None,
            Some("eu-west-1".into()),
            Some("knitnprint-staging-media".into()),
            None,
            None,
        )
        .unwrap();
        assert_eq!(settings.provider, StorageProvider::AwsS3);
        assert!(settings.endpoint.is_none());
        assert!(settings.access_key.is_none());
    }

    #[test]
    fn deployed_environments_reject_minio_and_static_credentials() {
        let endpoint = StorageSettings::from_values(
            Environment::Production,
            Some("http://minio:9000".into()),
            Some("eu-west-1".into()),
            Some("knitnprint-production-media".into()),
            None,
            None,
        )
        .err()
        .expect("deployed storage with a custom endpoint must fail");
        assert!(endpoint.contains("S3_ENDPOINT"));

        let credentials = StorageSettings::from_values(
            Environment::Production,
            None,
            Some("eu-west-1".into()),
            Some("knitnprint-production-media".into()),
            Some("key".into()),
            Some("secret".into()),
        )
        .err()
        .expect("deployed storage with static credentials must fail");
        assert!(credentials.contains("workload IAM role"));
    }

    #[test]
    fn deployed_environments_require_environment_scoped_buckets() {
        let staging = StorageSettings::from_values(
            Environment::Staging,
            None,
            Some("eu-west-1".into()),
            Some("knitnprint-production-media-123456789012".into()),
            None,
            None,
        )
        .err()
        .expect("staging must reject a production bucket");
        assert!(staging.contains("knitnprint-staging-"));

        let production = StorageSettings::from_values(
            Environment::Production,
            None,
            Some("eu-west-1".into()),
            Some("knitnprint-staging-media-123456789012".into()),
            None,
            None,
        )
        .err()
        .expect("production must reject a staging bucket");
        assert!(production.contains("knitnprint-production-"));
    }

    #[test]
    fn credentials_must_always_be_a_pair() {
        let error = StorageSettings::from_values(
            Environment::Development,
            None,
            None,
            None,
            Some("key".into()),
            None,
        )
        .err()
        .expect("a partial credential pair must fail");
        assert!(error.contains("configured together"));
    }

    #[tokio::test]
    async fn development_presigned_urls_target_minio() {
        let settings =
            StorageSettings::from_values(Environment::Development, None, None, None, None, None)
                .unwrap();
        let storage = S3CompatibleBackend::new(settings).await;
        let upload = storage
            .presign_upload(
                "uploads-quarantine/example/original.png",
                "image/png",
                128,
                Duration::from_secs(300),
            )
            .await
            .unwrap();
        let download = storage
            .presign_download(
                "media-private/example/detail.webp",
                Duration::from_secs(300),
            )
            .await
            .unwrap();

        for url in [upload, download] {
            assert!(url.starts_with("http://127.0.0.1:9100/knitnprint-media/"));
            assert!(url.contains("X-Amz-Algorithm=AWS4-HMAC-SHA256"));
            assert!(url.contains("X-Amz-Expires=300"));
        }
    }
}

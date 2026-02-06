pub mod proto;

use crate::proto::public_archive::public_archive_service_client::PublicArchiveServiceClient;
use crate::proto::public_archive::{GetPublicArchiveRequest, UpdatePublicArchiveRequest, File};
use async_trait::async_trait;
use libunftp::auth::UserDetail;
use libunftp::storage::{Fileinfo, Metadata, Permissions, Result, StorageBackend, Error, ErrorKind};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::io::AsyncReadExt;
use tokio::sync::RwLock;
use tonic::transport::Channel;

pub mod ext;
pub use ext::ServerExt;

#[derive(Debug, Clone)]
pub struct Anttp {
    client: PublicArchiveServiceClient<Channel>,
    address: Arc<RwLock<String>>,
    store_type: Option<String>,
}

impl Anttp {
    pub fn new(address: String) -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let endpoint = std::env::var("ANTTP_GRPC_ENDPOINT").unwrap_or_else(|_| "http://localhost:18887".to_string());
        let channel = tonic::transport::Channel::from_shared(endpoint)?.connect_lazy();
        let client = PublicArchiveServiceClient::new(channel);
        let store_type = Some("memory".to_string());
        Ok(Anttp {
            client,
            address: Arc::new(RwLock::new(address)),
            store_type,
        })
    }
}

#[derive(Debug)]
pub struct Meta {
    len: u64,
    is_dir: bool,
    modified: Option<SystemTime>,
}

#[async_trait]
impl<User: UserDetail> StorageBackend<User> for Anttp {
    type Metadata = Meta;

    fn supported_features(&self) -> u32 {
        0
    }

    async fn metadata<P: AsRef<Path> + Send + Debug>(&self, _user: &User, path: P) -> Result<Self::Metadata> {
        let path_str = path.as_ref().to_string_lossy().into_owned();
        let mut client = self.client.clone();
        let address = self.address.read().await.clone();
        let request = tonic::Request::new(GetPublicArchiveRequest {
            address,
            path: path_str,
            store_type: self.store_type.clone(),
        });

        let response = client.get_public_archive(request).await
            .map_err(|e| Error::new(ErrorKind::PermanentFileNotAvailable, e))?;
        
        let inner = response.into_inner();
        
        Ok(Meta {
            len: inner.content.as_ref().map(|c| c.len() as u64).unwrap_or(0),
            is_dir: !inner.items.is_empty(),
            modified: None, // GetPublicArchiveResponse doesn't have a top-level modified date yet
        })
    }

    async fn list<P>(&self, _user: &User, path: P) -> Result<Vec<Fileinfo<PathBuf, Self::Metadata>>>
    where
        P: AsRef<Path> + Send + Debug,
    {
        let path_str = path.as_ref().to_string_lossy().into_owned();
        let mut client = self.client.clone();
        let address = self.address.read().await.clone();
        let request = tonic::Request::new(GetPublicArchiveRequest {
            address,
            path: path_str,
            store_type: self.store_type.clone(),
        });

        let response = client.get_public_archive(request).await
            .map_err(|e| Error::new(ErrorKind::PermanentFileNotAvailable, e))?;
        
        let inner = response.into_inner();
        let mut fis = Vec::new();
        for item in inner.items {
            let item_path = PathBuf::from(&item.name);
            fis.push(Fileinfo {
                path: item_path,
                metadata: Meta {
                    len: item.size, 
                    is_dir: item.r#type == "directory",
                    modified: Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(item.modified)),
                },
            });
        }

        Ok(fis)
    }

    async fn get<P: AsRef<Path> + Send + Debug>(&self, _user: &User, path: P, _start_pos: u64) -> Result<Box<dyn tokio::io::AsyncRead + Send + Sync + Unpin>> {
        let path_str = path.as_ref().to_string_lossy().into_owned();
        let mut client = self.client.clone();
        let address = self.address.read().await.clone();
        let request = tonic::Request::new(GetPublicArchiveRequest {
            address,
            path: path_str,
            store_type: self.store_type.clone(),
        });

        let response = client.get_public_archive(request).await
            .map_err(|e| Error::new(ErrorKind::PermanentFileNotAvailable, e))?;
        
        let inner = response.into_inner();
        let content = inner.content.ok_or_else(|| Error::from(ErrorKind::PermanentFileNotAvailable))?;
        
        Ok(Box::new(std::io::Cursor::new(content)) as Box<dyn tokio::io::AsyncRead + Send + Sync + Unpin>)
    }

    async fn put<P: AsRef<Path> + Send, R: tokio::io::AsyncRead + Send + Sync + 'static + Unpin>(
        &self,
        _user: &User,
        mut bytes: R,
        path: P,
        _start_pos: u64,
    ) -> Result<u64> {
        let mut content = Vec::new();
        bytes.read_to_end(&mut content).await
            .map_err(|e| Error::new(ErrorKind::LocalError, e))?;
        let len = content.len() as u64;

        let path_ref = path.as_ref();
        let path_str = path_ref.to_string_lossy().into_owned();
        let filename = path_ref.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let mut client = self.client.clone();
        
        let mut address_guard = self.address.write().await;
        let request = tonic::Request::new(UpdatePublicArchiveRequest {
            address: address_guard.clone(),
            files: vec![File {
                name: filename,
                content,
            }],
            path: Some(path_str),
            store_type: self.store_type.clone(),
        });

        let response = client.update_public_archive(request).await
            .map_err(|e| Error::new(ErrorKind::PermanentFileNotAvailable, e))?;

        if let Some(new_address) = response.into_inner().address {
            *address_guard = new_address;
        }

        Ok(len)
    }

    async fn del<P: AsRef<Path> + Send + Debug>(&self, _user: &User, _path: P) -> Result<()> {
        Err(Error::from(ErrorKind::CommandNotImplemented))
    }

    async fn rmd<P: AsRef<Path> + Send + Debug>(&self, _user: &User, _path: P) -> Result<()> {
        Err(Error::from(ErrorKind::CommandNotImplemented))
    }

    async fn mkd<P: AsRef<Path> + Send + Debug>(&self, _user: &User, path: P) -> Result<()> {
        let path_str = path.as_ref().to_string_lossy().into_owned();
        
        let mut client = self.client.clone();
        let mut address_guard = self.address.write().await;
        
        let request = tonic::Request::new(UpdatePublicArchiveRequest {
            address: address_guard.clone(),
            files: vec![File {
                name: ".metadata".to_string(),
                content: Vec::new(),
            }],
            path: Some(path_str),
            store_type: self.store_type.clone(),
        });

        let response = client.update_public_archive(request).await
            .map_err(|e| Error::new(ErrorKind::PermanentFileNotAvailable, e))?;
        
        if let Some(new_address) = response.into_inner().address {
            *address_guard = new_address;
        }

        Ok(())
    }

    async fn rename<P: AsRef<Path> + Send + Debug>(&self, _user: &User, _from: P, _to: P) -> Result<()> {
        Err(Error::from(ErrorKind::CommandNotImplemented))
    }

    async fn cwd<P: AsRef<Path> + Send + Debug>(&self, _user: &User, _path: P) -> Result<()> {
        Ok(())
    }
}

impl Metadata for Meta {
    fn len(&self) -> u64 {
        self.len
    }

    fn is_dir(&self) -> bool {
        self.is_dir
    }

    fn is_file(&self) -> bool {
        !self.is_dir
    }

    fn is_symlink(&self) -> bool {
        false
    }

    fn modified(&self) -> Result<SystemTime> {
        self.modified.ok_or_else(|| Error::from(ErrorKind::PermanentFileNotAvailable))
            .or_else(|_| Ok(SystemTime::now()))
    }

    fn gid(&self) -> u32 {
        0
    }

    fn uid(&self) -> u32 {
        0
    }

    fn links(&self) -> u64 {
        1
    }

    fn permissions(&self) -> Permissions {
        Permissions(0o7755)
    }

    fn readlink(&self) -> Option<&Path> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_anttp_new() {
        // Just verify it doesn't panic during construction if possible,
        // though it currently tries to connect immediately.
        // We use a dummy address.
        let addr = "efdcdc93db39d5ffef254f9bb3e069fc6315a1054f20a8b00343629f7773663b".to_string();
        let _ = Anttp::new(addr);
    }

    #[tokio::test]
    async fn test_meta_new() {
        let now = SystemTime::now();
        let meta = Meta {
            len: 100,
            is_dir: true,
            modified: Some(now),
        };
        assert_eq!(meta.len(), 100);
        assert!(meta.is_dir());
        assert!(!meta.is_file());
        assert_eq!(meta.modified().unwrap(), now);
    }

    #[tokio::test]
    async fn test_del_not_implemented() {
        // DEL is currently not implemented, we verify it returns CommandNotImplemented.
        let addr = "efdcdc93db39d5ffef254f9bb3e069fc6315a1054f20a8b00343629f7773663b".to_string();
        let anttp = Anttp::new(addr).unwrap();
        let user = libunftp::auth::DefaultUser {};
        let result: Result<()> = anttp.del(&user, "some/path").await;
        match result {
            Err(e) => assert_eq!(e.kind(), ErrorKind::CommandNotImplemented),
            _ => panic!("Expected CommandNotImplemented"),
        }
    }
}

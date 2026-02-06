mod proto;

use crate::proto::public_archive::public_archive_service_client::PublicArchiveServiceClient;
use crate::proto::public_archive::{GetPublicArchiveRequest};
use async_trait::async_trait;
use libunftp::auth::UserDetail;
use libunftp::storage::{Fileinfo, Metadata, Permissions, Result, StorageBackend, Error, ErrorKind};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tonic::transport::Channel;

pub mod ext;
pub use ext::ServerExt;

#[derive(Debug, Clone)]
pub struct Anttp {
    client: PublicArchiveServiceClient<Channel>,
    address: String,
}

impl Anttp {
    pub async fn new(address: String) -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let client = PublicArchiveServiceClient::connect("http://localhost:18887").await?;
        Ok(Anttp { client, address })
    }
}

#[derive(Debug)]
pub struct Meta {
    len: u64,
    is_dir: bool,
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
        let request = tonic::Request::new(GetPublicArchiveRequest {
            address: self.address.clone(),
            path: path_str,
            store_type: None,
        });

        let response = client.get_public_archive(request).await
            .map_err(|e| Error::new(ErrorKind::PermanentFileNotAvailable, e))?;
        
        let inner = response.into_inner();
        
        Ok(Meta {
            len: inner.content.as_ref().map(|c| c.len() as u64).unwrap_or(0),
            is_dir: !inner.items.is_empty(),
        })
    }

    async fn list<P>(&self, _user: &User, path: P) -> Result<Vec<Fileinfo<PathBuf, Self::Metadata>>>
    where
        P: AsRef<Path> + Send + Debug,
    {
        let path_str = path.as_ref().to_string_lossy().into_owned();
        let mut client = self.client.clone();
        let request = tonic::Request::new(GetPublicArchiveRequest {
            address: self.address.clone(),
            path: path_str,
            store_type: None,
        });

        let response = client.get_public_archive(request).await
            .map_err(|e| Error::new(ErrorKind::PermanentFileNotAvailable, e))?;
        
        let inner = response.into_inner();
        let mut fis = Vec::new();
        for item in inner.items {
            let item_path = PathBuf::from(&item);
            // We don't have full metadata for each item in the list response, 
            // so we might need to fetch it or use defaults.
            // For now, let's assume if it's in the list, it's a file/dir.
            fis.push(Fileinfo {
                path: item_path,
                metadata: Meta {
                    len: 0, 
                    is_dir: false, // Need more info from AntTP if we want to distinguish here
                },
            });
        }

        Ok(fis)
    }

    async fn get<P: AsRef<Path> + Send + Debug>(&self, _user: &User, path: P, _start_pos: u64) -> Result<Box<dyn tokio::io::AsyncRead + Send + Sync + Unpin>> {
        let path_str = path.as_ref().to_string_lossy().into_owned();
        let mut client = self.client.clone();
        let request = tonic::Request::new(GetPublicArchiveRequest {
            address: self.address.clone(),
            path: path_str,
            store_type: None,
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
        _bytes: R,
        _path: P,
        _start_pos: u64,
    ) -> Result<u64> {
        Err(Error::from(ErrorKind::CommandNotImplemented))
    }

    async fn del<P: AsRef<Path> + Send + Debug>(&self, _user: &User, _path: P) -> Result<()> {
        Err(Error::from(ErrorKind::CommandNotImplemented))
    }

    async fn rmd<P: AsRef<Path> + Send + Debug>(&self, _user: &User, _path: P) -> Result<()> {
        Err(Error::from(ErrorKind::CommandNotImplemented))
    }

    async fn mkd<P: AsRef<Path> + Send + Debug>(&self, _user: &User, _path: P) -> Result<()> {
        Err(Error::from(ErrorKind::CommandNotImplemented))
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
        Ok(SystemTime::now())
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
        let _ = Anttp::new(addr).await;
    }
}

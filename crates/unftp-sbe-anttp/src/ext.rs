use crate::Filesystem;
use libunftp::auth::DefaultUser;
use libunftp::{Server, ServerBuilder};
use std::path::PathBuf;

/// Extension trait purely for construction convenience.
pub trait ServerExt {
    /// Create a new `Server` with the given Autonomi archive root pointer address.
    ///
    /// # Example
    ///
    /// ```rust
    /// use libunftp::Server;
    /// use unftp_sbe_anttp::ServerExt;
    ///
    /// let server = Server::with_anttp("<address>");
    /// ```
    fn with_anttp<P: Into<PathBuf> + Send + 'static>(path: P) -> ServerBuilder<Filesystem, DefaultUser> {
        let p = path.into();
        libunftp::ServerBuilder::new(Box::new(move || {
            let p = &p.clone();
            match Filesystem::new(p) {
                Ok(fs) => fs,
                Err(e) => panic!("Cannot open file system root {}: {}", p.display(), e),
            }
        }))
    }
}

impl ServerExt for Server<Filesystem, DefaultUser> {}

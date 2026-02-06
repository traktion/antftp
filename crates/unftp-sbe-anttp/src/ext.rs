use crate::Anttp;
use libunftp::auth::DefaultUser;
use libunftp::{Server, ServerBuilder};

/// Extension trait purely for construction convenience.
pub trait ServerExt {
    /// Create a new `Server` with the given AntTP address.
    ///
    /// # Example
    ///
    /// ```rust
    /// use libunftp::Server;
    /// use unftp_sbe_anttp::ServerExt;
    ///
    /// let server = Server::with_anttp();
    /// ```
    fn with_anttp() -> ServerBuilder<Anttp, DefaultUser> {
        libunftp::ServerBuilder::new(Box::new(move || {
            tokio::runtime::Handle::current().block_on(async {
                Anttp::new().await.expect("Cannot connect to AntTP")
            })
        }))
    }
}

impl ServerExt for Server<Anttp, DefaultUser> {}

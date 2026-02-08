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
    /// let server = Server::with_anttp("some_address");
    /// ```
    fn with_anttp(address: impl Into<String>) -> ServerBuilder<Anttp, DefaultUser> {
        let address = address.into();
        libunftp::ServerBuilder::new(Box::new(move || {
            let address = address.clone();
            Anttp::new(address).expect("Cannot connect to AntTP")
        }))
    }

    /// Create a new `Server` with the given AntTP address and optional pointer name.
    fn with_anttp_pointer(address: impl Into<String>, pointer_name: Option<String>) -> ServerBuilder<Anttp, DefaultUser> {
        let address = address.into();
        libunftp::ServerBuilder::new(Box::new(move || {
            let address = address.clone();
            Anttp::new_with_pointer(address, pointer_name.clone()).expect("Cannot connect to AntTP")
        }))
    }
}

impl ServerExt for Server<Anttp, DefaultUser> {}

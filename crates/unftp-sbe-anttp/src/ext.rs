use crate::Anttp;
use libunftp::auth::DefaultUser;
use libunftp::{Server, ServerBuilder};

use crate::proto::pointer::pointer_service_client::PointerServiceClient;
use tonic::transport::Channel;

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

    /// Create a new `Server` with the given AntTP address and pointer client.
    fn with_anttp_pointer(address: impl Into<String>, pointer_client: PointerServiceClient<Channel>, pointer_name: String) -> ServerBuilder<Anttp, DefaultUser> {
        let address = address.into();
        libunftp::ServerBuilder::new(Box::new(move || {
            let address = address.clone();
            Anttp::new_with_pointer(address, pointer_client.clone(), pointer_name.clone()).expect("Cannot connect to AntTP")
        }))
    }
}

impl ServerExt for Server<Anttp, DefaultUser> {}

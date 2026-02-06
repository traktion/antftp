use unftp_sbe_anttp::ServerExt;

#[tokio::main]
pub async fn main() {
    let server = libunftp::Server::with_anttp("efdcdc93db39d5ffef254f9bb3e069fc6315a1054f20a8b00343629f7773663b")
        .greeting("Welcome to ANT FTP server")
        .passive_ports(50000..=65535)
        .build()
        .unwrap();

    server.listen("127.0.0.1:2121").await.expect("Failed to start FTP listener");
}
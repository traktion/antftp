use unftp_sbe_anttp::ServerExt;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The AntTP archive hash to use
    #[arg(short, long, default_value = "efdcdc93db39d5ffef254f9bb3e069fc6315a1054f20a8b00343629f7773663b")]
    archive: String,

    /// The listen address for the FTP server (e.g., 127.0.0.1:2121)
    #[arg(short = 'l', long = "listen-address", default_value = "127.0.0.1:2121")]
    listen_address: String,
}

#[tokio::main]
pub async fn main() {
    let args = Args::parse();

    let server = libunftp::Server::with_anttp(&args.archive)
        .greeting("Welcome to ANT FTP server")
        .passive_ports(50000..=65535)
        .build()
        .unwrap();

    server.listen(&args.listen_address).await.expect("Failed to start FTP listener");
}
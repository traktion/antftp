use unftp_sbe_anttp::ServerExt;
use clap::Parser;
use tonic::transport::Channel;
use unftp_sbe_anttp::proto::pointer::pointer_service_client::PointerServiceClient;
use unftp_sbe_anttp::proto::pointer::GetPointerRequest;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The AntTP archive hash to use
    #[arg(short, long, default_value = "efdcdc93db39d5ffef254f9bb3e069fc6315a1054f20a8b00343629f7773663b")]
    archive: String,

    /// Optional pointer name to resolve the archive address from AntTP
    #[arg(short = 'p', long = "pointer-name")]
    pointer_name: Option<String>,

    /// The listen address for the FTP server (e.g., 127.0.0.1:2121)
    #[arg(short = 'l', long = "listen-address", default_value = "127.0.0.1:2121")]
    listen_address: String,
}

#[tokio::main]
pub async fn main() {
    let mut args = Args::parse();

    // If a pointer name is provided, resolve the archive address from AntTP
    if let Some(ref pointer_name) = args.pointer_name {
        let endpoint = std::env::var("ANTTP_GRPC_ENDPOINT").unwrap_or_else(|_| "http://localhost:18887".to_string());
        let channel = Channel::from_shared(endpoint).expect("invalid endpoint").connect_lazy();
        let mut client = PointerServiceClient::new(channel);
        let req = tonic::Request::new(GetPointerRequest { address: pointer_name.clone() });
        match client.get_pointer(req).await {
            Ok(resp) => {
                if let Some(pointer) = resp.into_inner().pointer {
                    args.archive = pointer.content;
                }
            }
            Err(e) => {
                eprintln!("Failed to resolve pointer '{}': {}. Falling back to provided archive.", pointer_name, e);
            }
        }
    }

    // Use the pointer-aware server builder when a pointer was specified; otherwise the default
    let server = if args.pointer_name.is_some() {
        libunftp::Server::with_anttp_pointer(&args.archive, args.pointer_name.clone())
    } else {
        libunftp::Server::with_anttp(&args.archive)
    }
    .greeting("Welcome to ANT FTP server")
    .passive_ports(50000..=65535)
    .build()
    .unwrap();

    server.listen(&args.listen_address).await.expect("Failed to start FTP listener");
}
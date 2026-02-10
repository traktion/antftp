use unftp_sbe_anttp::ServerExt;
use clap::Parser;
use tonic::transport::Channel;
use unftp_sbe_anttp::proto::pointer::pointer_service_client::PointerServiceClient;
use unftp_sbe_anttp::proto::pointer::{GetPointerRequest, UpdatePointerRequest, Pointer};
use unftp_sbe_anttp::proto::public_archive::public_archive_service_client::PublicArchiveServiceClient;
use unftp_sbe_anttp::proto::public_archive::PushPublicArchiveRequest;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};

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

    /// Network sync interval in minutes (only used when a pointer is provided)
    #[arg(short = 'n', long = "network-sync-timer", default_value = "10")]
    network_sync_timer: u64,
}

#[tokio::main]
pub async fn main() {
    let args = Args::parse();

    // Use the pointer-aware server builder when a pointer was specified; otherwise the default
    let server = if let Some(ref pointer_name) = args.pointer_name {
        let endpoint = std::env::var("ANTTP_GRPC_ENDPOINT").unwrap_or_else(|_| "http://localhost:18887".to_string());
        let channel = Channel::from_shared(endpoint.clone()).expect("Invalid endpoint").connect_lazy();
        let pointer_client = PointerServiceClient::new(channel.clone());

        // Start background network sync job (only when pointer provided)
        let pointer_name_cloned = pointer_name.clone();
        let sync_minutes = args.network_sync_timer;
        tokio::spawn(async move {
            let channel_bg = Channel::from_shared(endpoint).expect("Invalid endpoint").connect_lazy();
            let mut pointer_client_bg = PointerServiceClient::new(channel_bg.clone());
            let mut archive_client_bg = PublicArchiveServiceClient::new(channel_bg);
            let last_synced: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

            let mut interval = time::interval(Duration::from_secs(sync_minutes * 60));
            loop {
                interval.tick().await;
                // Read current archive address from disk (via pointer)
                let req = tonic::Request::new(GetPointerRequest { address: pointer_name_cloned.clone(), data_key: None });
                match pointer_client_bg.get_pointer(req).await {
                    Ok(resp) => {
                        if let Some(ptr) = resp.into_inner().pointer {
                            let current_addr = ptr.content;
                            let mut last = last_synced.lock().await;
                            if last.as_ref() != Some(&current_addr) {
                                // Push archive to network
                                let push_req = tonic::Request::new(PushPublicArchiveRequest { address: current_addr.clone(), store_type: Some("network".to_string()) });
                                if let Err(e) = archive_client_bg.push_public_archive(push_req).await {
                                    eprintln!("Network sync: failed to push archive: {}", e);
                                    continue;
                                }
                                // Update pointer on network
                                let up_req = tonic::Request::new(UpdatePointerRequest {
                                    address: pointer_name_cloned.clone(),
                                    pointer: Some(Pointer { name: Some(pointer_name_cloned.clone()), content: current_addr.clone(), address: None, counter: None, cost: None }),
                                    store_type: Some("network".to_string()),
                                    data_key: None,
                                });
                                if let Err(e) = pointer_client_bg.update_pointer(up_req).await {
                                    eprintln!("Network sync: failed to update pointer on network: {}", e);
                                    continue;
                                }
                                *last = Some(current_addr);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Network sync: failed to get pointer: {}", e);
                    }
                }
            }
        });

        libunftp::Server::with_anttp_pointer(&args.archive, pointer_client, pointer_name.clone())
    } else {
        libunftp::Server::with_anttp(&args.archive)
    }
    .greeting("Welcome to ANT FTP server")
    .passive_ports(50000..=65535)
    .build()
    .unwrap();

    server.listen(&args.listen_address).await.expect("Failed to start FTP listener");
}

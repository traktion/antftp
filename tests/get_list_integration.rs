use std::net::{TcpListener};
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::TcpListenerStream;
use suppaftp::AsyncFtpStream;
use unftp_sbe_anttp::ServerExt;
use serial_test::serial;

// Generated from proto (provided by unftp-sbe-anttp crate)
use unftp_sbe_anttp::proto::archive::archive_service_server::{ArchiveService, ArchiveServiceServer};
use unftp_sbe_anttp::proto::archive::{GetArchiveRequest, ArchiveResponse, Item};
use tonic::{Request, Response, Status};

struct MockArchiveService;

#[tonic::async_trait]
impl ArchiveService for MockArchiveService {
    async fn create_archive(
        &self,
        _request: Request<unftp_sbe_anttp::proto::archive::CreateArchiveRequest>,
    ) -> Result<Response<unftp_sbe_anttp::proto::archive::ArchiveResponse>, Status> {
        Err(Status::unimplemented("not needed for this test"))
    }

    async fn push_archive(
        &self,
        request: Request<unftp_sbe_anttp::proto::archive::PushArchiveRequest>,
    ) -> Result<Response<unftp_sbe_anttp::proto::archive::ArchiveResponse>, Status> {
        // Mock simply echoes back the same address as confirmation
        let req = request.into_inner();
        Ok(Response::new(unftp_sbe_anttp::proto::archive::ArchiveResponse {
            address: Some(req.address),
            items: vec![],
            content: None,
        }))
    }

    async fn update_archive(
        &self,
        request: Request<unftp_sbe_anttp::proto::archive::UpdateArchiveRequest>,
    ) -> Result<Response<unftp_sbe_anttp::proto::archive::ArchiveResponse>, Status> {
        let req = request.into_inner();
        
        // Basic validation of the new proto structure
        if req.path.is_none() {
            return Err(Status::invalid_argument("path is required in the new proto"));
        }

        let path = req.path.as_ref().unwrap();
        if path.ends_with("new_file.txt") {
            let file = &req.files[0];
            if file.name != "new_file.txt" {
                return Err(Status::invalid_argument(format!("Expected filename 'new_file.txt', got '{}'", file.name)));
            }
        }

        let mut new_address = req.address;
        new_address.push_str("_updated");
        Ok(Response::new(unftp_sbe_anttp::proto::archive::ArchiveResponse {
            address: Some(new_address),
            items: vec![],
            content: None,
        }))
    }

    async fn truncate_archive(
        &self,
        request: Request<unftp_sbe_anttp::proto::archive::TruncateArchiveRequest>,
    ) -> Result<Response<unftp_sbe_anttp::proto::archive::ArchiveResponse>, Status> {
        let req = request.into_inner();
        let mut new_address = req.address;
        new_address.push_str("_truncated");
        Ok(Response::new(unftp_sbe_anttp::proto::archive::ArchiveResponse {
            address: Some(new_address),
            items: vec![],
            content: None,
        }))
    }

    async fn get_archive(
        &self,
        request: Request<GetArchiveRequest>,
    ) -> Result<Response<ArchiveResponse>, Status> {
        let req = request.into_inner();
        let path = req.path.unwrap_or_default();
        // Root listing
        if path.is_empty() || path == "/" || path == "." {
            Ok(Response::new(ArchiveResponse {
                address: Some(req.address.clone()),
                items: vec![
                    Item {
                        name: "file1.txt".to_string(),
                        size: 11,
                        modified: 0,
                        r#type: "file".to_string(),
                    },
                    Item {
                        name: "dir".to_string(),
                        size: 0,
                        modified: 0,
                        r#type: "directory".to_string(),
                    },
                ],
                content: None,
            }))
        } else if path == "/file1.txt" || path == "file1.txt" {
            Ok(Response::new(ArchiveResponse {
                address: Some(req.address.clone()),
                items: vec![],
                content: Some(b"hello world".to_vec()),
            }))
        } else {
            // Unknown path
            return Err(Status::not_found("File not found"));
        }
    }
}

async fn start_mock_grpc() -> (String, JoinHandle<()>) {
    let std_listener = TcpListener::bind("127.0.0.1:0").expect("bind mock grpc");
    std_listener.set_nonblocking(true).expect("nonblocking");
    let addr = std_listener.local_addr().unwrap();
    let incoming = TcpListenerStream::new(tokio::net::TcpListener::from_std(std_listener).unwrap());

    let svc = ArchiveServiceServer::new(MockArchiveService);
    let handle = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(svc)
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });

    (format!("http://{}", addr), handle)
}

fn start_ftp_server(archive: &str, addr: &str) -> std::thread::JoinHandle<()> {
    let server = libunftp::Server::with_anttp(archive)
        .greeting("Welcome to ANT FTP server")
        .passive_ports(50000..=65535)
        .build()
        .unwrap();
    let addr = addr.to_string();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            server.listen(&addr).await.unwrap();
        });
    })
}

#[tokio::test]
#[serial]
async fn integration_list_and_get() {
    // 1) Start mock gRPC
    let (grpc_endpoint, _grpc_handle) = start_mock_grpc().await;
    unsafe { std::env::set_var("ANTTP_GRPC_ENDPOINT", &grpc_endpoint); }

    // 2) Pick random FTP port
    let ftp_listener = TcpListener::bind("127.0.0.1:0").expect("bind ftp");
    let ftp_addr = ftp_listener.local_addr().unwrap();
    drop(ftp_listener); // release so libunftp can bind
    let ftp_addr_str = format!("{}:{}", ftp_addr.ip(), ftp_addr.port());

    // 3) Start FTP server
    let _ftp_handle = start_ftp_server(
        "cec7a9eb2c644b9a5de58bbcdf2e893db9f0b2acd7fc563fc849e19d1f6bd872",
        &ftp_addr_str,
    );

    // 4) Give server a moment to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 5) Connect with suppaftp
    let addr = format!("{}:{}", ftp_addr.ip(), ftp_addr.port());
    let mut ftp_stream = AsyncFtpStream::connect(addr).await.expect("connect ftp");
    ftp_stream.login("anonymous", "anonymous").await.expect("login");

    // Root listing
    let list = ftp_stream.nlst(None).await.expect("nlst");
    assert!(list.iter().any(|item| item == "file1.txt"));

    // 7) Retrieve file
    let mut stream = ftp_stream.retr_as_stream("file1.txt").await.expect("retr_as_stream");
    let mut data = Vec::new();
    use async_std::io::ReadExt as _;
    stream.read_to_end(&mut data).await.expect("read_to_end");
    drop(stream);
    assert_eq!(data, b"hello world");

    ftp_stream.quit().await.ok();
}

#[tokio::test]
#[serial]
async fn integration_put_and_mkd() {
    // 1) Start mock gRPC
    let (grpc_endpoint, _grpc_handle) = start_mock_grpc().await;
    unsafe { std::env::set_var("ANTTP_GRPC_ENDPOINT", &grpc_endpoint); }

    // 2) Pick random FTP port
    let ftp_listener = TcpListener::bind("127.0.0.1:0").expect("bind ftp");
    let ftp_addr = ftp_listener.local_addr().unwrap();
    drop(ftp_listener); // release so libunftp can bind
    let ftp_addr_str = format!("{}:{}", ftp_addr.ip(), ftp_addr.port());

    // 3) Start FTP server
    let initial_address = "cec7a9eb2c644b9a5de58bbcdf2e893db9f0b2acd7fc563fc849e19d1f6bd872";
    let _ftp_handle = start_ftp_server(
        initial_address,
        &ftp_addr_str,
    );

    // 4) Give server a moment to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 5) Connect with suppaftp
    let mut ftp_stream = AsyncFtpStream::connect(&ftp_addr_str).await.expect("connect ftp");
    ftp_stream.login("anonymous", "anonymous").await.expect("login");

    // 6) Test MKD
    ftp_stream.mkdir("new_dir").await.expect("mkdir");
    
    // 7) Test PUT
    let content = b"new file content";
    let mut reader = content.as_slice();
    ftp_stream.put_file("new_file.txt", &mut reader).await.expect("put_file");

    ftp_stream.quit().await.ok();
}

#[tokio::test]
#[serial]
async fn integration_del_and_rmd() {
    // 1) Start mock gRPC
    let (grpc_endpoint, _grpc_handle) = start_mock_grpc().await;
    unsafe { std::env::set_var("ANTTP_GRPC_ENDPOINT", &grpc_endpoint); }

    // 2) Pick random FTP port
    let ftp_listener = TcpListener::bind("127.0.0.1:0").expect("bind ftp");
    let ftp_addr = ftp_listener.local_addr().unwrap();
    drop(ftp_listener); // release so libunftp can bind
    let ftp_addr_str = format!("{}:{}", ftp_addr.ip(), ftp_addr.port());

    // 3) Start FTP server
    let initial_address = "cec7a9eb2c644b9a5de58bbcdf2e893db9f0b2acd7fc563fc849e19d1f6bd872";
    let _ftp_handle = start_ftp_server(
        initial_address,
        &ftp_addr_str,
    );

    // 4) Give server a moment to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 5) Connect with suppaftp
    let mut ftp_stream = AsyncFtpStream::connect(&ftp_addr_str).await.expect("connect ftp");
    ftp_stream.login("anonymous", "anonymous").await.expect("login");

    // 6) Test DELE
    ftp_stream.rm("file1.txt").await.expect("rm file");
    
    // 7) Test RMD
    ftp_stream.rmdir("dir").await.expect("rmdir");

    ftp_stream.quit().await.ok();
}
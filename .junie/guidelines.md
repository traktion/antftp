### Project Overview
`antftp` is an FTP server skeleton designed to interact with **AntTP** via gRPC and Protobuf.

### Build/Configuration Instructions
- **Rust Version**: The project uses the 2024 edition.
- **Dependencies**: 
    - `libunftp`: The core FTP server engine.
- **gRPC/Protobuf Support**:
    - The project uses `tonic` and `prost` for gRPC/Protobuf.
    - **Requirement**: `protoc` must be installed on your system.
    - If `protoc` is missing, `build.rs` will emit a warning and disable gRPC features via the `grpc_disabled` cfg flag.
    - Protos are located in the `proto/` directory.

### Testing Information
#### Running Tests
- To run all tests in the workspace:
  ```bash
  cargo test
  ```
- To run tests for the storage backend crate specifically:
  ```bash
  cargo test -p unftp-sbe-anttp
  ```

#### Adding New Tests
Tests can be added as:
1. **Unit Tests**: Inside the `src/` files of each crate (e.g., `crates/unftp-sbe-anttp/src/lib.rs`).
2. **Integration Tests**: In a `tests/` directory within the crate (e.g., `crates/unftp-sbe-anttp/tests/`).
3. **Doc Tests**: Inside doc comments in `.rs` files.

#### Simple Test Example
To verify the storage backend can be initialized:
```rust
use unftp_sbe_anttp::Filesystem;

#[tokio::test]
async fn test_filesystem_initialization() {
    let temp_dir = std::env::temp_dir();
    let fs = Filesystem::new(&temp_dir);
    assert!(fs.is_ok(), "Filesystem should initialize successfully");
}
```

### Additional Development Information
- **Code Style**: Standard Rust conventions are followed.
- **Storage Backend**: The `unftp-sbe-anttp` crate implements the `StorageBackend` trait from `libunftp`.
- **Architecture Note**: The project is designed to bridge FTP commands to AntTP gRPC calls.
- **Capabilities**:

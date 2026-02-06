fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    let protoc_exists = Command::new("protoc").arg("--version").output().is_ok();

    if protoc_exists {
        tonic_build::compile_protos("../../proto/public_archive.proto")?;
    } else {
        println!("cargo:warning=protoc not found, disabling gRPC support");
        println!("cargo:rustc-cfg=grpc_disabled");
    }
    Ok(())
}

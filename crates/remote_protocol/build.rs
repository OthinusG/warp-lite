fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/remote_server.proto");
    prost_build::Config::new()
        .boxed(".remote_server.ManagedResponse.result.project_files")
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(&["proto/remote_server.proto"], &["proto/"])?;
    Ok(())
}

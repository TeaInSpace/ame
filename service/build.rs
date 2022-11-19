fn main() {
    // TODO: evaluate state of optionals in protobuf.
    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile(&["./task.proto"], &["."])
        .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
}

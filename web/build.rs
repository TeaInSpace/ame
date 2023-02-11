use std::io;

fn main() -> io::Result<()> {
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile(&["ame.proto"], &["../proto"])
}

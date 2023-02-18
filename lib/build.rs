use std::io;

fn main() -> io::Result<()> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .type_attribute(
            ".",
            "#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema )]",
        )
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile(&["ame.proto"], &["../proto"])
}

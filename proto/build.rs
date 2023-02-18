fn main() {
    // TODO: evaluate state of optionals in protobuf.
    tonic_build::configure()
        .type_attribute(
            ".",
            "#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]",
        )
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile(&["./ame.proto"], &["."])
        .unwrap_or_else(|e| panic!("Failed to compile protos {e:?}"));
}

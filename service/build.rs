fn main() {
    tonic_build::configure()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile(&["../proto/ame.proto"], &["../proto"])
        .unwrap_or_else(|e| panic!("Failed to compile protos {e:?}"));
}

use prost_build::Config;
use std::io;

fn main() -> io::Result<()> {
    let mut prost_cfg = Config::new();
    prost_cfg.btree_map(["resources", "ingressAnnotations"]);

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .type_attribute(
            ".",
            "#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema )]",
        )
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        .field_attribute(
            "ProjectCfg.models",
            "#[serde(skip_serializing_if = \"Vec::is_empty\", default)]",
        )
        .field_attribute(
            "ProjectCfg.dataSets",
            "#[serde(skip_serializing_if = \"Vec::is_empty\", default)]",
        )
        .field_attribute(
            "TaskCfg.dataSets",
            "#[serde(skip_serializing_if = \"Vec::is_empty\", default)]",
        )
        .field_attribute(
            "ProjectCfg.templates",
            "#[serde(skip_serializing_if = \"Vec::is_empty\", default)]",
        )
        .field_attribute(
            "ProjectCfg.tasks",
            "#[serde(default = \"Vec::<TaskCfg>::new\")]",
        )
        .field_attribute("paths", "#[serde(default = \"Vec::<String>::new\")]")
        .field_attribute(
            "TaskCfg.secrets",
            "#[serde(default = \"Vec::<Secret>::new\")]",
        )
        .field_attribute(
            "TaskCfg.taskRef",
            "#[serde(skip_serializing_if = \"Option::is_none\")]",
        )
        .field_attribute(
            "TaskCfg.fromTemplate",
            "#[serde(skip_serializing_if = \"Option::is_none\")]",
        )
        .field_attribute(
            "TaskCfg.artifactCfg",
            "#[serde(skip_serializing_if = \"Option::is_none\")]",
        )
        .field_attribute(
            "TaskCfg.triggers",
            "#[serde(skip_serializing_if = \"Option::is_none\")]",
        )
        .field_attribute("TaskCfg.env", "#[serde(default = \"Vec::<EnvVar>::new\")]")
        .field_attribute(
            "resources",
            "#[serde(default = \"std::collections::BTreeMap::<String, String>::new\")]",
        )
        .field_attribute(
            "ingressAnnotations",
            "#[serde(default = \"std::collections::BTreeMap::<String, String>::new\")]",
        )
        .field_attribute("Secret.variant", "#[serde(flatten)]")
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_with_config(prost_cfg, &["ame.proto"], &["./"])
}

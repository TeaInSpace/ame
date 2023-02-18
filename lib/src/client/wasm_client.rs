use crate::{grpc::ame_service_client::AmeServiceClient, AmeServiceClientCfg, Result};

use tonic_web_wasm_client::Client;

pub type AmeClient = AmeServiceClient<Client>;

pub fn build_ame_client(cfg: AmeServiceClientCfg) -> Result<AmeClient> {
    Ok(AmeServiceClient::new(Client::new(cfg.endpoint)))
}

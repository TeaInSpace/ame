#[cfg(feature = "native-client")]
pub mod native_client;

#[cfg(feature = "native-client")]
pub mod auth;

#[cfg(feature = "web-components")]
pub mod wasm_client;

#[cfg(feature = "web-components")]
mod common;
#[cfg(feature = "web-components")]
pub use common::*;

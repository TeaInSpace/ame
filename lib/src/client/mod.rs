#[cfg(all(feature = "native-client", not(features = "web-components")))]
mod native_client;
#[cfg(all(feature = "native-client", not(features = "web-components")))]
pub use native_client::*;

#[cfg(all(feature = "web-components", not(features = "native-client")))]
mod wasm_client;

#[cfg(all(feature = "web-components", not(features = "native-client")))]
pub use wasm_client::*;

#[cfg(any(feature = "web-components", feature = "native-client"))]
mod common;
#[cfg(any(feature = "web-components", feature = "native-client"))]
pub use common::*;

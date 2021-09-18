mod asset_server;
mod handle;
mod plugin;
mod storage;
pub mod util;

pub use asset_server::AssetServer;
use handle::WeakHandle;

use distill_core::type_uuid::TypeUuid;
use distill_core::TypeUuidDynamic;
use serde::Deserialize;

pub use distill_loader;
pub use plugin::{AssetPlugin, AssetStage};
pub use storage::Assets;

pub mod prelude {
    pub use crate::handle::{Handle, HandleUntyped, WeakHandle};
    #[cfg(feature = "asset-daemon")]
    pub use crate::plugin::AssetDaemonSettings;
    #[cfg(feature = "packfile")]
    pub use crate::plugin::PackfileSettings;
    pub use crate::plugin::{AddAsset, AssetPlugin, AssetServerSettings};
    pub use crate::{Asset, AssetEvent, AssetServer, Assets};

    pub use distill_core::type_uuid::{self, TypeUuid};
    #[cfg(feature = "serde-importers")]
    pub use distill_importer::SerdeImportable;

    // required for SerdeImportable
    #[doc(hidden)]
    #[cfg(feature = "serde-importers")]
    pub use distill_importer;
    #[doc(hidden)]
    #[cfg(feature = "serde-importers")]
    pub use distill_importer::typetag;

    pub use serde::{Deserialize, Serialize};
}

pub trait Asset: TypeUuid + AssetDynamic {}

pub trait AssetDynamic: TypeUuidDynamic + for<'a> Deserialize<'a> + Send + Sync + 'static {}

impl<T> Asset for T where T: TypeUuid + AssetDynamic + TypeUuidDynamic {}

impl<T> AssetDynamic for T where T: Send + Sync + 'static + TypeUuidDynamic + for<'a> Deserialize<'a>
{}

/// Events that happen on assets of type `T`
pub enum AssetEvent<A: Asset> {
    Modified { handle: WeakHandle<A> },
    Removed { handle: WeakHandle<A> },
}
impl<A: Asset> AssetEvent<A> {
    pub fn handle(&self) -> &WeakHandle<A> {
        match self {
            AssetEvent::Modified { handle } => handle,
            AssetEvent::Removed { handle } => handle,
        }
    }
}

impl<A: Asset> std::fmt::Debug for AssetEvent<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let variant = match self {
            AssetEvent::Modified { .. } => "Modified",
            AssetEvent::Removed { .. } => "Removed",
        };
        let name = format!("AssetEvent<{}>::{}", std::any::type_name::<A>(), variant);
        f.debug_struct(&name)
            .field("handle", self.handle())
            .finish()
    }
}

mod asset_server;
mod handle;
pub mod importer;
mod plugin;
mod storage;
pub mod util;

pub use asset_server::AssetServer;
use bevy_reflect::TypeUuid;

use distill_core::TypeUuidDynamic;

pub use distill_importer;
pub use handle::{Handle, HandleUntyped, WeakHandle};
pub use plugin::{AddAsset, AssetPlugin, AssetStage};
pub use storage::Assets;

pub mod prelude {
    pub use crate::handle::{Handle, HandleUntyped, WeakHandle};
    #[cfg(feature = "asset-daemon")]
    pub use crate::plugin::AssetDaemonSettings;
    #[cfg(feature = "packfile")]
    pub use crate::plugin::PackfileSettings;
    pub use crate::plugin::{AddAsset, AssetPlugin, AssetServerSettings};
    pub use crate::{Asset, AssetEvent, AssetServer, Assets};

    pub use bevy_reflect::TypeUuid;

    pub use serde::{Deserialize, Serialize};
}

pub trait Asset: TypeUuid + AssetDynamic {}

pub trait AssetDynamic: TypeUuidDynamic + Send + Sync + 'static {}

impl<T> Asset for T where T: TypeUuid + AssetDynamic + TypeUuidDynamic {}

impl<T> AssetDynamic for T where T: Send + Sync + 'static + TypeUuidDynamic {}

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

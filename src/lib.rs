mod asset_server;
mod handle;
mod plugin;
mod storage;

pub use asset_server::AssetServer;

use distill::core::type_uuid::TypeUuid;
use distill::core::TypeUuidDynamic;

use serde::Deserialize;

pub use distill::loader;
pub use plugin::{AssetPlugin, AssetStage};
pub use storage::Assets;

pub mod prelude {
    pub use crate::handle::{Handle, HandleUntyped, WeakHandle};
    pub use crate::plugin::{AddAsset, AssetPlugin, AssetServerSettings};
    pub use crate::{Asset, AssetServer, Assets};

    pub use distill::core::type_uuid::{self, TypeUuid};
    pub use distill::importer::SerdeImportable;

    // required for SerdeImportable
    #[doc(hidden)]
    pub use distill::importer as distill_importer;
    #[doc(hidden)]
    pub use distill::importer::typetag;

    pub use serde::{Deserialize, Serialize};
}

pub trait Asset: TypeUuid + AssetDynamic {}

pub trait AssetDynamic: TypeUuidDynamic + for<'a> Deserialize<'a> + Send + Sync + 'static {}

impl<T> Asset for T where T: TypeUuid + AssetDynamic + TypeUuidDynamic {}

impl<T> AssetDynamic for T where T: Send + Sync + 'static + TypeUuidDynamic + for<'a> Deserialize<'a>
{}

use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};

use distill::loader::crossbeam_channel::Sender;
use distill::loader::handle::{AssetHandle, RefOp, TypedAssetStorage};
use distill::loader::storage::{
    AssetLoadOp, AssetStorage, IndirectionTable, LoadHandle, LoaderInfoProvider,
};
use distill::loader::AssetTypeId;

use super::Asset;

struct AssetState<A> {
    version: u32,
    asset: A,
}
pub struct Assets<A: Asset> {
    refop_sender: Arc<Sender<RefOp>>,
    assets: HashMap<LoadHandle, AssetState<A>>,
    uncommitted: HashMap<LoadHandle, AssetState<A>>,
    indirection_table: IndirectionTable,
}
impl<A: Asset> Assets<A> {
    pub fn new(sender: Arc<Sender<RefOp>>, indirection_table: IndirectionTable) -> Self {
        Self {
            refop_sender: sender,
            assets: HashMap::new(),
            uncommitted: HashMap::new(),
            indirection_table,
        }
    }

    pub fn get<T: AssetHandle>(&self, handle: &T) -> Option<&A> {
        let handle = self.resolve_handle(handle.load_handle())?;
        self.assets.get(&handle).map(|a| &a.asset)
    }

    pub fn get_version<T: AssetHandle>(&self, handle: &T) -> Option<u32> {
        let handle = self.resolve_handle(handle.load_handle())?;
        self.assets.get(&handle).map(|a| a.version)
    }

    pub fn get_asset_with_version<T: AssetHandle>(&self, handle: &T) -> Option<(&A, u32)> {
        let handle = self.resolve_handle(handle.load_handle())?;
        self.assets.get(&handle).map(|a| (&a.asset, a.version))
    }

    fn resolve_handle(&self, load_handle: LoadHandle) -> Option<LoadHandle> {
        if load_handle.is_indirect() {
            self.indirection_table.resolve(load_handle)
        } else {
            Some(load_handle)
        }
    }
}

pub(crate) struct SharedAssets<'a, A: Asset>(pub Mutex<&'a mut Assets<A>>);

impl<'a, A: Asset> AssetStorage for SharedAssets<'a, A> {
    fn update_asset(
        &self,
        loader_info: &dyn LoaderInfoProvider,
        _asset_type: &AssetTypeId,
        data: Vec<u8>,
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        version: u32,
    ) -> Result<(), Box<dyn Error + Send + 'static>> {
        let mut this = self.0.lock().unwrap();

        // To enable automatic serde of Handle, we need to set up a SerdeContext with a RefOp sender
        let asset = futures_executor::block_on(distill::loader::handle::SerdeContext::with(
            loader_info,
            (*this.refop_sender).clone(),
            async { bincode::deserialize::<A>(&data) },
        ));
        let asset = match asset {
            Ok(asset) => asset,
            Err(e) => {
                load_op.error(e);
                return Ok(());
            }
        };

        this.uncommitted
            .insert(load_handle, AssetState { version, asset });
        bevy_log::info!("{} bytes loaded for {:?}", data.len(), load_handle);
        // The loading process could be async, in which case you can delay
        // calling `load_op.complete` as it should only be done when the asset is usable.
        load_op.complete();
        Ok(())
    }

    fn commit_asset_version(
        &self,
        _asset_type: &AssetTypeId,
        load_handle: LoadHandle,
        _version: u32,
    ) {
        let mut this = self.0.lock().unwrap();
        // The commit step is done after an asset load has completed.
        // It exists to avoid frames where an asset that was loaded is unloaded, which
        // could happen when hot reloading. To support this case, you must support having multiple
        // versions of an asset loaded at the same time.
        let asset_state = this
            .uncommitted
            .remove(&load_handle)
            .expect("asset not present when committing");
        this.assets.insert(load_handle, asset_state);
        bevy_log::info!("Commit {:?}", load_handle);
    }

    fn free(&self, _asset_type: &AssetTypeId, load_handle: LoadHandle, version: u32) {
        let mut this = self.0.lock().unwrap();

        if let Some(asset) = this.uncommitted.get(&load_handle) {
            if asset.version == version {
                this.uncommitted.remove(&load_handle);
            }
        }
        if let Some(asset) = this.assets.get(&load_handle) {
            if asset.version == version {
                this.assets.remove(&load_handle);
            }
        }
        bevy_log::info!("Free {:?}", load_handle);
    }
}

impl<A: Asset> TypedAssetStorage<A> for Assets<A> {
    fn get<T: AssetHandle>(&self, handle: &T) -> Option<&A> {
        self.get(handle)
    }

    fn get_version<T: AssetHandle>(&self, handle: &T) -> Option<u32> {
        self.get_version(handle)
    }

    fn get_asset_with_version<T: AssetHandle>(&self, handle: &T) -> Option<(&A, u32)> {
        self.get_asset_with_version(handle)
    }
}

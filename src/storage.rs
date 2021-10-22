use std::error::Error;
use std::sync::Arc;

use bevy_app::Events;
use bevy_ecs::prelude::*;

use bevy_utils::HashMap;
use distill_loader::crossbeam_channel::Sender;
use distill_loader::handle::{AssetHandle, RefOp, TypedAssetStorage};
use distill_loader::storage::{
    AssetLoadOp, AssetStorage, HandleAllocator, IndirectionTable, LoadHandle, LoaderInfoProvider,
};
use distill_loader::AssetTypeId;
use serde::de::DeserializeSeed;

use crate::prelude::{Handle, WeakHandle};
use crate::AssetEvent;

use super::Asset;

struct AssetState<A> {
    version: u32,
    asset: A,
}
pub struct Assets<A: Asset> {
    pub(crate) refop_sender: Arc<Sender<RefOp>>,
    handle_allocator: Arc<dyn HandleAllocator>,
    assets: HashMap<LoadHandle, AssetState<A>>,
    uncommitted: HashMap<LoadHandle, AssetState<A>>,
    indirection_table: IndirectionTable,
    events: Events<AssetEvent<A>>,
}
impl<A: Asset> Assets<A> {
    pub fn new(
        sender: Arc<Sender<RefOp>>,
        handle_allocator: Arc<dyn HandleAllocator>,
        indirection_table: IndirectionTable,
    ) -> Self {
        Self {
            refop_sender: sender,
            handle_allocator,
            assets: HashMap::default(),
            uncommitted: HashMap::default(),
            indirection_table,
            events: Events::default(),
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

    pub fn get_mut<T: AssetHandle>(&mut self, handle: &T) -> Option<&mut A> {
        let handle = self.resolve_handle(handle.load_handle())?;

        let (asset, version) = self
            .assets
            .get_mut(&handle)
            .map(|a| (&mut a.asset, a.version))?;

        self.events.send(AssetEvent::Modified {
            handle: WeakHandle::new(handle),
            version,
        });

        Some(asset)
    }

    pub fn add(&mut self, asset: A) -> Handle<A> {
        let load_handle = self.handle_allocator.alloc();
        self.assets
            .insert(load_handle, AssetState { version: 0, asset });

        self.events.send(AssetEvent::Modified {
            handle: WeakHandle::new(load_handle),
            version: 0,
        });

        Handle::new((*self.refop_sender).clone(), load_handle)
    }

    pub fn remove<T: AssetHandle>(&mut self, handle: &T) -> Option<A> {
        let handle = self.resolve_handle(handle.load_handle())?;
        let (asset, version) = self.assets.remove(&handle).map(|a| (a.asset, a.version))?;
        self.events.send(AssetEvent::Removed {
            handle: WeakHandle::new(handle),
            version,
        });
        Some(asset)
    }

    pub fn iter(&self) -> impl Iterator<Item = (WeakHandle<A>, &A)> {
        self.assets
            .iter()
            .map(|(&k, v)| (WeakHandle::new(k), &v.asset))
    }

    pub fn ids(&self) -> impl Iterator<Item = WeakHandle<A>> + '_ {
        self.assets.keys().map(|&handle| WeakHandle::new(handle))
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    pub fn resolve<T: AssetHandle>(&self, handle: &T) -> Option<WeakHandle<A>> {
        let load_handle = self.resolve_handle(handle.load_handle())?;
        Some(WeakHandle::new(load_handle))
    }

    fn resolve_handle(&self, load_handle: LoadHandle) -> Option<LoadHandle> {
        if load_handle.is_indirect() {
            self.indirection_table.resolve(load_handle)
        } else {
            Some(load_handle)
        }
    }

    pub(crate) fn asset_event_system(
        mut events: EventWriter<AssetEvent<A>>,
        mut assets: ResMut<Assets<A>>,
    ) {
        // Check if the events are empty before calling `drain`.
        // As `drain` triggers change detection.
        if !assets.events.is_empty() {
            events.send_batch(assets.events.drain())
        }
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

struct AssetStorageData<'a, A: Asset, D> {
    seed: D,
    assets: &'a mut Assets<A>,
}

impl<'a, A, D> AssetStorage for AssetStorageData<'a, A, D>
where
    A: Asset,
    D: FromWorld + Clone + for<'de> DeserializeSeed<'de, Value = A>,
{
    fn update_asset(
        &mut self,
        loader_info: &dyn LoaderInfoProvider,
        asset_type: &AssetTypeId,
        data: Vec<u8>,
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        version: u32,
    ) -> Result<(), Box<dyn Error + Send + 'static>> {
        debug_assert_eq!(*A::TYPE_UUID.as_bytes(), asset_type.0);

        use bincode::Options;
        let bincode = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes();

        let seed = self.seed.clone();

        // To enable automatic serde of Handle, we need to set up a SerdeContext with a RefOp sender
        let asset = futures_executor::block_on(distill_loader::handle::SerdeContext::with(
            loader_info,
            (*self.assets.refop_sender).clone(),
            async { bincode.deserialize_seed::<D>(seed, &data) },
        ));
        let asset = match asset {
            Ok(asset) => asset,
            Err(e) => {
                load_op.error(e);
                return Ok(());
            }
        };

        self.assets
            .uncommitted
            .insert(load_handle, AssetState { version, asset });
        load_op.complete();

        bevy_log::trace!(
            "updating asset {:?}@{} (type {}, {} bytes loaded)",
            load_handle,
            version,
            std::any::type_name::<A>(),
            data.len()
        );

        Ok(())
    }

    fn commit_asset_version(
        &mut self,
        asset_type: &AssetTypeId,
        load_handle: LoadHandle,
        version: u32,
    ) {
        debug_assert_eq!(*A::TYPE_UUID.as_bytes(), asset_type.0);

        bevy_log::trace!(
            "commiting asset {:?}@{} (type {})",
            load_handle,
            version,
            std::any::type_name::<A>(),
        );
        let handle = WeakHandle::new(load_handle);
        self.assets
            .events
            .send(AssetEvent::Modified { handle, version });

        // The commit step is done after an asset load has completed.
        // It exists to avoid frames where an asset that was loaded is unloaded, which
        // could happen when hot reloading. To support this case, you must support having multiple
        // versions of an asset loaded at the same time.
        let asset_state = self
            .assets
            .uncommitted
            .remove(&load_handle)
            .expect("asset not present when committing");
        self.assets.assets.insert(load_handle, asset_state);
    }

    fn free(&mut self, asset_type: &AssetTypeId, load_handle: LoadHandle, version: u32) {
        debug_assert_eq!(*A::TYPE_UUID.as_bytes(), asset_type.0);

        if let Some(asset) = self.assets.uncommitted.get(&load_handle) {
            if asset.version == version {
                self.assets.uncommitted.remove(&load_handle);
            }
        }
        if let Some(asset) = self.assets.assets.get(&load_handle) {
            if asset.version == version {
                self.assets.assets.remove(&load_handle);
            }
        }

        let handle = WeakHandle::new(load_handle);
        self.assets
            .events
            .send(AssetEvent::Removed { handle, version });

        bevy_log::trace!("free {:?}@{}", load_handle, version);
    }
}

// TODO: this is very ugly, maybe there is another way?
type AssetStorageProvider =
    Box<dyn (Fn(&mut World, &mut dyn FnMut(&mut dyn AssetStorage))) + Send + Sync + 'static>;

#[derive(Default)]
pub struct AssetResources(HashMap<AssetTypeId, AssetStorageProvider>);
impl AssetResources {
    pub fn add<A, D>(&mut self)
    where
        A: Asset,
        D: FromWorld + for<'de> DeserializeSeed<'de, Value = A> + Clone,
    {
        let asset_type = AssetTypeId(*A::TYPE_UUID.as_bytes());
        self.0.insert(
            asset_type,
            Box::new(|world, callback| {
                let seed = D::from_world(world);
                let assets = world.get_resource_mut::<Assets<A>>().unwrap().into_inner();
                let mut storage = AssetStorageData { seed, assets };

                callback(&mut storage);
            }),
        );
    }
}

pub(crate) struct WorldAssetStorage<'w> {
    pub world: &'w mut World,
    pub asset_resources: &'w AssetResources,
}
impl<'w> WorldAssetStorage<'w> {
    fn with<R>(
        &mut self,
        asset_type: &AssetTypeId,
        f: impl FnOnce(&mut dyn AssetStorage) -> R,
    ) -> R {
        let func = self
            .asset_resources
            .0
            .get(asset_type)
            .unwrap_or_else(|| panic!("asset not registered: {}", asset_type));

        let mut f = Some(f);
        let mut result = None;

        func(&mut self.world, &mut |storage| {
            let f = f.take().unwrap();
            result = Some(f(storage));
        });

        result.unwrap()
    }
}
impl AssetStorage for WorldAssetStorage<'_> {
    fn update_asset(
        &mut self,
        loader_info: &dyn LoaderInfoProvider,
        asset_type_id: &AssetTypeId,
        data: Vec<u8>,
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        version: u32,
    ) -> Result<(), Box<dyn Error + Send + 'static>> {
        self.with(asset_type_id, |storage| {
            storage.update_asset(
                loader_info,
                asset_type_id,
                data,
                load_handle,
                load_op,
                version,
            )
        })
    }

    fn commit_asset_version(
        &mut self,
        asset_type: &AssetTypeId,
        load_handle: LoadHandle,
        version: u32,
    ) {
        self.with(asset_type, |storage| {
            storage.commit_asset_version(asset_type, load_handle, version)
        })
    }

    fn free(&mut self, asset_type_id: &AssetTypeId, load_handle: LoadHandle, version: u32) {
        self.with(asset_type_id, |storage| {
            storage.free(asset_type_id, load_handle, version)
        })
    }
}

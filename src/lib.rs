mod asset_server;
mod storage;

pub use asset_server::AssetServer;
pub use storage::Assets;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use distill::core::type_uuid::TypeUuid;
use distill::core::TypeUuidDynamic;
use distill::daemon::AssetDaemon;
use distill::loader::crossbeam_channel::{unbounded, Receiver, Sender};
use distill::loader::handle::{self, RefOp};
use distill::loader::storage::DefaultIndirectionResolver;
use distill::loader::{Loader, RpcIO};
use serde::Deserialize;

pub use distill::loader;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

use self::storage::SharedAssets;

pub mod prelude {
    pub use crate::loader::handle::Handle;
    pub use crate::{AddAsset, Asset, AssetServer, Assets};
}

pub struct AssetPlugin;

pub struct RefopReceiver(pub Receiver<RefOp>);
pub struct RefopSender(pub Arc<Sender<RefOp>>);

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        std::thread::spawn(|| {
            AssetDaemon::default()
                .with_db_path(".assets_db")
                .with_address("127.0.0.1:9999".parse().unwrap())
                .with_asset_dirs(vec![PathBuf::from("assets")])
                .run();
        });

        let (refop_sender, refop_receiver) = unbounded();
        let refop_sender = Arc::new(refop_sender);

        let loader = Loader::new(Box::new(RpcIO::default()));
        let asset_server = AssetServer::new(loader, Arc::clone(&refop_sender));

        app.insert_resource(asset_server)
            .insert_resource(RefopReceiver(refop_receiver))
            .insert_resource(RefopSender(refop_sender))
            .add_stage_before(
                CoreStage::PreUpdate,
                AssetStage::LoadAssets,
                SystemStage::parallel()
                    .with_system(process_asset_events.label(AssetSystem::ProcessAssetEvents)),
            );
    }
}

#[derive(StageLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub enum AssetStage {
    LoadAssets,
}

#[derive(SystemLabel, Debug, Clone, Hash, PartialEq, Eq)]
enum AssetSystem {
    ProcessAssetEvents,
}

pub trait Asset: TypeUuid + AssetDynamic {}

pub trait AssetDynamic: TypeUuidDynamic + for<'a> Deserialize<'a> + Send + Sync + 'static {}

impl<T> Asset for T where T: TypeUuid + AssetDynamic + TypeUuidDynamic {}

impl<T> AssetDynamic for T where T: Send + Sync + 'static + TypeUuidDynamic + for<'a> Deserialize<'a>
{}

fn process_asset_events(asset_server: Res<AssetServer>, refop_receiver: Res<RefopReceiver>) {
    handle::process_ref_ops(asset_server.loader(), &refop_receiver.0);
}

fn process_asset_events_per_asset<A: Asset>(
    mut asset_server: ResMut<AssetServer>,
    mut asset_storage: ResMut<Assets<A>>,
) {
    let shared_assets = SharedAssets(Mutex::new(&mut *asset_storage));
    asset_server
        .loader_mut()
        .process(&shared_assets, &DefaultIndirectionResolver)
        .unwrap();
}

pub trait AddAsset {
    fn add_asset<T: Asset>(&mut self) -> &mut Self;

    /*fn init_asset_loader<T>(&mut self) -> &mut Self
    where
        T: AssetLoader + FromWorld;
    fn add_asset_loader<T>(&mut self, loader: T) -> &mut Self
    where
        T: AssetLoader;*/
}

impl AddAsset for App {
    fn add_asset<A: Asset>(&mut self) -> &mut Self {
        let assets = {
            let refop_sender = self.world.get_resource::<RefopSender>().unwrap();
            let asset_server = self.world.get_resource::<AssetServer>().unwrap();
            Assets::<A>::new(
                Arc::clone(&refop_sender.0),
                asset_server.loader().indirection_table(),
            )
        };
        self.world.insert_resource(assets);

        self.add_system_to_stage(
            AssetStage::LoadAssets,
            process_asset_events_per_asset::<A>.after(AssetSystem::ProcessAssetEvents),
        );

        self
    }
}

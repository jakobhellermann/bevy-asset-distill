mod asset_server;
mod handle;
mod storage;

pub use asset_server::AssetServer;
use distill::loader::io::LoaderIO;
use prelude::{Handle, HandleUntyped};
pub use storage::Assets;

use std::fs::File;
use std::hash::Hash;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use distill::core::type_uuid::TypeUuid;
use distill::core::TypeUuidDynamic;
use distill::daemon::AssetDaemon;
use distill::loader::crossbeam_channel::{unbounded, Receiver, Sender};
use distill::loader::handle::RefOp;
use distill::loader::storage::DefaultIndirectionResolver;
use distill::loader::{Loader, PackfileReader, RpcIO};
use serde::Deserialize;

pub use distill::loader;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

use self::storage::SharedAssets;

pub mod prelude {
    pub use crate::handle::{Handle, HandleUntyped};
    pub use crate::{AddAsset, Asset, AssetServer, AssetServerSettings, Assets};
}

pub struct AssetPlugin;

pub struct RefopReceiver(pub Receiver<RefOp>);
pub struct RefopSender(pub Arc<Sender<RefOp>>);

pub enum AssetServerSettings {
    Daemon {
        db_path: PathBuf,
        address: SocketAddr,
        clear_db_on_start: bool,
    },
    Packfile {
        path: PathBuf,
    },
    PackfileStatic(&'static [u8]),
}
impl AssetServerSettings {
    fn daemon(&self) -> Option<AssetDaemon> {
        match *self {
            AssetServerSettings::Daemon {
                ref db_path,
                ref address,
                clear_db_on_start,
            } => {
                let db_path = db_path.clone();
                let address = address.clone();
                let mut asset_daemon = AssetDaemon::default()
                    .with_db_path(db_path)
                    .with_address(address)
                    .with_asset_dirs(vec![PathBuf::from("assets")]);
                if clear_db_on_start {
                    asset_daemon = asset_daemon.with_clear_db_on_start();
                }
                Some(asset_daemon)
            }
            _ => None,
        }
    }

    fn loader_io(&self) -> Result<Box<dyn LoaderIO>, Box<dyn std::error::Error>> {
        Ok(match self {
            AssetServerSettings::Daemon { address, .. } => {
                Box::new(RpcIO::new(address.to_string()).unwrap())
            }
            AssetServerSettings::Packfile { path } => {
                let file = File::open(path)?;
                Box::new(PackfileReader::new_from_file(file)?)
            }
            AssetServerSettings::PackfileStatic(buffer) => {
                Box::new(PackfileReader::new_from_buffer(buffer)?)
            }
        })
    }
}

impl Default for AssetServerSettings {
    fn default() -> Self {
        AssetServerSettings::Daemon {
            db_path: PathBuf::from(".assets_db"),
            address: ([127, 0, 0, 1], 9999).into(),
            clear_db_on_start: false,
        }
    }
}

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        let asset_server_settings = app
            .world
            .get_resource_or_insert_with(AssetServerSettings::default);

        if let Some(daemon) = asset_server_settings.daemon() {
            std::thread::spawn(|| daemon.run());
        }

        let (refop_sender, refop_receiver) = unbounded();
        let refop_sender = Arc::new(refop_sender);

        let loader_io = asset_server_settings
            .loader_io()
            .expect("failed to create asset loader IO");
        let loader = Loader::new(loader_io);
        let asset_server = AssetServer::new(loader, Arc::clone(&refop_sender));

        app.register_type::<HandleUntyped>()
            .insert_resource(asset_server)
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
    loader::handle::process_ref_ops(asset_server.loader(), &refop_receiver.0);
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

        self.register_type::<Handle<A>>().add_system_to_stage(
            AssetStage::LoadAssets,
            process_asset_events_per_asset::<A>.after(AssetSystem::ProcessAssetEvents),
        );

        self
    }
}

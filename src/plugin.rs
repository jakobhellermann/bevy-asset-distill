use std::fs::File;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::prelude::*;
use crate::storage::SharedAssets;
use crate::AssetEvent;
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

use distill::daemon::AssetDaemon;
use distill::importer::BoxedImporter;
use distill::loader::crossbeam_channel::{unbounded, Receiver, Sender};
use distill::loader::handle::RefOp;
use distill::loader::io::LoaderIO;
use distill::loader::storage::{
    AtomicHandleAllocator, DefaultIndirectionResolver, HandleAllocator,
};
use distill::loader::{self, Loader, PackfileReader, RpcIO};

#[derive(StageLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub enum AssetStage {
    LoadAssets,
    AssetEvents,
}

#[derive(SystemLabel, Debug, Clone, Hash, PartialEq, Eq)]
enum AssetSystem {
    ProcessAssetEvents,
}

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
impl Default for AssetServerSettings {
    fn default() -> Self {
        AssetServerSettings::Daemon {
            db_path: PathBuf::from(".assets_db"),
            address: ([127, 0, 0, 1], 9999).into(),
            clear_db_on_start: false,
        }
    }
}
impl AssetServerSettings {
    fn daemon(&self, asset_loaders: AssetLoaders) -> Option<AssetDaemon> {
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
                    .with_importers_boxed(asset_loaders.0)
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

pub struct AssetPlugin;

struct RefopReceiver(Receiver<RefOp>);
struct RefopSender(Arc<Sender<RefOp>>);
struct AssetHandleAllocator(Arc<dyn HandleAllocator>);

#[derive(Default)]
struct AssetLoaders(Vec<(&'static str, Box<dyn BoxedImporter + 'static>)>);

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        let world = &mut app.world;
        let asset_loaders = world.remove_resource::<AssetLoaders>().unwrap_or_default();
        let asset_server_settings = world.get_resource_or_insert_with(AssetServerSettings::default);

        if let Some(daemon) = asset_server_settings.daemon(asset_loaders) {
            std::thread::spawn(|| daemon.run());
        }

        let (refop_sender, refop_receiver) = unbounded();
        let refop_sender = Arc::new(refop_sender);

        let loader_io = asset_server_settings
            .loader_io()
            .expect("failed to create asset loader IO");

        let handle_allocator =
            Arc::new(AtomicHandleAllocator::default()) as Arc<dyn HandleAllocator>;
        let loader = Loader::new_with_handle_allocator(loader_io, Arc::clone(&handle_allocator));
        let asset_server = AssetServer::new(loader, Arc::clone(&refop_sender));

        app.register_type::<HandleUntyped>()
            .insert_resource(asset_server)
            .insert_resource(RefopReceiver(refop_receiver))
            .insert_resource(RefopSender(refop_sender))
            .insert_resource(AssetHandleAllocator(handle_allocator))
            .add_stage_before(
                CoreStage::PreUpdate,
                AssetStage::LoadAssets,
                SystemStage::parallel()
                    .with_system(process_asset_events.label(AssetSystem::ProcessAssetEvents)),
            )
            .add_stage_after(
                CoreStage::PostUpdate,
                AssetStage::AssetEvents,
                SystemStage::parallel(),
            );
    }
}

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
    fn init_asset_loader<T: BoxedImporter + FromWorld>(
        &mut self,
        extension: &'static str,
    ) -> &mut Self;
    fn add_asset_loader<T: BoxedImporter>(
        &mut self,
        extension: &'static str,
        loader: T,
    ) -> &mut Self;
}

impl AddAsset for App {
    fn add_asset<A: Asset>(&mut self) -> &mut Self {
        let assets = {
            let refop_sender = self.world.get_resource::<RefopSender>().unwrap();
            let asset_server = self.world.get_resource::<AssetServer>().unwrap();
            let handle_allocator = self.world.get_resource::<AssetHandleAllocator>().unwrap();
            Assets::<A>::new(
                Arc::clone(&refop_sender.0),
                Arc::clone(&handle_allocator.0),
                asset_server.loader().indirection_table(),
            )
        };
        self.world.insert_resource(assets);

        self.register_type::<Handle<A>>()
            .add_event::<AssetEvent<A>>()
            .add_system_to_stage(
                AssetStage::LoadAssets,
                process_asset_events_per_asset::<A>.after(AssetSystem::ProcessAssetEvents),
            )
            .add_system_to_stage(AssetStage::AssetEvents, Assets::<A>::asset_event_system);

        self
    }

    fn init_asset_loader<T: BoxedImporter + FromWorld>(
        &mut self,
        extension: &'static str,
    ) -> &mut Self {
        let loader = T::from_world(&mut self.world);
        Self::add_asset_loader(self, extension, loader)
    }

    fn add_asset_loader<T: BoxedImporter>(
        &mut self,
        extension: &'static str,
        loader: T,
    ) -> &mut Self {
        self.world
            .get_resource_or_insert_with(AssetLoaders::default)
            .0
            .push((extension, Box::new(loader)));
        self
    }
}

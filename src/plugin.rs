use std::sync::Arc;

use crate::prelude::*;
use crate::storage::{AssetResources, WorldAssetStorage};
use crate::AssetEvent;
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

use distill_importer::BoxedImporter;
use distill_loader::crossbeam_channel::{unbounded, Receiver, Sender};
use distill_loader::handle::RefOp;
use distill_loader::io::LoaderIO;
use distill_loader::storage::{AtomicHandleAllocator, DefaultIndirectionResolver, HandleAllocator};
use distill_loader::{self, Loader};

#[derive(StageLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub enum AssetStage {
    LoadAssets,
}

#[derive(SystemLabel, Debug, Clone, Hash, PartialEq, Eq)]
enum AssetSystem {
    ProcessAssetEvents,
}

#[derive(Debug, Clone)]
pub enum AssetServerSettings {
    #[cfg(feature = "asset-daemon")]
    Daemon(AssetDaemonSettings),
    #[cfg(feature = "rpc-io")]
    DaemonWebsocket(AssetDaemonWebsocketSettings),
    #[cfg(feature = "packfile")]
    Packfile(PackfileSettings),
}
impl AssetServerSettings {
    fn default_fallback() -> Option<Self> {
        #[cfg(feature = "asset-daemon")]
        return Some(AssetServerSettings::Daemon(AssetDaemonSettings::default()));
        #[cfg(all(not(feature = "asset-daemon"), feature = "rpc-io"))]
        #[cfg(all(
            not(feature = "asset-daemon"),
            feature = "rpc-io",
            target_arch = "wasm32"
        ))]
        return Some(AssetServerSettings::DaemonWebsocket(
            AssetDaemonWebsocketSettings::default(),
        ));
        #[cfg(not(any(
            feature = "asset-daemon",
            all(
                not(feature = "asset-daemon"),
                feature = "rpc-io",
                target_arch = "wasm32"
            )
        )))]
        return None;
    }
}

#[cfg(feature = "packfile")]
#[derive(Debug, Clone)]
pub enum PackfileSettings {
    #[cfg(not(target_family = "wasm"))]
    Path(std::path::PathBuf),
    Static(&'static [u8]),
}

#[cfg(feature = "asset-daemon")]
#[derive(Debug, Clone)]
pub struct AssetDaemonSettings {
    asset_dirs: Vec<std::path::PathBuf>,
    db_path: std::path::PathBuf,
    address: std::net::SocketAddr,
    clear_db_on_start: bool,
}

#[cfg(feature = "asset-daemon")]
impl Default for AssetDaemonSettings {
    fn default() -> Self {
        AssetDaemonSettings {
            asset_dirs: vec![std::path::PathBuf::from("assets")],
            db_path: std::path::PathBuf::from(".assets_db"),
            address: ([127, 0, 0, 1], 9999).into(),
            clear_db_on_start: false,
        }
    }
}

#[cfg(feature = "rpc-io")]
#[derive(Debug, Clone)]
pub struct AssetDaemonWebsocketSettings {
    address: std::net::SocketAddr,
}
#[cfg(feature = "rpc-io")]
impl Default for AssetDaemonWebsocketSettings {
    fn default() -> Self {
        AssetDaemonWebsocketSettings {
            address: ([127, 0, 0, 1], 9998).into(),
        }
    }
}

impl AssetServerSettings {
    #[cfg(feature = "asset-daemon")]
    fn daemon(&self, asset_loaders: AssetLoaders) -> Option<distill_daemon::AssetDaemon> {
        match *self {
            AssetServerSettings::Daemon(AssetDaemonSettings {
                ref asset_dirs,
                ref db_path,
                ref address,
                clear_db_on_start,
            }) => {
                let db_path = db_path.clone();
                let address = address.clone();
                let mut asset_daemon = distill_daemon::AssetDaemon::default()
                    .with_db_path(db_path)
                    .with_address(address)
                    .with_importers_boxed(asset_loaders.0)
                    .with_asset_dirs(asset_dirs.clone());
                if clear_db_on_start {
                    asset_daemon = asset_daemon.with_clear_db_on_start();
                }
                Some(asset_daemon)
            }
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }

    fn loader_io(&self) -> Result<Box<dyn LoaderIO>, Box<dyn std::error::Error>> {
        match *self {
            #[cfg(feature = "asset-daemon")]
            AssetServerSettings::Daemon(ref settings) => Ok(Box::new(
                distill_loader::RpcIO::new(distill_loader::rpc_io::RpcConnectionType::TCP(
                    settings.address.to_string(),
                ))
                .unwrap(),
            )),
            #[cfg(feature = "rpc-io")]
            AssetServerSettings::DaemonWebsocket(ref settings) => Ok(Box::new(
                distill_loader::RpcIO::new(distill_loader::rpc_io::RpcConnectionType::Websocket(
                    settings.address.to_string(),
                ))
                .unwrap(),
            )),
            #[cfg(feature = "packfile")]
            #[cfg(not(target_family = "wasm"))]
            AssetServerSettings::Packfile(PackfileSettings::Path(ref path)) => {
                let file = std::fs::File::open(path)?;
                Ok(Box::new(distill_loader::PackfileReader::new_from_file(
                    file,
                )?))
            }
            #[cfg(feature = "packfile")]
            AssetServerSettings::Packfile(PackfileSettings::Static(bytes)) => Ok(Box::new(
                distill_loader::PackfileReader::new_from_buffer(bytes)?,
            )),
        }
    }
}

pub struct AssetPlugin;

struct RefopReceiver(Receiver<RefOp>);
struct RefopSender(Arc<Sender<RefOp>>);
struct AssetHandleAllocator(Arc<dyn HandleAllocator>);

#[derive(Default)]
struct AssetLoaders(Vec<(&'static [&'static str], Box<dyn BoxedImporter + 'static>)>);

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        let world = &mut app.world;
        #[allow(unused_variables)]
        let asset_loaders = world.remove_resource::<AssetLoaders>().unwrap_or_default();

        let asset_server_settings = world.get_resource_or_insert_with(|| {
            AssetServerSettings::default_fallback()
                .unwrap_or_else(|| panic!("missing `AssetServerSettings` resource. Either insert it or enable the `asset-daemon` feature or enable `rpc-io` and start the daemon yourself"))
        });

        #[cfg(feature = "asset-daemon")]
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
            .init_resource::<AssetResources>()
            .insert_resource(asset_server)
            .insert_resource(RefopReceiver(refop_receiver))
            .insert_resource(RefopSender(refop_sender))
            .insert_resource(AssetHandleAllocator(handle_allocator))
            .add_stage_before(
                CoreStage::PreUpdate,
                AssetStage::LoadAssets,
                SystemStage::parallel().with_system(
                    process_asset_events
                        .exclusive_system()
                        .at_start()
                        .label(AssetSystem::ProcessAssetEvents),
                ),
            );
    }
}

fn process_asset_events(world: &mut World) {
    world.resource_scope(|world, mut asset_server: Mut<AssetServer>| {
        let refop_receiver = world.get_resource::<RefopReceiver>().unwrap();
        distill_loader::handle::process_ref_ops(asset_server.loader(), &refop_receiver.0);

        world.resource_scope(|world, asset_resources: Mut<AssetResources>| {
            let mut asset_storage = WorldAssetStorage(world, &*asset_resources);

            asset_server
                .loader_mut()
                .process(&mut asset_storage, &DefaultIndirectionResolver)
                .unwrap();
        });
    });
}

pub trait AddAsset {
    fn add_asset<T: Asset>(&mut self) -> &mut Self;
    fn init_asset_loader<T: BoxedImporter + FromWorld>(
        &mut self,
        extensions: &'static [&'static str],
    ) -> &mut Self;
    fn add_asset_loader<T: BoxedImporter>(
        &mut self,
        extensions: &'static [&'static str],
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

        self.world
            .get_resource_mut::<AssetResources>()
            .unwrap()
            .add::<A>();

        self.register_type::<Handle<A>>()
            .add_event::<AssetEvent<A>>()
            .add_system_to_stage(
                AssetStage::LoadAssets,
                Assets::<A>::asset_event_system, // .after(AssetSystem::ProcessAssetEvents),
            );

        self
    }

    fn init_asset_loader<T: BoxedImporter + FromWorld>(
        &mut self,
        extensions: &'static [&'static str],
    ) -> &mut Self {
        let loader = T::from_world(&mut self.world);
        Self::add_asset_loader(self, extensions, loader)
    }

    fn add_asset_loader<T: BoxedImporter>(
        &mut self,
        extensions: &'static [&'static str],
        loader: T,
    ) -> &mut Self {
        self.world
            .get_resource_or_insert_with(AssetLoaders::default)
            .0
            .push((extensions, Box::new(loader)));
        self
    }
}

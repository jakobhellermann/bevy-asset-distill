use bevy_app::prelude::*;
use bevy_app::ScheduleRunnerPlugin;
use bevy_asset_distill::importer::RonImporter;
use bevy_asset_distill::prelude::*;
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_log::LogPlugin;

#[derive(Serialize, Deserialize, TypeUuid, Debug)]
#[uuid = "fab4249b-f95d-411d-a017-7549df090a4f"]
pub struct CustomAsset {
    pub cool_string: String,
}

fn main() {
    App::new()
        .add_plugin(ScheduleRunnerPlugin::default())
        .add_plugin(LogPlugin)
        .add_asset_loader("casset", RonImporter::<CustomAsset>::new())
        .add_plugin(AssetPlugin)
        .add_asset::<CustomAsset>()
        .add_startup_system(setup)
        .add_system(system)
        .run();
}

struct HandleComponent(Handle<CustomAsset>);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load("custom_asset.casset");
    dbg!(&handle);

    commands.spawn().insert(HandleComponent(handle));
}

fn system(
    mut has_printed: Local<bool>,
    asset_server: Res<AssetServer>,
    query: Query<&HandleComponent>,
    custom_assets: Res<Assets<CustomAsset>>,
    mut asset_events: EventReader<AssetEvent<CustomAsset>>,
) {
    let handle = &query.single().0;

    for event in asset_events.iter() {
        info!("Asset event: {:?}", event);
        *has_printed = false;
    }

    if *has_printed {
        return;
    }

    let load_status = asset_server.get_load_status(handle);
    info!("load status: {:?}", load_status);

    let custom_asset = custom_assets.get(handle);

    if let Some(custom_asset) = custom_asset {
        info!("{:?}", custom_asset);
        *has_printed = true;
    }
}

use bevy_app::prelude::*;
use bevy_app::ScheduleRunnerPlugin;
use bevy_asset::importer::RonImporter;
use bevy_asset::prelude::*;
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
        .add_asset_loader(&["casset"], RonImporter::<CustomAsset>::new())
        .add_plugin(AssetPlugin)
        .add_asset::<CustomAsset>()
        .add_startup_system(setup)
        .add_system(system)
        .run();
}

#[derive(Component)]
struct HandleComponent(Handle<CustomAsset>);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load("custom_asset.casset");

    commands.spawn().insert(HandleComponent(handle));
}

fn system(
    mut has_printed: Local<bool>,
    query: Query<&HandleComponent>,
    custom_assets: Res<Assets<CustomAsset>>,
    mut asset_events: EventReader<AssetEvent<CustomAsset>>,
) {
    let handle = &query.single().0;

    asset_events
        .iter()
        .filter(|event| custom_assets.resolve(handle).as_ref() == Some(event.handle()))
        .for_each(|_| *has_printed = false);

    if *has_printed {
        return;
    }
    let custom_asset = match custom_assets.get(handle) {
        Some(custom_asset) => custom_asset,
        None => return,
    };

    info!("{:?}", custom_asset);
    *has_printed = true;
}

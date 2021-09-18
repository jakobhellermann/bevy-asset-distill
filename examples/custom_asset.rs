use bevy_asset_distill::prelude::*;

use bevy_app::prelude::*;
use bevy_app::ScheduleRunnerPlugin;
use bevy_ecs::prelude::*;
use bevy_log::LogPlugin;

#[derive(Serialize, Deserialize, TypeUuid, SerdeImportable, Debug)]
#[uuid = "fab4249b-f95d-411d-a017-7549df090a4f"]
pub struct CustomAsset {
    pub cool_string: String,
}

fn main() {
    App::new()
        .add_plugin(ScheduleRunnerPlugin::default())
        .add_plugin(LogPlugin)
        // .insert_resource(AssetServerSettings::PackfileStatic(include_bytes!("../assets.pack")))
        .add_plugin(AssetPlugin)
        .add_asset::<CustomAsset>()
        .add_startup_system(setup)
        .add_system(system)
        .run();
}

struct HandleComponent(Handle<CustomAsset>);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load("custom_asset.ron");

    commands.spawn().insert(HandleComponent(handle));
}

fn system(
    mut has_printed: Local<bool>,
    query: Query<&HandleComponent>,
    custom_assets: Res<Assets<CustomAsset>>,
    mut asset_events: EventReader<AssetEvent<CustomAsset>>,
) {
    for event in asset_events.iter() {
        println!("Asset event: {:?}", event);
        *has_printed = false;
    }

    if *has_printed {
        return;
    }
    for handle in query.iter() {
        let custom_asset = custom_assets.get(&handle.0);

        if let Some(custom_asset) = custom_asset {
            println!("{:?}", custom_asset);
            *has_printed = true;
        }
    }
}

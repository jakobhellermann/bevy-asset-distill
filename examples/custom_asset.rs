use bevy_asset_distill::prelude::*;
use bevy_asset_distill::{AddAsset, AssetPlugin, Assets};

use bevy_app::prelude::*;
use bevy_app::ScheduleRunnerPlugin;
use bevy_ecs::prelude::*;

#[derive(Serialize, Deserialize, TypeUuid, SerdeImportable, Debug)]
#[uuid = "fab4249b-f95d-411d-a017-7549df090a4f"]
pub struct CustomAsset {
    pub cool_string: String,
}

fn main() {
    App::new()
        .add_plugin(AssetPlugin)
        .add_plugin(ScheduleRunnerPlugin::default())
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
) {
    if *has_printed {
        return;
    }
    for handle in query.iter() {
        let custom_asset = custom_assets.get(&handle.0);

        if let Some(custom_asset) = custom_asset {
            dbg!(&custom_asset);
            *has_printed = true;
        }
    }
}

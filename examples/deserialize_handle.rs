use bevy_asset_distill::importer::RonImporter;
use bevy_asset_distill::prelude::*;

use bevy_app::prelude::*;
use bevy_app::ScheduleRunnerPlugin;
use bevy_ecs::prelude::*;
use bevy_log::LogPlugin;

#[derive(TypeUuid, Deserialize, Serialize, Debug)]
#[uuid = "61d4452b-b891-4016-9404-65c9541e1d49"]
struct StandardMaterial {
    color: [f32; 4],
    texture_by_path: Handle<Texture>,
    texture_by_uuid: Handle<Texture>,
}

#[derive(TypeUuid, Deserialize, Serialize, Debug)]
#[uuid = "1ef01889-ee91-4bc8-8e7d-9d93361a67cc"]
struct Texture {
    bytes: Vec<u8>,
}

fn main() {
    App::new()
        .add_plugin(ScheduleRunnerPlugin::default())
        .add_plugin(LogPlugin::default())
        .add_asset_loader("mat", RonImporter::<StandardMaterial>::new())
        .add_asset_loader("tex", RonImporter::<Texture>::new())
        .add_plugin(AssetPlugin)
        .add_asset::<Texture>()
        .add_asset::<StandardMaterial>()
        .add_startup_system(setup)
        .add_system(system)
        .run();
}

struct HandleComponent<T: Asset>(Handle<T>);
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle: Handle<StandardMaterial> = asset_server.load("deserialize_handle/test.mat");
    commands.spawn().insert(HandleComponent(handle));
}

fn system(
    query: Query<&HandleComponent<StandardMaterial>>,
    materials: Res<Assets<StandardMaterial>>,
    textures: Res<Assets<Texture>>,
) {
    let handle = query.single();
    let material = materials.get(&handle.0);

    if let Some(material) = material {
        let texture = textures.get(&material.texture_by_path).unwrap();
        println!("{:?}", material);
        println!("{:?}", texture);

        std::process::exit(0);
    }
}

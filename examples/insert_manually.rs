use bevy_app::prelude::*;
use bevy_app::ScheduleRunnerSettings;
use bevy_asset_distill::prelude::*;
use bevy_ecs::prelude::*;

use bevy_app::ScheduleRunnerPlugin;

#[derive(Debug, TypeUuid, Deserialize)]
#[uuid = "aee46b37-d4d1-4dcf-812c-ca5fa48eeee5"]
struct Material {
    color: [f32; 4],
}

#[derive(Bundle)]
struct PbrBundle {
    material: Handle<Material>,
}

fn main() {
    App::new()
        .insert_resource(ScheduleRunnerSettings {
            run_mode: bevy_app::RunMode::Once,
        })
        .add_plugin(ScheduleRunnerPlugin)
        .add_plugin(AssetPlugin)
        .add_asset::<Material>()
        .add_startup_system(setup)
        .add_system(system)
        .run();
}

fn setup(mut commands: Commands, mut materials: ResMut<Assets<Material>>) {
    let material = materials.add(Material {
        color: [1.0, 0.0, 1.0, 1.0],
    });

    commands.spawn_bundle(PbrBundle { material });
}

fn system(objects: Query<&Handle<Material>>, materials: ResMut<Assets<Material>>) {
    for material_handle in objects.iter() {
        let material = materials.get(material_handle).unwrap();
        println!("{:?}", material);
    }
}

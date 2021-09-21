use bevy_app::prelude::*;
use bevy_app::{AppExit, ScheduleRunnerPlugin};
use bevy_asset_distill::prelude::*;
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_log::LogPlugin;

#[derive(Serialize, Deserialize, TypeUuid, SerdeImportable, Debug)]
#[uuid = "5812e726-a166-401f-88bf-5b77fa6add0b"]
pub struct Material {
    pub color: [f32; 4],
}

#[derive(Bundle)]
struct PbrBundle {
    material: Handle<Material>,
}

fn main() {
    App::new()
        .add_plugin(ScheduleRunnerPlugin::default())
        .add_plugin(LogPlugin::default())
        .add_plugin(AssetPlugin)
        .add_asset::<Material>()
        .add_startup_system(setup)
        .add_system(system)
        .run();
}

fn setup(mut commands: Commands, asset_server: ResMut<AssetServer>) {
    let material = asset_server.load("material.ron");
    commands.spawn_bundle(PbrBundle { material });
}

fn system(
    objects: Query<&Handle<Material>>,
    materials: ResMut<Assets<Material>>,
    mut app_exit: EventWriter<AppExit>,
) {
    let material_handle = objects.single();
    let material = match materials.get(material_handle) {
        Some(material) => material,
        None => return,
    };

    info!("{:?}", material);

    app_exit.send(AppExit);
}

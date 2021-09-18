use bevy_app::AppExit;
use bevy_asset_distill::prelude::*;

use bevy_app::prelude::*;
use bevy_app::ScheduleRunnerPlugin;
use bevy_ecs::prelude::*;
use bevy_log::LogPlugin;

fn main() {
    App::new()
        .add_plugin(ScheduleRunnerPlugin::default())
        .add_plugin(LogPlugin)
        .insert_resource(AssetServerSettings::Packfile(PackfileSettings::Static(
            include_bytes!("../resources/assets.pack"),
        )))
        .add_asset_loader("txt", bevy_asset_distill::importer::TextImporter)
        .add_plugin(AssetPlugin)
        .add_asset::<String>()
        .add_startup_system(setup)
        .add_system(system)
        .run();
}

struct HandleComponent(Handle<String>);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load("lorem_ipsum.txt");
    commands.spawn().insert(HandleComponent(handle));
}

fn system(
    query: Query<&HandleComponent>,
    text_assets: Res<Assets<String>>,
    mut app_exit: EventWriter<AppExit>,
) {
    let handle = query.single();
    let lorem_ipsum = match text_assets.get(&handle.0) {
        Some(text) => text,
        _ => return,
    };

    println!("{}...", &lorem_ipsum[..100]);
    app_exit.send(AppExit);
}
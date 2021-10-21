use bevy_app::AppExit;
use bevy_asset_distill::prelude::*;

use bevy_app::prelude::*;
use bevy_app::ScheduleRunnerPlugin;
use bevy_asset_distill::util::AssetUuidImporterState;
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_log::LogPlugin;
use bevy_utils::HashMap;
use distill_core::AssetUuid;
use distill_importer::{ImportedAsset, Importer, ImporterValue};

#[derive(Debug, TypeUuid, Deserialize, Serialize)]
#[uuid = "b3099243-1a5a-4f2a-8d41-cfeef8b053ff"]
#[serde(transparent)]
struct TheAsset(String);

#[derive(Deserialize)]
struct SceneSer {
    assets: HashMap<AssetUuid, TheAsset>,
    entities: Vec<Handle<TheAsset>>,
}

#[derive(Debug, TypeUuid, Deserialize, Serialize)]
#[uuid = "1ca9f19d-a862-4b73-9e9a-c514b1c0ee45"]
struct Scene {
    entities: Vec<Handle<TheAsset>>,
}

#[derive(TypeUuid)]
#[uuid = "204bf5cd-2941-479f-a7a8-8782ef1ad9f5"]
struct SceneImporter;
impl Importer for SceneImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        1
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();
    type State = AssetUuidImporterState;

    fn import(
        &self,
        _: &mut distill_importer::ImportOp,
        source: &mut dyn std::io::Read,
        _: &Self::Options,
        state: &mut Self::State,
    ) -> distill_importer::Result<ImporterValue> {
        let data: SceneSer = ron::de::from_reader(source)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

        let inline_assets = data.assets.into_iter().map(|(id, asset)| ImportedAsset {
            id: id,
            search_tags: vec![],
            build_deps: vec![],
            load_deps: vec![],
            build_pipeline: None,
            asset_data: Box::new(asset),
        });

        let data = Scene {
            entities: data.entities,
        };

        let mut assets = vec![];
        assets.push(ImportedAsset {
            id: state.id(),
            search_tags: vec![],
            build_deps: vec![],
            load_deps: vec![],
            build_pipeline: None,
            asset_data: Box::new(data),
        });
        assets.extend(inline_assets);

        Ok(ImporterValue { assets })
    }
}

fn main() {
    App::new()
        .add_plugin(ScheduleRunnerPlugin::default())
        .add_plugin(LogPlugin::default())
        .add_asset_loader(&["scene.ron"], SceneImporter)
        .add_plugin(AssetPlugin)
        .add_asset::<Scene>()
        .add_asset::<TheAsset>()
        .add_startup_system(setup)
        .add_system(system)
        .run();
}

#[derive(Component)]
struct HandleComponent(Handle<Scene>);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle: Handle<Scene> = asset_server.load("a.scene.ron");
    commands.spawn().insert(HandleComponent(handle));
}

fn system(
    query: Query<&HandleComponent>,
    scenes: Res<Assets<Scene>>,
    inner_assets: Res<Assets<TheAsset>>,
    mut app_exit: EventWriter<AppExit>,
) {
    let handle = query.single();
    let scene = match scenes.get(&handle.0) {
        Some(image) => image,
        None => return,
    };

    info!("scene: {:?}", scene);
    info!("asset: {:?}", inner_assets.get(&scene.entities[0]));

    app_exit.send(AppExit);
}

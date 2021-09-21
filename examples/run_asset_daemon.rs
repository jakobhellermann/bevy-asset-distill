use bevy_asset_distill::importer::{RonImporter, TextImporter};
use distill_daemon::AssetDaemon;

fn main() {
    tracing_subscriber::fmt::init();

    let (handle, _) = AssetDaemon::default()
        .with_importer(&["casset"], RonImporter::<assets::CustomAsset>::new())
        .with_importer(&["bmat"], RonImporter::<assets::Material>::new())
        .with_importer(&["tex"], RonImporter::<assets::Texture>::new())
        .with_importer(&["mat"], RonImporter::<assets::StandardMaterial>::new())
        .with_importer(&["txt"], TextImporter)
        .run();
    handle.join().unwrap();
}

// in order to be able to reimport assets on change, the standalone asset daemon needs to have
// the importers defined. Usually you would just import them from somewhere else inside your app,
// but you can't import from example so the types are just copied here.
mod assets {
    use bevy_asset_distill::prelude::*;

    #[derive(Serialize, Deserialize, TypeUuid, Debug)]
    #[uuid = "fab4249b-f95d-411d-a017-7549df090a4f"]
    pub struct CustomAsset {
        pub cool_string: String,
    }
    #[derive(Serialize, Deserialize, TypeUuid, Debug)]
    #[uuid = "5812e726-a166-401f-88bf-5b77fa6add0b"]
    pub struct Material {
        pub color: [f32; 4],
    }

    #[derive(TypeUuid, Deserialize, Serialize, Debug)]
    #[uuid = "61d4452b-b891-4016-9404-65c9541e1d49"]
    pub struct StandardMaterial {
        color: [f32; 4],
        texture_by_path: Handle<Texture>,
        texture_by_uuid: Handle<Texture>,
    }

    #[derive(TypeUuid, Deserialize, Serialize, Debug)]
    #[uuid = "1ef01889-ee91-4bc8-8e7d-9d93361a67cc"]
    pub struct Texture {
        bytes: Vec<u8>,
    }
}

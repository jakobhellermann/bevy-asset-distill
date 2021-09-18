use bevy_asset_distill::prelude::*;

use bevy_app::prelude::*;
use bevy_app::ScheduleRunnerPlugin;
use bevy_asset_distill::util::AssetUuidImporterState;
use bevy_ecs::prelude::*;
use bevy_log::LogPlugin;
use distill_importer::{ImportedAsset, Importer, ImporterValue};
use image::RgbaImage;

#[derive(TypeUuid, Deserialize, Serialize, Clone)]
#[uuid = "86490ac2-84b0-4c3c-94ab-a620bd545b38"]
#[serde(from = "ImageTransfer")]
#[serde(into = "ImageTransfer")]
struct Image(RgbaImage);

#[derive(Deserialize, Serialize)]
struct ImageTransfer(u32, u32, Vec<u8>);
impl From<Image> for ImageTransfer {
    fn from(Image(image): Image) -> Self {
        ImageTransfer(image.width(), image.height(), image.into_raw())
    }
}
impl From<ImageTransfer> for Image {
    fn from(ImageTransfer(width, height, buf): ImageTransfer) -> Image {
        Image(RgbaImage::from_raw(width, height, buf).unwrap())
    }
}

#[derive(TypeUuid)]
#[uuid = "1a8ede27-3963-4ac5-af24-cf95b7cf5640"]
struct ImageImporter;
impl Importer for ImageImporter {
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
        let id = state.id();

        let mut buf = Vec::new();
        source.read_to_end(&mut buf)?;

        let image =
            image::io::Reader::with_format(std::io::Cursor::new(buf), image::ImageFormat::Png)
                .decode()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?
                .to_rgba8();

        let data = Image(image);

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(data),
            }],
        })
    }
}

fn main() {
    App::new()
        .add_plugin(ScheduleRunnerPlugin::default())
        .add_plugin(LogPlugin::default())
        .add_asset_loader("png", ImageImporter)
        .add_plugin(AssetPlugin)
        .add_asset::<Image>()
        .add_startup_system(setup)
        .add_system(system)
        .run();
}

struct HandleComponent(Handle<Image>);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load("base_color.png");
    commands.spawn().insert(HandleComponent(handle));
}

fn system(query: Query<&HandleComponent>, custom_assets: Res<Assets<Image>>) {
    let handle = query.single();
    let custom_asset = custom_assets.get(&handle.0);

    if let Some(custom_asset) = custom_asset {
        println!(
            "Image dimensions are {}x{}",
            custom_asset.0.width(),
            custom_asset.0.height()
        );

        std::process::exit(0);
    }
}

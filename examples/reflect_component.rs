use bevy_asset::importer::text_importer::Text;
use bevy_asset::importer::TextImporter;
use bevy_asset::prelude::*;

use bevy_app::prelude::*;
use bevy_app::ScheduleRunnerPlugin;
use bevy_core::CorePlugin;
use bevy_ecs::prelude::*;
use bevy_log::{info, LogPlugin};

fn main() {
    let mut app = App::new();
    app.add_plugin(ScheduleRunnerPlugin::default())
        .add_plugin(LogPlugin::default())
        .add_plugin(CorePlugin)
        .add_plugin(AssetPlugin)
        .init_asset_loader::<TextImporter>(&["txt"])
        .init_asset_loader::<ReflectImporter>(&["component.ron"])
        .add_asset::<Text>()
        .add_asset_seeded::<ReflectAsset, ReflectAssetDeserializer>()
        .register_type::<Vec<String>>()
        .add_startup_system(setup)
        .add_system(system);

    app.run();
}

struct HandleComponent(Handle<ReflectAsset>);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle: Handle<ReflectAsset> = asset_server.load("test.component.ron");
    commands.spawn().insert(HandleComponent(handle));
}

#[derive(Reflect, Default, Debug)]
struct StandardMaterial {
    a: f32,
    handle: Option<Handle<Text>>,
}

fn system(
    mut has_printed: Local<bool>,
    query: Query<&HandleComponent>,
    reflect_values: Res<Assets<ReflectAsset>>,
    text_assets: Res<Assets<Text>>,
    mut asset_events: EventReader<AssetEvent<ReflectAsset>>,
) {
    asset_events.iter().for_each(|_| *has_printed = false);

    if *has_printed {
        return;
    }

    let handle = query.single();
    let reflect = match reflect_values.get(&handle.0) {
        Some(asset) => &*asset.0,
        None => return,
    };

    let mut material = StandardMaterial::default();
    material.apply(reflect);

    info!("{:?}", material);
    info!("{:?}", text_assets.get(material.handle.as_ref().unwrap()));

    *has_printed = true;
}

struct DebugReflect<'a>(&'a dyn Reflect);
impl std::fmt::Debug for DebugReflect<'_> {
    fn fmt(&self, f: &mut serde::__private::Formatter<'_>) -> std::fmt::Result {
        use std::borrow::Cow;
        match self.0.reflect_ref() {
            bevy_reflect::ReflectRef::Struct(strukt) => {
                let mut f = f.debug_struct(strukt.type_name());
                for (i, field) in strukt.iter_fields().enumerate() {
                    let name = strukt
                        .name_at(i)
                        .map(Cow::Borrowed)
                        .unwrap_or_else(|| Cow::Owned(i.to_string()));
                    f.field(&name, &DebugReflect(field));
                }

                f.finish()
            }
            bevy_reflect::ReflectRef::TupleStruct(_) => todo!(),
            bevy_reflect::ReflectRef::Tuple(_) => todo!(),
            bevy_reflect::ReflectRef::List(list) => f.debug_list().entries(list.iter()).finish(),
            bevy_reflect::ReflectRef::Map(_) => todo!(),
            bevy_reflect::ReflectRef::Value(value) => value.fmt(f),
        }
    }
}

use bevy_asset::util::AssetUuidImporterState;
use bevy_reflect::serde::{ReflectDeserializer, ReflectSerializer};
use bevy_reflect::{Reflect, TypeRegistryArc};
use distill_importer::{ImportedAsset, Importer, ImporterValue};
use serde::de::DeserializeSeed;

#[derive(TypeUuid)]
#[uuid = "17c56d21-f3bc-49e2-8ef8-7ad17349d067"]
pub struct ReflectImporter {
    type_registry: TypeRegistryArc,
}

impl FromWorld for ReflectImporter {
    fn from_world(world: &mut World) -> Self {
        ReflectImporter {
            type_registry: world.get_resource::<TypeRegistryArc>().unwrap().clone(),
        }
    }
}

impl Importer for ReflectImporter {
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
        dbg!("reimport");
        let err = |e| Box::new(e) as Box<dyn std::error::Error + Send>;

        let mut buf = Vec::new();
        source.read_to_end(&mut buf)?;

        let mut deserializer = ron::Deserializer::from_bytes(&buf).map_err(err)?;

        let type_registry = self.type_registry.read();
        let component_deserializer = bevy_reflect::serde::ReflectDeserializer::new(&*type_registry);

        let component = component_deserializer
            .deserialize(&mut deserializer)
            .map_err(err)?;

        let data = ReflectTransfer(component, self.type_registry.clone());

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id: state.id(),
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(data),
            }],
        })
    }
}

#[derive(TypeUuid)]
#[uuid = "8e202064-f432-421a-b615-3d353bc6058e"]
struct ReflectTransfer(Box<dyn Reflect>, TypeRegistryArc);
impl Serialize for ReflectTransfer {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let registry = self.1.read();
        ReflectSerializer::new(&*self.0, &*registry).serialize(serializer)
    }
}

impl From<ReflectTransfer> for ReflectAsset {
    fn from(transfer: ReflectTransfer) -> Self {
        ReflectAsset(transfer.0)
    }
}

#[derive(TypeUuid)]
#[uuid = "8e202064-f432-421a-b615-3d353bc6058e"]
pub struct ReflectAsset(Box<dyn Reflect>);

#[derive(Clone)]
struct ReflectAssetDeserializer(TypeRegistryArc);
impl FromWorld for ReflectAssetDeserializer {
    fn from_world(world: &mut World) -> Self {
        ReflectAssetDeserializer(world.get_resource::<TypeRegistryArc>().unwrap().clone())
    }
}

impl<'de> DeserializeSeed<'de> for ReflectAssetDeserializer {
    type Value = ReflectAsset;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        let component = ReflectDeserializer::new(&*self.0.read()).deserialize(deserializer)?;
        Ok(ReflectAsset(component))
    }
}

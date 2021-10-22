use std::ops::Deref;

use crate::prelude::*;
use crate::util::AssetUuidImporterState;
use distill_importer::{ImportedAsset, Importer, ImporterValue};

#[derive(TypeUuid, Default)]
#[uuid = "5dc1ef8a-4b0c-423f-9d60-e953292e2d1d"]
pub struct TextImporter;

#[derive(TypeUuid, Serialize, Deserialize, Debug)]
#[serde(transparent)]
#[uuid = "7c877a39-ee66-4295-a123-eb12fd8e147e"]
pub struct Text(pub String);
impl Deref for Text {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Importer for TextImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        1
    }

    fn version(&self) -> u32 {
        1
    }

    type Options = ();
    type State = AssetUuidImporterState;

    fn import(
        &self,
        _: &mut distill_importer::ImportOp,
        source: &mut dyn std::io::Read,
        _: &Self::Options,
        state: &mut Self::State,
    ) -> Result<ImporterValue, distill_importer::Error> {
        let mut string = String::new();
        source.read_to_string(&mut string)?;

        let id = state.id();

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(Text(string)),
            }],
        })
    }
}

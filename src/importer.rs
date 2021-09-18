use crate::prelude::*;
use distill_importer::{ImportedAsset, Importer, ImporterValue};

use crate::util::AssetUuidImporterState;

#[derive(TypeUuid)]
#[uuid = "5dc1ef8a-4b0c-423f-9d60-e953292e2d1d"]
pub struct TextImporter;

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
                asset_data: Box::new(string),
            }],
        })
    }
}

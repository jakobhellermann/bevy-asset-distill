use std::marker::PhantomData;

use distill_importer::{ImportedAsset, Importer, ImporterValue};

use crate::prelude::*;
use crate::util::AssetUuidImporterState;

pub struct RonImporter<A: Asset + Serialize>(PhantomData<A>);
#[cfg(feature = "ron-importer")]
impl<A: Asset + Serialize> TypeUuid for RonImporter<A> {
    const UUID: [u8; 16] = [
        247, 147, 34, 237, 214, 174, 75, 180, 169, 124, 10, 136, 213, 57, 10, 161,
    ];
}

#[cfg(feature = "ron-importer")]
impl<A: Asset + Serialize> RonImporter<A> {
    pub fn new() -> Self {
        RonImporter(PhantomData)
    }
}
#[cfg(feature = "ron-importer")]
impl<A: Asset + Serialize> Importer for RonImporter<A> {
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
    ) -> distill_importer::Result<ImporterValue> {
        let data: A = ron::de::from_reader(source)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

        let id = state.id();

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

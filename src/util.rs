use crate::prelude::*;
use distill_core::AssetUuid;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "c3b191fe-1143-4187-b0e1-1ea7a33e47cb"]
pub struct AssetUuidImporterState(Option<AssetUuid>);
impl AssetUuidImporterState {
    pub fn id(&mut self) -> AssetUuid {
        *self
            .0
            .get_or_insert_with(|| AssetUuid(*Uuid::new_v4().as_bytes()))
    }
}

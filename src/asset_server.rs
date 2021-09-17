use std::str::FromStr;
use std::sync::Arc;

use distill::core::{uuid, AssetTypeId, AssetUuid};
use distill::loader::crossbeam_channel::Sender;
use distill::loader::handle::{AssetHandle, RefOp};
use distill::loader::storage::{IndirectIdentifier, LoadInfo, LoadStatus};
use distill::loader::{LoadHandle, Loader};

use crate::prelude::*;

pub struct AssetServer {
    loader: Loader,
    refop_sender: Arc<Sender<RefOp>>,
}

#[derive(Debug, Clone)]
pub enum AssetLoadRef {
    UUID(AssetUuid),
    Indirect(IndirectIdentifier),
}

impl From<AssetUuid> for AssetLoadRef {
    fn from(uuid: AssetUuid) -> Self {
        AssetLoadRef::UUID(uuid)
    }
}
impl From<IndirectIdentifier> for AssetLoadRef {
    fn from(id: IndirectIdentifier) -> Self {
        AssetLoadRef::Indirect(id)
    }
}
impl From<&str> for AssetLoadRef {
    fn from(str: &str) -> Self {
        str.parse().unwrap()
    }
}

impl FromStr for AssetLoadRef {
    type Err = ParseAssetPathError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(uuid) = uuid::Uuid::parse_str(s) {
            return Ok(AssetLoadRef::UUID(AssetUuid(*uuid.as_bytes())));
        }

        parse_asset_path(s).map(AssetLoadRef::Indirect)
    }
}

// TODO impl error
#[derive(Debug)]
pub enum ParseAssetPathError {
    LabelWithoutAssetTypeId,
    UUID(uuid::Error),
}
// path.ron
// scene.gltf#Mesh0@80a27027-221a-4fb6-8456-fed18acd12d7
// scene.gltf@80a27027-221a-4fb6-8456-fed18acd12d7
fn parse_asset_path(asset_path: &str) -> Result<IndirectIdentifier, ParseAssetPathError> {
    fn rsplit_once_or_all(input: &str, separator: char) -> (&str, Option<&str>) {
        match input.rsplit_once(separator) {
            Some((before, id)) => (before, Some(id)),
            None => (input, None),
        }
    }

    let (path_maybe_label, asset_type) = rsplit_once_or_all(asset_path, '@');
    let (path, tag) = rsplit_once_or_all(path_maybe_label, '#');

    let asset_type = asset_type
        .map(uuid::Uuid::parse_str)
        .transpose()
        .map_err(ParseAssetPathError::UUID)?
        .map(|uuid| AssetTypeId(*uuid.as_bytes()));

    let id = match (tag, asset_type) {
        (None, None) => IndirectIdentifier::Path(path.to_string()),
        (None, Some(asset_type)) => IndirectIdentifier::PathWithType(path.to_string(), asset_type),
        (Some(_), None) => return Err(ParseAssetPathError::LabelWithoutAssetTypeId),
        (Some(tag), Some(asset_type)) => {
            IndirectIdentifier::PathWithTagAndType(path.to_string(), tag.to_string(), asset_type)
        }
    };

    Ok(id)
}

impl AssetServer {
    pub fn new(loader: Loader, refop_sender: Arc<Sender<RefOp>>) -> AssetServer {
        AssetServer {
            loader,
            refop_sender,
        }
    }

    pub fn loader(&self) -> &Loader {
        &self.loader
    }
    pub fn loader_mut(&mut self) -> &mut Loader {
        &mut self.loader
    }

    pub fn load<A: Asset>(&self, load: impl Into<AssetLoadRef>) -> Handle<A> {
        let load_handle = self.load_internal(load.into());
        let handle = Handle::<A>::new((*self.refop_sender).clone(), load_handle);
        handle
    }
    pub fn load_untyped(&self, load: impl Into<AssetLoadRef>) -> HandleUntyped {
        let load_handle = self.load_internal(load.into());
        let handle = HandleUntyped::new((*self.refop_sender).clone(), load_handle);
        handle
    }

    fn load_internal(&self, load: AssetLoadRef) -> LoadHandle {
        match load {
            AssetLoadRef::UUID(uuid) => self.loader.add_ref(uuid),
            AssetLoadRef::Indirect(id) => self.loader.add_ref_indirect(id),
        }
    }

    pub fn get_load_status<A: AssetHandle>(&self, handle: A) -> LoadStatus {
        self.loader.get_load_status(handle.load_handle())
    }

    pub fn get_load_info<A: AssetHandle>(&self, handle: A) -> Option<LoadInfo> {
        self.loader.get_load_info(handle.load_handle())
    }
}

use crate::prelude::*;
use bevy_reflect::prelude::*;
use std::borrow::Borrow;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use bevy_ecs::prelude::Component;
use distill_loader::crossbeam_channel::Sender;
use distill_loader::handle::{self, AssetHandle, RefOp};
use distill_loader::LoadHandle;
use serde::Serialize;

#[derive(Component)]
pub struct Handle<A: Asset>(handle::Handle<A>);

impl<A: Asset> Handle<A> {
    pub(crate) fn new(refop_sender: Sender<RefOp>, load_handle: LoadHandle) -> Handle<A> {
        Handle(handle::Handle::new(refop_sender, load_handle))
    }

    pub fn clone_weak(&self) -> WeakHandle<A> {
        WeakHandle::new(self.load_handle())
    }
}
impl<A: Asset> AssetHandle for Handle<A> {
    fn load_handle(&self) -> LoadHandle {
        self.0.load_handle()
    }
}
impl<A: Asset> AssetHandle for &Handle<A> {
    fn load_handle(&self) -> LoadHandle {
        self.0.load_handle()
    }
}
impl<A: Asset> Debug for Handle<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Handle").field(&self.0).finish()
    }
}
impl<A: Asset> Clone for Handle<A> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<A: Asset> Hash for Handle<A> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}
impl<A: Asset> PartialEq for Handle<A> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<A: Asset> Eq for Handle<A> {}
impl<A: Asset> Serialize for Handle<A> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}
impl<'de, A: Asset> Deserialize<'de> for Handle<A> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        handle::Handle::deserialize(deserializer).map(Handle)
    }
}

impl<A: Asset> bevy_reflect::GetTypeRegistration for Handle<A> {
    fn get_type_registration() -> bevy_reflect::TypeRegistration {
        let registration = bevy_reflect::TypeRegistration::of::<Handle<A>>();
        // TODO ReflectComponent
        // let reflect_component = ReflectComponent {};
        // registration.insert::<ReflectComponent>(reflect_component);
        registration
    }
}
unsafe impl<A: Asset> Reflect for Handle<A> {
    fn type_name(&self) -> &str {
        std::any::type_name::<Handle<A>>()
    }

    fn any(&self) -> &dyn std::any::Any {
        self
    }

    fn any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn apply(&mut self, other: &dyn Reflect) {
        *self = other
            .downcast_ref::<Self>()
            .unwrap_or_else(|| panic!("Value is not {}", std::any::type_name::<Self>()))
            .clone();
    }

    fn set(&mut self, other: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        other.downcast().map(|other| *self = *other)
    }

    fn reflect_ref(&self) -> bevy_reflect::ReflectRef {
        bevy_reflect::ReflectRef::Value(self)
    }

    fn reflect_mut(&mut self) -> bevy_reflect::ReflectMut {
        bevy_reflect::ReflectMut::Value(self)
    }

    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone()) as Box<dyn Reflect>
    }

    fn reflect_hash(&self) -> Option<u64> {
        None
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        value.downcast_ref::<Self>().map(|value| value == self)
    }

    fn serializable(&self) -> Option<bevy_reflect::serde::Serializable> {
        Some(bevy_reflect::serde::Serializable::Owned(Box::new(self)))
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct HandleUntyped(handle::GenericHandle);
impl HandleUntyped {
    pub(crate) fn new(refop_sender: Sender<RefOp>, load_handle: LoadHandle) -> HandleUntyped {
        HandleUntyped(handle::GenericHandle::new(refop_sender, load_handle))
    }
}

impl AssetHandle for HandleUntyped {
    fn load_handle(&self) -> LoadHandle {
        self.0.load_handle()
    }
}
impl AssetHandle for &HandleUntyped {
    fn load_handle(&self) -> LoadHandle {
        self.0.load_handle()
    }
}
impl Serialize for HandleUntyped {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for HandleUntyped {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        handle::GenericHandle::deserialize(deserializer).map(HandleUntyped)
    }
}

impl bevy_reflect::GetTypeRegistration for HandleUntyped {
    fn get_type_registration() -> bevy_reflect::TypeRegistration {
        let registration = bevy_reflect::TypeRegistration::of::<HandleUntyped>();
        // TODO ReflectComponent
        // let reflect_component = ReflectComponent {};
        // registration.insert::<ReflectComponent>(reflect_component);
        registration
    }
}
unsafe impl Reflect for HandleUntyped {
    fn type_name(&self) -> &str {
        std::any::type_name::<HandleUntyped>()
    }

    fn any(&self) -> &dyn std::any::Any {
        self
    }

    fn any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn apply(&mut self, other: &dyn Reflect) {
        *self = other
            .downcast_ref::<Self>()
            .unwrap_or_else(|| panic!("Value is not {}", std::any::type_name::<Self>()))
            .clone();
    }

    fn set(&mut self, other: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        other.downcast().map(|other| *self = *other)
    }

    fn reflect_ref(&self) -> bevy_reflect::ReflectRef {
        bevy_reflect::ReflectRef::Value(self)
    }

    fn reflect_mut(&mut self) -> bevy_reflect::ReflectMut {
        bevy_reflect::ReflectMut::Value(self)
    }

    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone()) as Box<dyn Reflect>
    }

    fn reflect_hash(&self) -> Option<u64> {
        None
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        value.downcast_ref::<Self>().map(|value| value == self)
    }

    fn serializable(&self) -> Option<bevy_reflect::serde::Serializable> {
        Some(bevy_reflect::serde::Serializable::Owned(Box::new(self)))
    }
}

#[repr(transparent)]
pub struct WeakHandle<A: Asset>(handle::WeakHandle, PhantomData<A>);
impl<A: Asset> WeakHandle<A> {
    pub(crate) fn new(load_handle: LoadHandle) -> WeakHandle<A> {
        WeakHandle(handle::WeakHandle::new(load_handle), PhantomData)
    }

    fn ref_from_raw(handle: &handle::WeakHandle) -> &WeakHandle<A> {
        // Safety: WeakHandle is #[repr(transparent)]
        unsafe { std::mem::transmute(handle) }
    }
}

impl<A: Asset> AssetHandle for WeakHandle<A> {
    fn load_handle(&self) -> LoadHandle {
        self.0.load_handle()
    }
}
impl<A: Asset> AssetHandle for &WeakHandle<A> {
    fn load_handle(&self) -> LoadHandle {
        self.0.load_handle()
    }
}
impl<A: Asset> Debug for WeakHandle<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Handle").field(&self.0).finish()
    }
}
impl<A: Asset> Clone for WeakHandle<A> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}
impl<A: Asset> Copy for WeakHandle<A> {}
impl<A: Asset> Hash for WeakHandle<A> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}
impl<A: Asset> PartialEq for WeakHandle<A> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<A: Asset> PartialEq<WeakHandle<A>> for Handle<A> {
    fn eq(&self, other: &WeakHandle<A>) -> bool {
        self.load_handle() == other.load_handle()
    }
}

impl<A: Asset> Eq for WeakHandle<A> {}
impl<A: Asset> PartialEq<Handle<A>> for WeakHandle<A> {
    fn eq(&self, other: &Handle<A>) -> bool {
        self.load_handle() == other.load_handle()
    }
}
impl<A: Asset> std::cmp::PartialOrd for WeakHandle<A> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<A: Asset> std::cmp::Ord for WeakHandle<A> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.load_handle().0.cmp(&other.load_handle().0)
    }
}
impl<A: Asset> Borrow<WeakHandle<A>> for Handle<A> {
    fn borrow(&self) -> &WeakHandle<A> {
        let handle: &handle::WeakHandle =
            std::borrow::Borrow::<handle::WeakHandle>::borrow(&self.0);
        WeakHandle::ref_from_raw(handle)
    }
}

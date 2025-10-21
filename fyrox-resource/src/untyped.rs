// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! A module for untyped resources. See [`UntypedResource`] docs for more info.

use crate::state::ResourceDataWrapper;
use crate::{
    core::{
        math::curve::Curve, parking_lot::Mutex, reflect::prelude::*, uuid, uuid::Uuid,
        visitor::prelude::*, TypeUuidProvider,
    },
    manager::ResourceManager,
    state::{LoadError, ResourceState},
    Resource, ResourceData, ResourceLoadError, TypedResourceData, CURVE_RESOURCE_UUID,
    MODEL_RESOURCE_UUID, SHADER_RESOURCE_UUID, SOUND_BUFFER_RESOURCE_UUID, TEXTURE_RESOURCE_UUID,
};
use crate::{ResourceHeaderGuard, FONT_RESOURCE_UUID, HRIR_SPHERE_RESOURCE_UUID};
use fxhash::FxHasher64;
use fyrox_core::io::FileError;
use fyrox_core::log::Log;
use fyrox_core::parking_lot::MutexGuard;
use fyrox_core::SafeLock;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::{
    error::Error,
    ffi::OsStr,
    fmt::{Debug, Display, Formatter},
    future::Future,
    hash::{Hash, Hasher},
    marker::PhantomData,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

const MISSING_RESOURCE_MANAGER: &str =
    "Resource data constructor container must be provided when serializing resources!";

/// The UUIDs for resources that cannot be visited as embedded data because none of their
/// fields are visited. This is used by `LegacyHeader` to avoid mistakenly interpretting
/// a visitor node as an embedded resource.
/// Being on this list does *not* prevent future versions of these resources from being
/// embedded. It is purely a heuristic to assist with reading legacy resources.
const INVALID_EMBEDDED_RESOURCES: &[Uuid] = &[
    SOUND_BUFFER_RESOURCE_UUID,
    MODEL_RESOURCE_UUID,
    HRIR_SPHERE_RESOURCE_UUID,
    FONT_RESOURCE_UUID,
];

#[derive(Default, Debug, Visit, Clone, PartialEq, Eq, Hash)]
enum OldResourceKind {
    #[default]
    Embedded,
    External(PathBuf),
}

#[derive(Debug)]
enum LegacyHeader {
    Path(PathBuf),
    Uuid(Uuid),
    Data(Box<dyn ResourceData>),
}

impl LegacyHeader {
    fn visit_path(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut path = PathBuf::default();
        path.visit(name, visitor)?;
        if path.as_os_str().is_empty() {
            return Err(VisitError::FileLoadError(FileError::Custom(
                "Empty path".to_string(),
            )));
        }
        *self = Self::Path(path);
        Ok(())
    }
    fn visit_details(&mut self, type_uuid: Uuid, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region("Details")?;
        if type_uuid == SOUND_BUFFER_RESOURCE_UUID {
            let mut sound_region = region.enter_region("0")?;
            self.visit_path("Path", &mut sound_region)
        } else {
            self.visit_path("Path", &mut region)
        }
    }
    fn take_data(&mut self, uuid: Uuid) -> Option<Box<dyn ResourceData>> {
        if let Self::Data(data) = std::mem::replace(self, Self::Uuid(uuid)) {
            Some(data)
        } else {
            None
        }
    }
    fn is_valid_embedded_type_uuid(uuid: Uuid) -> bool {
        INVALID_EMBEDDED_RESOURCES.contains(&uuid)
    }
}

impl Default for LegacyHeader {
    fn default() -> Self {
        Self::Path(PathBuf::default())
    }
}

impl Visit for LegacyHeader {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        assert!(visitor.is_reading());
        let mut region = visitor.enter_region(name)?;

        let mut type_uuid = Uuid::default();
        if type_uuid.visit("TypeUuid", &mut region).is_err() {
            // We might be reading the old version, try to guess an actual type uuid by
            // the inner content of the resource data.
            type_uuid = guess_uuid(&mut region);
        };
        let resource_manager = region
            .blackboard
            .get::<ResourceManager>()
            .expect(MISSING_RESOURCE_MANAGER);
        let resource_manager_state = resource_manager.state();
        let Some(mut instance) = resource_manager_state
            .constructors_container
            .try_create(&type_uuid)
        else {
            return Err(VisitError::User(format!(
                "There's no constructor registered for type {type_uuid}!"
            )));
        };
        drop(resource_manager_state);

        let mut id: u32 = 0;

        if id.visit("Id", &mut region).is_ok() {
            // We're interested only in embedded resources.
            if id == 2 {
                let result = self.visit_details(type_uuid, &mut region);
                if let Err(err0) = result {
                    if Self::is_valid_embedded_type_uuid(type_uuid) {
                        let result = instance.visit("Details", &mut region);
                        if let Err(err1) = result {
                            let result = instance.visit("State", &mut region);
                            if let Err(err2) = result {
                                return Err(err0.multiple(err1).multiple(err2));
                            }
                        }
                        *self = Self::Data(instance);
                        Ok(())
                    } else {
                        Err(err0)
                    }
                } else {
                    result
                }
            } else {
                Err(VisitError::User("Old resource".into()))
            }
        } else {
            let mut uuid = Uuid::default();
            if uuid.visit("ResourceUuid", &mut region).is_ok() && !uuid.is_nil() {
                *self = Self::Uuid(uuid);
                return Ok(());
            }
            let mut old_kind = OldResourceKind::Embedded;
            old_kind.visit("Kind", &mut region)?;
            match old_kind {
                OldResourceKind::External(path) => {
                    if path.as_os_str().is_empty() {
                        return Err(VisitError::FileLoadError(FileError::Custom(
                            "Empty path".to_string(),
                        )));
                    }
                    *self = Self::Path(path);
                }
                OldResourceKind::Embedded => {
                    instance.visit("State", &mut region)?;
                    *self = Self::Data(instance);
                }
            }
            Ok(())
        }
    }
}

// Heuristic function to guess resource uuid based on inner content of a resource.
fn guess_uuid(region: &mut Visitor) -> Uuid {
    assert!(region.is_reading());

    let guard = region.enter_region("Details");
    let mut region = match guard {
        Ok(region) => region,
        Err(ref err) => {
            Log::err(err.to_string());
            drop(guard);
            Log::info(region.debug());
            return TEXTURE_RESOURCE_UUID;
        }
    };

    let mut mip_count = 0u32;
    if mip_count.visit("MipCount", &mut region).is_ok() {
        return TEXTURE_RESOURCE_UUID;
    }

    let mut curve = Curve::default();
    if curve.visit("Curve", &mut region).is_ok() {
        return CURVE_RESOURCE_UUID;
    }

    let mut id = 0u32;
    if id.visit("Id", &mut region).is_ok() {
        return SOUND_BUFFER_RESOURCE_UUID;
    }

    let mut path = PathBuf::new();
    if path.visit("Path", &mut region).is_ok() {
        let ext = path.extension().unwrap_or_default().to_ascii_lowercase();
        if ext == OsStr::new("rgs")
            || ext == OsStr::new("fbx")
            || ext == OsStr::new("gltf")
            || ext == OsStr::new("glb")
        {
            return MODEL_RESOURCE_UUID;
        } else if ext == OsStr::new("shader")
            || path == OsStr::new("Standard")
            || path == OsStr::new("StandardTwoSides")
            || path == OsStr::new("StandardTerrain")
        {
            return SHADER_RESOURCE_UUID;
        }
    }

    Default::default()
}

/// Kind of a resource. It defines how the resource manager will treat a resource content on serialization.
#[derive(Default, Reflect, Debug, Visit, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    /// The content of embedded resources will be fully serialized.
    #[default]
    Embedded,
    /// The content of external resources will not be serialized, instead only the path to the content
    /// will be serialized and the content will be loaded from it when needed.
    ///
    /// ## Built-in Resources
    ///
    /// This resource kind could also be used to create built-in resources (the data of which is
    /// embedded directly in the executable using [`include_bytes`] macro). All that is needed is to
    /// create a static resource variable and register it in built-in resources of the resource manager.
    /// In this case, the path becomes an identifier and it must be unique. See [`ResourceManager`] docs
    /// for more info about built-in resources.
    External,
}

impl ResourceKind {
    /// Switches the resource kind to [`Self::External`].
    #[inline]
    pub fn make_external(&mut self) {
        *self = Self::External;
    }

    /// Switches the resource kind to [`Self::Embedded`]
    #[inline]
    pub fn make_embedded(&mut self) {
        *self = Self::Embedded;
    }

    /// Checks, if the resource kind is [`Self::Embedded`]
    #[inline]
    pub fn is_embedded(&self) -> bool {
        matches!(self, Self::Embedded)
    }

    /// Checks, if the resource kind is [`Self::External`]
    #[inline]
    pub fn is_external(&self) -> bool {
        !self.is_embedded()
    }
}

impl Display for ResourceKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Embedded => {
                write!(f, "Embedded")
            }
            Self::External => {
                write!(f, "External")
            }
        }
    }
}

/// Header of a resource, it contains a common data about the resource, such as its data type uuid,
/// its kind, etc.
#[derive(Reflect, Clone, Debug)]
pub struct ResourceHeader {
    /// The unique identifier of this resource.
    pub uuid: Uuid,
    /// Kind of the resource. See [`ResourceKind`] for more info.
    pub kind: ResourceKind,
    /// Actual state of the resource. See [`ResourceState`] for more info.
    pub state: ResourceState,
}

impl Default for ResourceHeader {
    fn default() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            kind: Default::default(),
            state: Default::default(),
        }
    }
}

impl From<Uuid> for ResourceHeader {
    fn from(uuid: Uuid) -> Self {
        Self {
            uuid,
            kind: ResourceKind::External,
            state: ResourceState::Unloaded,
        }
    }
}

impl ResourceHeader {
    /// The type of the data, if this resource is Ok.
    pub fn type_uuid(&self) -> Option<Uuid> {
        if let ResourceState::Ok { data } = &self.state {
            Some(data.type_uuid())
        } else {
            None
        }
    }
}

impl Display for ResourceHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.uuid, f)?;
        f.write_char(':')?;
        match self.kind {
            ResourceKind::Embedded => f.write_str("Embed")?,
            ResourceKind::External => f.write_str("Extern")?,
        }
        f.write_char(':')?;
        match &self.state {
            ResourceState::Unloaded => f.write_str("Unloaded"),
            ResourceState::Pending { .. } => f.write_str("Pending"),
            ResourceState::LoadError { path, error } => write!(f, "Error({path:?}, {error})"),
            ResourceState::Ok { .. } => f.write_str("Ok"),
        }
    }
}

impl Visit for ResourceHeader {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;
        if region.is_reading() {
            self.kind = ResourceKind::Embedded;
            let mut actual_type_uuid = Uuid::default();
            actual_type_uuid.visit("TypeUuid", &mut region)?;
            let resource_manager = region
                .blackboard
                .get::<ResourceManager>()
                .expect(MISSING_RESOURCE_MANAGER)
                .clone();
            let Some(mut data) = resource_manager
                .state()
                .constructors_container
                .try_create(&actual_type_uuid)
            else {
                return Err(VisitError::User(format!(
                    "There's no constructor registered for type {actual_type_uuid}!"
                )));
            };
            data.visit("Data", &mut region)?;
            self.state = ResourceState::Ok {
                data: ResourceDataWrapper(data),
            };
            Ok(())
        } else {
            match (&self.kind, &mut self.state) {
                (ResourceKind::Embedded, ResourceState::Ok { data }) => {
                    let mut type_uuid = data.type_uuid();
                    type_uuid.visit("TypeUuid", &mut region)?;
                    data.visit("Data", &mut region)
                }
                (ResourceKind::External, _) => {
                    Err(VisitError::User("Writing an external resource".into()))
                }
                _ => Err(VisitError::User(
                    "Writing an embedded resource that is not ok.".into(),
                )),
            }
        }
    }
}

/// Untyped resource is a universal way of storing arbitrary resource types. Internally it wraps
/// [`ResourceState`] in a `Arc<Mutex<>` so the untyped resource becomes shareable. In most of the
/// cases you don't need to deal with untyped resources, use typed [`Resource`] wrapper instead.
/// Untyped resource could be useful in cases when you need to collect a set of resources of different
/// types in a single collection and do something with them.
///
/// ## Handle
///
/// Since untyped resources stores the actual data in a shared storage, the resource instance could
/// be considered as a handle. Such "handles" have special behaviour on serialization and
/// deserialization to keep pointing to the same storage.
///
/// ## Serialization and Deserialization
///
/// Every resource writes its own kind, type uuid of the data and optionally the data itself.
///
/// Serialization/deserialization of the data is different depending on the actual resource kind
/// (see [`ResourceKind`]):
///
/// 1) [`ResourceKind::Embedded`] - the resource data will be serialized together with the resource
/// handle. The data will be loaded back on deserialization stage from the backing storage.
/// 2) [`ResourceKind::External`] - the resource data won't be serialized and will be reloaded from
/// the external source.
///
/// When the resource is deserialized, the resource system at first looks for an already loaded
/// resource with the same kind and if it is found, replaces current instance with the loaded one.
/// If not - loads the resource and also replaces the instance. This step is crucial for uniqueness
/// of the resource handles.
///
/// To put everything simple: when you save a resource handle, it writes only path to it, then when
/// you load it you need to make sure that all references to a resource points to the same resource
/// instance.
///
/// ## Default state
///
/// Default state of every untyped resource is [`ResourceState::LoadError`] with a warning message,
/// that the resource is in default state. This is a trade-off to prevent wrapping internals into
/// `Option`, that in some cases could lead to convoluted code with lots of `unwrap`s and state
/// assumptions.
#[derive(Default, Clone, Reflect, TypeUuidProvider, Deserialize)]
#[serde(from = "Uuid")]
#[type_uuid(id = "21613484-7145-4d1c-87d8-62fa767560ab")]
pub struct UntypedResource(pub Arc<Mutex<ResourceHeader>>);

impl Serialize for UntypedResource {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let header = self.lock();
        if header.kind == ResourceKind::Embedded {
            panic!("Embedded resources cannot be serialized.");
        }
        header.uuid.serialize(serializer)
    }
}

impl Visit for UntypedResource {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let result = self.visit_with_type_uuid(name, None, visitor);
        if let Err(err) = &result {
            Log::err(format!("Resource error for untyped resource: {err}"));
            if let Ok(region) = visitor.enter_region(name) {
                region.debug();
            }
            self.commit_error(PathBuf::default(), err.to_string());
        }
        result
    }
}

impl Display for UntypedResource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(header) = self.0.try_lock() {
            Display::fmt(&header, f)
        } else {
            f.write_str("locked")
        }
    }
}

impl Debug for UntypedResource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "UntypedResource({self})")
    }
}

impl PartialEq for UntypedResource {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for UntypedResource {}

impl Hash for UntypedResource {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}

impl From<Uuid> for UntypedResource {
    fn from(uuid: Uuid) -> Self {
        ResourceHeader::from(uuid).into()
    }
}

impl From<ResourceHeader> for UntypedResource {
    fn from(header: ResourceHeader) -> Self {
        Self(Arc::new(Mutex::new(header)))
    }
}

impl UntypedResource {
    /// Visit this resource handle with the given UUID for the type of the resource data.
    pub fn visit_with_type_uuid(
        &mut self,
        name: &str,
        type_uuid: Option<Uuid>,
        visitor: &mut Visitor,
    ) -> VisitResult {
        let mut region = visitor.enter_region(name)?;
        if region.is_reading() {
            let mut uuid = Uuid::default();
            match uuid.visit("Uuid", &mut region) {
                Ok(()) => {
                    self.read_visit(uuid, type_uuid, &mut region)?;
                    drop(region);
                }
                Err(_) => {
                    drop(region);
                    self.legacy_visit(name, visitor)?;
                }
            }
            let resource_manager = visitor
                .blackboard
                .get::<ResourceManager>()
                .expect("Resource manager must be available when deserializing resources!")
                .clone();
            resource_manager.state().request_resource(self);
            Ok(())
        } else {
            self.resource_uuid().visit("Uuid", &mut region)?;
            let header_guard = self.lock();
            let is_embedded = header_guard.kind.is_embedded();
            let is_ok = header_guard.state.is_ok();
            drop(header_guard);
            if is_embedded && is_ok {
                self.0.visit("Embedded", &mut region)
            } else if is_embedded {
                true.visit("Default", &mut region)
            } else {
                Ok(())
            }
        }
    }
    fn legacy_visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut header: Arc<Mutex<LegacyHeader>> = Default::default();
        let result = header.visit(name, visitor);
        if let Err(err1) = result {
            header = Arc::default();
            if let Ok(mut region) = visitor.enter_region(name) {
                let mut region = if region.has_region("Value") {
                    region.enter_region("Value").unwrap()
                } else {
                    region
                };
                let result = header.visit("State", &mut region);
                if let Err(err2) = result {
                    if let Ok(mut region) = region.enter_region("State") {
                        let result = header.visit("Data", &mut region);
                        if let Err(err3) = result {
                            return Err(err1.multiple(err2).multiple(err3));
                        }
                    } else {
                        return Err(err1.multiple(err2));
                    }
                }
            } else {
                return Err(err1);
            }
        }
        let resource_manager = visitor
            .blackboard
            .get::<ResourceManager>()
            .expect("Resource manager must be available when deserializing resources!")
            .clone();
        let mut state = resource_manager.state();
        let mut header = header.try_lock().expect("header locked");
        match *header {
            LegacyHeader::Path(ref path) => {
                Log::info(format!("Requesting {path:?}"));
                *self = state.request(path);
            }
            LegacyHeader::Uuid(uuid) => {
                *self = uuid.into();
            }
            LegacyHeader::Data(_) => {
                let uuid = Uuid::new_v4();
                let data = header.take_data(uuid).unwrap();
                *self = Self::new_ok_untyped(uuid, ResourceKind::Embedded, data);
            }
        }
        Ok(())
    }
    fn read_visit(
        &mut self,
        resource_uuid: Uuid,
        type_uuid: Option<Uuid>,
        visitor: &mut Visitor,
    ) -> VisitResult {
        let mut is_default = false;
        if is_default.visit("Default", visitor).is_ok() && is_default {
            *self = Self::default();
            self.lock().uuid = resource_uuid;
            Ok(())
        } else if visitor.has_region("Embedded") {
            self.0.visit("Embedded", visitor)?;
            self.lock().uuid = resource_uuid;
            if let (Some(expected), Some(actual)) = (type_uuid, self.lock().type_uuid()) {
                if expected != actual {
                    return Err(format!(
                        "Unable to deserialize untyped resource into its typed \
                    version, because types do not match! Untyped resource has \
                    {actual} type, but the required type is {expected}.",
                    )
                    .into());
                }
            }
            Ok(())
        } else {
            *self = resource_uuid.into();
            Ok(())
        }
    }
    /// Lock the shared header of this resource.
    pub fn typed_lock<T: TypedResourceData>(&self) -> ResourceHeaderGuard<'_, T> {
        self.lock().into()
    }
    /// Lock the shared header of this resource.
    pub fn lock(&self) -> MutexGuard<'_, ResourceHeader> {
        self.0.safe_lock()
    }
    /// Attempt to lock the shared header. None if the header is already locked.
    pub fn try_typed_lock<T: TypedResourceData>(&self) -> Option<ResourceHeaderGuard<'_, T>> {
        self.try_lock().map(|g| g.into())
    }
    /// Attempt to lock the shared header. None if the header is already locked.
    pub fn try_lock(&self) -> Option<MutexGuard<'_, ResourceHeader>> {
        self.0.try_lock()
    }
    /// Creates new untyped resource in unloaded state with the given UUID.
    pub fn new_unloaded(resource_uuid: Uuid) -> Self {
        ResourceHeader {
            uuid: resource_uuid,
            kind: ResourceKind::External,
            state: ResourceState::Unloaded,
        }
        .into()
    }
    /// Creates new untyped resource in pending state with the given UUID.
    pub fn new_pending(resource_uuid: Uuid, kind: ResourceKind) -> Self {
        ResourceHeader {
            uuid: resource_uuid,
            kind,
            state: ResourceState::new_pending(),
        }
        .into()
    }

    /// Creates new untyped resource in ok (fully loaded) state using the given data of any type, that
    /// implements [`ResourceData`] trait.
    pub fn new_ok<T>(resource_uuid: Uuid, kind: ResourceKind, data: T) -> Self
    where
        T: ResourceData,
    {
        ResourceHeader {
            uuid: resource_uuid,
            kind,
            state: ResourceState::new_ok(data),
        }
        .into()
    }

    /// Creates new untyped resource in ok (fully loaded) state using the given data.
    pub fn new_ok_untyped(
        resource_uuid: Uuid,
        kind: ResourceKind,
        data: Box<dyn ResourceData>,
    ) -> Self {
        ResourceHeader {
            uuid: resource_uuid,
            kind,
            state: ResourceState::new_ok_untyped(data),
        }
        .into()
    }

    /// Creates new untyped resource in ok (fully loaded) state using the given data of any type, that
    /// implements [`ResourceData`] trait. The resource kind is set to [`ResourceKind::Embedded`].
    pub fn new_embedded<T: ResourceData>(data: T) -> Self {
        Self::new_ok(Uuid::new_v4(), ResourceKind::Embedded, data)
    }

    /// Creates new untyped resource in error state.
    pub fn new_load_error(kind: ResourceKind, path: PathBuf, error: LoadError) -> Self {
        ResourceHeader {
            uuid: Uuid::new_v4(),
            kind,
            state: ResourceState::new_load_error(path, error),
        }
        .into()
    }

    /// The UUID of the resource. All resources must have a UUID, even if they are not loaded
    /// because the UUID is how the resource manager knows the path to load from.
    pub fn resource_uuid(&self) -> Uuid {
        self.lock().uuid
    }

    /// Returns actual unique type id of underlying resource data.
    pub fn type_uuid(&self) -> Option<Uuid> {
        let header = self.lock();
        match header.state {
            ResourceState::Ok { ref data, .. } => Some(data.type_uuid()),
            _ => None,
        }
    }

    /// Tries to get an actual unique type id of underlying resource data. Returns `None` if the
    /// resource cannot be locked or if it is not loaded.
    pub fn type_uuid_non_blocking(&self) -> Option<Uuid> {
        let header = self.try_lock()?;
        match header.state {
            ResourceState::Ok { ref data, .. } => Some(data.type_uuid()),
            _ => None,
        }
    }

    /// Tries to get a type name of the resource data. Data type name is available only for fully
    /// loaded resources (in [`ResourceState::Ok`] state).
    pub fn data_type_name(&self) -> Option<String> {
        match self.lock().state {
            ResourceState::Ok { ref data, .. } => Some(Reflect::type_name(&**data).to_string()),
            _ => None,
        }
    }

    /// Same as [`Self::data_type_name`], but returns `Unknown` string if the resource is not in
    /// [`ResourceState::Ok`] state.
    pub fn data_type_name_or_unknown(&self) -> String {
        self.data_type_name()
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// Returns true if the resource has not been requested.
    pub fn is_unloaded(&self) -> bool {
        matches!(self.lock().state, ResourceState::Unloaded)
    }
    /// Returns true if the resource is still loading.
    pub fn is_loading(&self) -> bool {
        matches!(self.lock().state, ResourceState::Pending { .. })
    }

    /// Returns true if the resource is completely loaded.
    pub fn is_ok(&self) -> bool {
        matches!(self.lock().state, ResourceState::Ok { .. })
    }

    /// Returns true if the resource failed to load.
    pub fn is_failed_to_load(&self) -> bool {
        matches!(self.lock().state, ResourceState::LoadError { .. })
    }

    /// Returns true if the resource is procedural (its data is generated at runtime, not stored in an external
    /// file).
    pub fn is_embedded(&self) -> bool {
        self.lock().kind.is_embedded()
    }

    /// Returns exact amount of users of the resource.
    #[inline]
    pub fn use_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    /// Returns a pointer as numeric value which can be used as a hash.
    #[inline]
    pub fn key(&self) -> u64 {
        let mut hasher = FxHasher64::default();
        self.hash(&mut hasher);
        hasher.finish()
    }

    /// Returns path of the untyped resource.
    pub fn kind(&self) -> ResourceKind {
        self.lock().kind
    }

    /// Set a new path for the untyped resource.
    pub fn set_kind(&self, new_kind: ResourceKind) {
        self.lock().kind = new_kind;
    }

    /// Tries to save the resource to the specified path.
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        match self.lock().state {
            ResourceState::Pending { .. }
            | ResourceState::LoadError { .. }
            | ResourceState::Unloaded => Err("Unable to save unloaded resource!".into()),
            ResourceState::Ok { ref mut data, .. } => data.save(path),
        }
    }

    /// Tries to cast untyped resource to a particular type.
    pub fn try_cast<T>(&self) -> Option<Resource<T>>
    where
        T: TypedResourceData,
    {
        if self.type_uuid() == Some(<T as TypeUuidProvider>::type_uuid()) {
            Some(Resource {
                untyped: self.clone(),
                phantom: PhantomData::<T>,
            })
        } else {
            None
        }
    }

    /// Modify this resource into the [`ResourceState::Pending`] state.
    pub fn make_pending(&mut self) {
        self.lock().state = ResourceState::new_pending();
    }
    /// Changes ResourceState::Pending state to ResourceState::Ok(data) with given `data`.
    /// Additionally, it wakes all futures. Panics if the resource is unrequested.
    #[inline]
    pub fn commit(&self, state: ResourceState) {
        self.lock().state.commit(state);
    }

    /// Changes internal state to [`ResourceState::Ok`]. Panics if the resource is unrequested.
    pub fn commit_ok<T: ResourceData>(&self, data: T) {
        self.lock().state.commit_ok(data);
    }

    /// Changes internal state to [`ResourceState::LoadError`].
    pub fn commit_error<E: ResourceLoadError>(&mut self, path: PathBuf, error: E) {
        self.lock().state.commit_error(path, error);
    }
}

impl Future for UntypedResource {
    type Output = Result<Self, LoadError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut guard = self.lock();
        match guard.state {
            ResourceState::Pending { ref mut wakers, .. } => {
                wakers.add_waker(cx.waker());
                Poll::Pending
            }
            ResourceState::Unloaded => Poll::Ready(Err(LoadError::new(
                "Unloaded resource is not loading".to_string(),
            ))),
            ResourceState::LoadError { ref error, .. } => Poll::Ready(Err(error.clone())),
            ResourceState::Ok { .. } => Poll::Ready(Ok(self.clone())),
        }
    }
}

#[cfg(test)]
mod test {
    use futures::task::noop_waker;
    use fyrox_core::futures;
    use std::error::Error;
    use std::task::{self};

    use crate::io::FsResourceIo;

    use super::*;

    #[derive(Debug, Default, Reflect, Visit, Clone, Copy)]
    struct Stub {}

    impl ResourceData for Stub {
        fn type_uuid(&self) -> Uuid {
            Uuid::default()
        }

        fn save(&mut self, _path: &Path) -> Result<(), Box<dyn Error>> {
            Err("Saving is not supported!".to_string().into())
        }

        fn can_be_saved(&self) -> bool {
            false
        }

        fn try_clone_box(&self) -> Option<Box<dyn ResourceData>> {
            Some(Box::new(*self))
        }
    }

    impl TypeUuidProvider for Stub {
        fn type_uuid() -> Uuid {
            Uuid::default()
        }
    }

    impl ResourceLoadError for str {}

    #[test]
    fn visit_for_untyped_resource() {
        let mut r = UntypedResource::default();
        let mut visitor = Visitor::default();

        assert!(r.visit("name", &mut visitor).is_ok());

        let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
        visitor
            .save_binary_to_memory(&mut cursor)
            .expect("Failed to write binary for visitor");
        cursor.set_position(0);
        let mut visitor = Visitor::load_binary_from_memory(cursor.get_ref())
            .expect("Failed to read binary for visitor");
        visitor.blackboard.register(Arc::new(ResourceManager::new(
            Arc::new(FsResourceIo),
            Arc::default(),
        )));
        assert!(r.visit("name", &mut visitor).is_ok());
        assert!(r.is_embedded());
        assert!(r.is_failed_to_load());
    }

    #[test]
    fn untyped_resource_use_count() {
        let r = UntypedResource::default();

        assert_eq!(r.use_count(), 1);
    }

    #[test]
    fn untyped_resource_try_cast() {
        let r = UntypedResource::default();
        let r2 = UntypedResource::new_ok(Uuid::new_v4(), ResourceKind::External, Stub {});

        assert!(r.try_cast::<Stub>().is_none());
        assert!(r2.try_cast::<Stub>().is_some());
    }

    #[test]
    fn untyped_resource_poll() {
        let stub = Stub {};

        let waker = noop_waker();
        let mut cx = task::Context::from_waker(&waker);

        let mut r = UntypedResource::from(ResourceHeader {
            uuid: Uuid::new_v4(),
            kind: ResourceKind::External,
            state: ResourceState::Ok {
                data: ResourceDataWrapper(Box::new(stub)),
            },
        });
        assert!(Pin::new(&mut r).poll(&mut cx).is_ready());

        let mut r = UntypedResource::from(ResourceHeader {
            uuid: Uuid::new_v4(),
            kind: ResourceKind::External,
            state: ResourceState::LoadError {
                path: Default::default(),
                error: Default::default(),
            },
        });
        assert!(Pin::new(&mut r).poll(&mut cx).is_ready());
    }
}

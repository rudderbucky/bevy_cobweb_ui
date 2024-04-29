use std::any::{type_name, TypeId};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use bevy::prelude::*;
use bevy::utils::warn_once;
use bevy_cobweb::prelude::*;
use smallvec::SmallVec;

use crate::*;

//-------------------------------------------------------------------------------------------------------------------

fn setup_loadablesheet(sheet_list: Res<LoadableSheetList>, mut loadablesheet: ReactResMut<LoadableSheet>)
{
    // begin tracking expected loadablesheet files
    for file in sheet_list.iter_files() {
        loadablesheet
            .get_noreact()
            .prepare_file(LoadableFile::new(file.as_str()));
    }
}

//-------------------------------------------------------------------------------------------------------------------

fn load_loadable_changes(
    mut c: Commands,
    mut events: EventReader<AssetEvent<LoadableSheetAsset>>,
    sheet_list: Res<LoadableSheetList>,
    mut assets: ResMut<Assets<LoadableSheetAsset>>,
    mut loadablesheet: ReactResMut<LoadableSheet>,
    types: Res<AppTypeRegistry>,
)
{
    if events.is_empty() {
        return;
    }

    let type_registry = types.read();
    let mut need_reactions = false;

    for event in events.read() {
        let id = match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => id,
            _ => {
                tracing::debug!("ignoring loadablesheet asset event {:?}", event);
                continue;
            }
        };

        let Some(handle) = sheet_list.get_handle(*id) else {
            tracing::warn!("encountered loadablesheet asset event {:?} for an untracked asset", id);
            continue;
        };

        let Some(asset) = assets.remove(handle) else {
            tracing::error!("failed to remove loadablesheet asset {:?}", handle);
            continue;
        };

        let loadablesheet = loadablesheet.get_noreact();
        parse_loadablesheet_file(&type_registry, loadablesheet, asset.file, asset.data);
        need_reactions = true;
    }

    if need_reactions {
        loadablesheet.get_mut(&mut c);
    }
}

//-------------------------------------------------------------------------------------------------------------------

fn cleanup_loadablesheet(
    mut loadablesheet: ReactResMut<LoadableSheet>,
    mut removed: RemovedComponents<HasLoadables>,
)
{
    for removed in removed.read() {
        loadablesheet.get_noreact().remove_entity(removed);
    }

    loadablesheet.get_noreact().cleanup_pending();
}

//-------------------------------------------------------------------------------------------------------------------

struct ErasedLoadable
{
    type_id: TypeId,
    loadable: ReflectedLoadable,
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Copy, Clone)]
struct RefSubscription
{
    entity: Entity,
    setter: ContextSetter,
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Clone)]
pub(crate) enum ReflectedLoadable
{
    Value(Arc<Box<dyn Reflect + 'static>>),
    DeserializationFailed(Arc<serde_json::Error>),
}

impl ReflectedLoadable
{
    pub(crate) fn equals(&self, other: &ReflectedLoadable) -> Option<bool>
    {
        let (Self::Value(this), Self::Value(other)) = (self, other) else {
            return Some(false);
        };

        this.reflect_partial_eq(other.as_reflect())
    }

    pub(crate) fn get_value<T: Loadable>(&self, loadable_ref: &LoadableRef) -> Option<T>
    {
        match self {
            ReflectedLoadable::Value(loadable) => {
                let Some(new_value) = T::from_reflect(loadable.as_reflect()) else {
                    let temp = T::default();
                    let mut hint = serde_json::to_string_pretty(&temp).unwrap();
                    if hint.len() > 250 {
                        hint = serde_json::to_string(&temp).unwrap();
                    }
                    tracing::error!("failed reflecting loadable {:?} at path {:?} in file {:?}\n\
                        serialization hint: {}",
                        type_name::<T>(), loadable_ref.path.path, loadable_ref.file, hint.as_str());
                    return None;
                };
                Some(new_value)
            }
            ReflectedLoadable::DeserializationFailed(err) => {
                let temp = T::default();
                let mut hint = serde_json::to_string_pretty(&temp).unwrap();
                if hint.len() > 250 {
                    hint = serde_json::to_string(&temp).unwrap();
                }
                tracing::error!("failed deserializing loadable {:?} at path {:?} in file {:?}, {:?}\n\
                    serialization hint: {}",
                    type_name::<T>(), loadable_ref.path.path, loadable_ref.file, **err, hint.as_str());
                None
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Reactive resource for managing loadables loaded from loadablesheet assets.
///
/**
### Loadablesheet asset format

Loadablesheets are written as JSON files with the extension `.loadable.json`. You must register loadablesheets in your app with
[`LoadableSheetListAppExt::add_load_sheet`].

The loadablesheet format has a short list of rules.

- Each file must have one map at the base layer.
```json
{

}
```
- If the first map entry's key is `"using"`, then the value should be an array of full type names. This array
    should contain full type names for any [`Loadable`] that has an ambiguous short name (this will happen if there are
    multiple `Reflect` types with the same short name). Note that currently we only support one version of a shortname
    per file.
```json
{
    "using": [
        "crate::my_module::Color",
        "bevy_cobweb_ui::layout::Layout"
    ]
}
```
- All other map keys may either be [`Loadable`] short type names or node path references.
    A loadable short name is a marker for a loadable, and is followed by a map containing the serialized value of that loadable.
    Node path references are used to locate specific loadables in the overall structure, and each node should be a map of loadables and
    other nodes. The leaf nodes of the overall structure will be loadables.
```json
{
    "using": [ "bevy_cobweb_ui::layout::Dims" ],

    "node1": {
        "Dims": {"Percent": [50.0, 50.0]},

        "node2": {
            "Dims": {"Percent": [50.0, 50.0]}
        }
    }
}
```
- A loadable name may be followed by the keyword `"inherited"`, which means the loadable value will be inherited from the most
    recent instance of that loadable below it in the tree. Inheritance is ordering-dependent, so if you don't want a loadable
    to be inherited, insert it below any child nodes.
```json
{
    "using": [ "bevy_cobweb_ui::layout::Dims" ],

    "node1": {
        "Dims": {"Percent": [50.0, 50.0]},

        "node2": {
            "Dims": "inherited"
        }
    }
}
```
- Node path references may be combined into path segments, which can be used to reduce indentation. If a loadable is inherited
    in an abbreviated path, it will inherit from the current scope, not its path-parent.
```json
{
    "using": [ "bevy_cobweb_ui::layout::Dims" ],

    "Dims": {"Percent": [25.0, 25.0]},

    "node1": {
        "Dims": {"Percent": [50.0, 50.0]}
    },

    "node1::node2": {
        // This inherits {25.0, 25.0}.
        "Dims": "inherited"
    }
}
```
*/
//TODO: add "MY_CONSTANT_X" references with "constants" section
//TODO: add "imports" section that brings "using" and "constants" sections from other files (track dependencies in
// LoadableSheet)
// - warn if there are unresolved dependencies after all initial files have been loaded and handled
#[derive(ReactResource)]
pub struct LoadableSheet
{
    /// Tracks loadables in all loadable files.
    loadables: HashMap<LoadableRef, SmallVec<[ErasedLoadable; 4]>>,
    /// Tracks which files have not initialized yet.
    pending: HashSet<LoadableFile>,
    /// Tracks the total number of loadable sheets that should load.
    ///
    /// Used for progress tracking on initial load.
    total_expected_sheets: usize,

    /// Tracks subscriptions to loadable paths.
    subscriptions: HashMap<LoadableRef, SmallVec<[RefSubscription; 1]>>,
    /// Tracks entities for cleanup.
    subscriptions_rev: HashMap<Entity, LoadableRef>,

    /// Records entities that need loadable updates.
    /// - We clear this at the end of every tick, so there should not be stale `ReflectedLoadable` values.
    needs_updates:
        HashMap<TypeId, SmallVec<[(ReflectedLoadable, LoadableRef, SmallVec<[RefSubscription; 1]>); 1]>>,
}

impl LoadableSheet
{
    /// Prepares a loadablesheet file.
    fn prepare_file(&mut self, file: LoadableFile)
    {
        let _ = self.pending.insert(file.clone());
        self.total_expected_sheets += 1;
    }

    /// Initializes a loadablesheet file.
    pub(crate) fn initialize_file(&mut self, file: LoadableFile)
    {
        let _ = self.pending.remove(&file);
    }

    /// Gets the loadablesheet's loading progress on startup.
    ///
    /// Returns `(num uninitialized files, num total files)`.
    pub fn loading_progress(&self) -> (usize, usize)
    {
        (self.pending.len(), self.total_expected_sheets)
    }

    /// Inserts a loadable at the specified path if its value will change.
    ///
    /// Returns `true` if this method added any pending subscriber updates.
    pub(crate) fn insert(
        &mut self,
        loadable_ref: &LoadableRef,
        loadable: ReflectedLoadable,
        type_id: TypeId,
        full_type_name: &str,
    ) -> bool
    {
        match self.loadables.entry(loadable_ref.clone()) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                let mut vec = SmallVec::default();
                vec.push(ErasedLoadable { type_id, loadable: loadable.clone() });
                entry.insert(vec);
            }
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                // Insert if the loadable value changed.
                if let Some(erased_loadable) = entry.get_mut().iter_mut().find(|e| e.type_id == type_id) {
                    match erased_loadable.loadable.equals(&loadable) {
                        Some(true) => return false,
                        Some(false) => {
                            // Replace the existing value.
                            *erased_loadable = ErasedLoadable { type_id, loadable: loadable.clone() };
                        }
                        None => {
                            tracing::error!("failed updating loadable {:?} at {:?}, its reflected value doesn't implement \
                                PartialEq", full_type_name, loadable_ref);
                            return false;
                        }
                    }
                } else {
                    entry
                        .get_mut()
                        .push(ErasedLoadable { type_id, loadable: loadable.clone() });
                }
            }
        }

        // Identify entites that should update.
        let Some(subscriptions) = self.subscriptions.get(&loadable_ref) else { return false };
        if subscriptions.len() == 0 {
            return false;
        }
        let entry = self.needs_updates.entry(type_id).or_default();
        entry.push((loadable, loadable_ref.clone(), subscriptions.clone()));

        true
    }

    /// Adds an entity to the tracking context.
    ///
    /// Schedules callbacks that will run to handle pending updates for the entity.
    pub(crate) fn track_entity(
        &mut self,
        entity: Entity,
        loadable_ref: LoadableRef,
        setter: ContextSetter,
        c: &mut Commands,
        callbacks: &LoaderCallbacks,
    )
    {
        // Add to subscriptions.
        let subscription = RefSubscription { entity, setter };
        self.subscriptions
            .entry(loadable_ref.clone())
            .or_default()
            .push(subscription);
        self.subscriptions_rev.insert(entity, loadable_ref.clone());

        // Get already-loaded loadables that the entity is subscribed to.
        let Some(loadables) = self.loadables.get(&loadable_ref) else { return };

        // Schedule updates for each loadable.
        for loadable in loadables.iter() {
            let type_id = loadable.type_id;
            self.needs_updates.entry(type_id).or_default().push((
                loadable.loadable.clone(),
                loadable_ref.clone(),
                SmallVec::from_elem(subscription, 1),
            ));

            let Some(syscommand) = callbacks.get(type_id) else {
                tracing::warn!("found loadable at {:?} that wasn't registered as a loadable loadable", loadable_ref);
                continue;
            };

            c.add(syscommand);
        }

        // Notify the entity that some of its loadables have loaded.
        if loadables.len() > 0 {
            c.react().entity_event(entity, Loaded);
        }
    }

    /// Cleans up despawned entities.
    fn remove_entity(&mut self, dead_entity: Entity)
    {
        let Some(loadable_ref) = self.subscriptions_rev.remove(&dead_entity) else { return };
        let Some(subscribed) = self.subscriptions.get_mut(&loadable_ref) else { return };
        let Some(dead) = subscribed.iter().position(|s| s.entity == dead_entity) else { return };
        subscribed.swap_remove(dead);
    }

    /// Cleans up pending updates that failed to be processed.
    fn cleanup_pending(&mut self)
    {
        if self.needs_updates.len() > 0 {
            // Note: This can technically print spuriously if the user spawns loaded entities in Last and doesn't
            // call `apply_deferred` before the cleanup system runs.
            warn_once!("The loadable sheet contains pending updates for types that weren't registered. This warning only \
                prints once, and may print spuriously if you spawn loaded entities in Last.");
        }
        self.needs_updates.clear();
    }

    /// Updates entities that subscribed to `T` found at recently-updated loadable paths.
    pub(crate) fn update_loadables<T: Loadable>(
        &mut self,
        mut callback: impl FnMut(Entity, ContextSetter, &LoadableRef, &ReflectedLoadable),
    )
    {
        let Some(mut needs_updates) = self.needs_updates.remove(&TypeId::of::<T>()) else { return };

        for (loadable, loadable_ref, mut subscriptions) in needs_updates.drain(..) {
            for subscription in subscriptions.drain(..) {
                (callback)(subscription.entity, subscription.setter, &loadable_ref, &loadable);
            }
        }
    }
}

impl Default for LoadableSheet
{
    fn default() -> Self
    {
        Self {
            loadables: HashMap::default(),
            pending: HashSet::default(),
            total_expected_sheets: 0,
            subscriptions: HashMap::default(),
            subscriptions_rev: HashMap::default(),
            needs_updates: HashMap::default(),
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Plugin that enables loading.
pub(crate) struct LoadableSheetPlugin;

impl Plugin for LoadableSheetPlugin
{
    fn build(&self, app: &mut App)
    {
        app.init_react_resource::<LoadableSheet>()
            .add_systems(PreStartup, setup_loadablesheet)
            .add_systems(First, load_loadable_changes)
            .add_systems(Last, cleanup_loadablesheet);
    }
}

//-------------------------------------------------------------------------------------------------------------------

use std::any::TypeId;
use std::collections::HashMap;
use std::marker::PhantomData;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy_cobweb::prelude::*;

use crate::*;

//-------------------------------------------------------------------------------------------------------------------

fn register_loadable_impl<M, T: 'static>(
    app: &mut App,
    callback: impl IntoSystem<(), (), M> + Send + Sync + 'static + Copy,
    _p: PhantomData<T>,
    register_type: &'static str,
)
{
    if !app.world.contains_resource::<LoaderCallbacks>() {
        app.init_resource::<LoaderCallbacks>();
    }

    CallbackSystem::new(
        move |mut c: Commands, mut loaders: ResMut<LoaderCallbacks>|
        {
            let entry = loaders.callbacks.entry(TypeId::of::<T>());
            if matches!(entry, std::collections::hash_map::Entry::Occupied(_))
            {
                tracing::warn!("tried registering {register_type} loadable {} multiple times", std::any::type_name::<T>());
            }

            entry.or_insert_with(
                    || c.react().on_persistent(resource_mutation::<LoadableSheet>(), callback)
                );
        }
    ).run(&mut app.world, ());
}

//-------------------------------------------------------------------------------------------------------------------

/// Updates the loadable `React<T>` on entities.
fn reactive_loader<T: ReactComponent + Loadable>(
    mut c: Commands,
    mut loadables: ReactResMut<LoadableSheet>,
    mut entities: Query<Option<&mut React<T>>>,
)
{
    loadables.get_noreact().update_loadables::<T>(|entity, loadable_ref, loadable| {
        let Ok(component) = entities.get_mut(entity) else { return };
        let Some(new_val) = loadable.get_value(loadable_ref) else { return };

        match component {
            Some(mut component) => {
                *component.get_mut(&mut c) = new_val;
            }
            None => {
                c.react().insert(entity, new_val);
            }
        }

        c.react().entity_event::<Loaded>(entity, Loaded);
    });
}

//-------------------------------------------------------------------------------------------------------------------

/// Updates the loadable bundle `T` on entities.
fn bundle_loader<T: Bundle + Loadable>(mut c: Commands, mut loadables: ReactResMut<LoadableSheet>)
{
    loadables.get_noreact().update_loadables::<T>(|entity, loadable_ref, loadable| {
        let Some(bundle) = loadable.get_value::<T>(loadable_ref) else { return };
        let Some(mut ec) = c.get_entity(entity) else { return };
        ec.try_insert(bundle);

        c.react().entity_event::<Loaded>(entity, Loaded);
    });
}

//-------------------------------------------------------------------------------------------------------------------

/// Uses `T` to derive changes on subscribed entities.
fn derived_loader<T: ApplyLoadable + Loadable>(mut c: Commands, mut loadables: ReactResMut<LoadableSheet>)
{
    loadables.get_noreact().update_loadables::<T>(|entity, loadable_ref, loadable| {
        let Some(value) = loadable.get_value::<T>(loadable_ref) else { return };
        let Some(mut ec) = c.get_entity(entity) else { return };
        value.apply(&mut ec);

        c.react().entity_event::<Loaded>(entity, Loaded);
    });
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource)]
pub(crate) struct LoaderCallbacks
{
    callbacks: HashMap<TypeId, SystemCommand>,
}

impl LoaderCallbacks
{
    pub(crate) fn get(&self, type_id: TypeId) -> Option<SystemCommand>
    {
        self.callbacks.get(&type_id).cloned()
    }
}

impl Default for LoaderCallbacks
{
    fn default() -> Self
    {
        Self { callbacks: HashMap::default() }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Helper trait for registering entities for loadable loading.
pub trait StyleLoadingEntityCommandsExt
{
    /// Registers the current entity to load loadables from `loadable_ref`.
    fn load(&mut self, loadable_ref: LoadableRef) -> &mut Self;
}

impl StyleLoadingEntityCommandsExt for EntityCommands<'_>
{
    fn load(&mut self, loadable_ref: LoadableRef) -> &mut Self
    {
        self.insert(HasLoadables);

        let id = self.id();
        self.commands().syscall(
            (id, loadable_ref),
            |In((id, loadable_ref)): In<(Entity, LoadableRef)>,
             mut c: Commands,
             loaders: Res<LoaderCallbacks>,
             mut loadablesheet: ReactResMut<LoadableSheet>| {
                loadablesheet.get_noreact().track_entity(id, loadable_ref, &mut c, &loaders);
            },
        );

        self
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Extends `App` with methods supporting [`LoadableSheet`] use.
pub trait LoadableRegistrationAppExt
{
    /// Registers a loadable type that will be inserted as [`T`] bundles on entities that subscribe to
    /// loadablesheet paths containing the type.
    fn register_loadable<T: Bundle + Loadable>(&mut self) -> &mut Self;

    /// Registers a loadable type that will be inserted as [`React<T>`] components on entities that subscribe to
    /// loadablesheet paths containing the type.
    fn register_reactive_loadable<T: ReactComponent + Loadable>(&mut self) -> &mut Self;

    /// Registers a loadable type that will be inserted as [`T`] bundles on entities that subscribe to
    /// loadablesheet paths containing the type.
    fn register_derived_loadable<T: ApplyLoadable + Loadable>(&mut self) -> &mut Self;
}

impl LoadableRegistrationAppExt for App
{
    fn register_loadable<T: Bundle + Loadable>(&mut self) -> &mut Self
    {
        register_loadable_impl(self, bundle_loader::<T>, PhantomData::<T>::default(), "bundle");
        self
    }

    fn register_reactive_loadable<T: ReactComponent + Loadable>(&mut self) -> &mut Self
    {
        register_loadable_impl(
            self,
            reactive_loader::<T>,
            PhantomData::<T>::default(),
            "reactive",
        );
        self
    }

    fn register_derived_loadable<T: ApplyLoadable + Loadable>(&mut self) -> &mut Self
    {
        register_loadable_impl(self, derived_loader::<T>, PhantomData::<T>::default(), "derived");
        self
    }
}

//-------------------------------------------------------------------------------------------------------------------

pub(crate) struct LoaderPlugin;

impl Plugin for LoaderPlugin
{
    fn build(&self, app: &mut App)
    {
        app.init_resource::<LoaderCallbacks>();
    }
}

//-------------------------------------------------------------------------------------------------------------------

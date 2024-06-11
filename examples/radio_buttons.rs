//! A simple radio button widget.

use bevy::prelude::*;
use bevy::window::WindowTheme;
use bevy_cobweb::prelude::*;
use bevy_cobweb_ui::prelude::*;
use sickle::theme::pseudo_state::{PseudoState, PseudoStates};
use sickle::theme::{ComponentThemePlugin, DefaultTheme, UiContext};
use sickle::ui_builder::*;
use sickle::widgets::container::UiContainerExt;
use sickle::{DefaultTheme, SickleUiPlugin, UiContext};

//-------------------------------------------------------------------------------------------------------------------

#[derive(Component)]
struct RadioButtonManager
{
    selected: Option<Entity>,
}

impl RadioButtonManager
{
    fn new() -> Self
    {
        Self { selected: None }
    }

    /// Inserts the manager onto the builder entity.
    ///
    /// Returns the entity where the manager is stored.
    fn insert(self, node: &mut UiBuilder<Entity>) -> Entity
    {
        node.insert(self);
        node.id()
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Marker component for the radio button theme.
#[derive(Component, DefaultTheme, UiContext, Copy, Clone, Debug)]
struct RadioButton;

//-------------------------------------------------------------------------------------------------------------------

struct RadioButtonBuilder
{
    text: String,
}

impl RadioButtonBuilder
{
    fn new(text: impl Into<String>) -> Self
    {
        Self { text: text.into() }
    }

    /// Builds the button as a child of the builder entity.
    ///
    /// The `manager_entity` should have a [`RadioButtonManager`] component.
    fn build<'w, 's, 'a>(
        self,
        manager_entity: Entity,
        node: &'a mut UiBuilder<'w, 's, '_, Entity>,
    ) -> UiBuilder<'w, 's, 'a, Entity>
    {
        let file = LoadableRef::from_file("widgets.radio_button");

        let mut core_entity = Entity::PLACEHOLDER;
        node.load_theme::<RadioButton>(file.e("core"), file.e("core"), |core, path| {
            core_entity = core.id();
            core.insert(RadioButton)
                .insert(PropagateControl)
                .entity_commands()
                // Note: this callback could be moved to an EntityWorldReactor, with the manager entity as entity
                // data.
                .on_pressed(
                    // Select this button.
                    move |mut c: Commands, states: Query<&PseudoStates>| {
                        if let Ok(states) = states.get(core_entity) {
                            if states.has(&PseudoState::Selected) {
                                return;
                            }
                        }

                        c.react().entity_event(core_entity, Select);
                    },
                )
                .on_select(
                    // Save the newly-selected button and deselect the previously selected.
                    move |mut c: Commands, mut managers: Query<&mut RadioButtonManager>| {
                        let Ok(mut manager) = managers.get_mut(manager_entity) else { return };
                        if let Some(prev) = manager.selected {
                            c.react().entity_event(prev, Deselect);
                        }
                        manager.selected = Some(core_entity);
                    },
                );

            core.load(path.e("outline"), |outline, path| {
                outline.load(path.e("indicator"), |_, _| {});
            });

            core.load(path.e("text"), |text, _| {
                // Note: The text needs to be updated on load otherwise it may be overwritten.
                let text_val = self.text;
                text.update_on(entity_event::<Loaded>(text.id()), |id| {
                    move |mut e: TextEditor| {
                        e.write(id, |t| write!(t, "{}", text_val.as_str()));
                    }
                });
            });
        });

        // Return UiBuilder for root of button where interactions will be detected.
        node.commands().ui_builder(core_entity)
    }
}

//-------------------------------------------------------------------------------------------------------------------

fn build_ui(mut c: Commands)
{
    let file = LoadableRef::from_file("examples.radio_buttons");
    static OPTIONS: [&'static str; 3] = ["A", "B", "C"];

    c.ui_builder(UiRoot).load(file.e("root"), |root, path| {
        // Display the selected option.
        let mut display_text = Entity::PLACEHOLDER;
        root.load(path.e("display"), |display, path| {
            display.load(path.e("text"), |text, _| {
                display_text = text.id();
            });
        });

        // Insert radio buttons.
        root.load(path.e("radio_frame"), |frame, _| {
            let manager_entity = RadioButtonManager::new().insert(frame);

            for (i, option) in OPTIONS.iter().enumerate() {
                // Add radio button.
                let button_entity = RadioButtonBuilder::new(*option)
                    .build(manager_entity, frame)
                    .entity_commands()
                    .on_select(move |mut e: TextEditor| {
                        e.write(display_text, |t| write!(t, "Selected: {}", option));
                    })
                    .id();

                // Select the first option.
                if i == 0 {
                    frame.react().entity_event(button_entity, Select);
                }
            }
        });
    });
}

//-------------------------------------------------------------------------------------------------------------------

fn init_loading_display(mut c: Commands)
{
    c.ui_builder(UiRoot)
        .container(NodeBundle::default(), |node| {
            node.insert_reactive(FlexStyle::default())
                .insert_derived(Width(Val::Vw(100.)))
                .insert_derived(Height(Val::Vh(100.)))
                .insert_derived(SetFlexDirection(FlexDirection::Column))
                .insert_derived(SetJustifyMain(JustifyMain::Center))
                .insert_derived(SetJustifyCross(JustifyCross::Center))
                .despawn_on_broadcast::<StartupLoadingDone>();

            node.container(NodeBundle::default(), |node| {
                node.insert_derived(TextLine { text: "Loading...".into(), font: None, size: 75.0 });
            });
        });
}

//-------------------------------------------------------------------------------------------------------------------

fn setup(mut c: Commands)
{
    c.spawn(Camera2dBundle {
        transform: Transform { translation: Vec3 { x: 0., y: 0., z: 1000. }, ..default() },
        ..default()
    });
}

//-------------------------------------------------------------------------------------------------------------------

fn main()
{
    App::new()
        .add_plugins(bevy::DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { window_theme: Some(WindowTheme::Dark), ..Default::default() }),
            ..Default::default()
        }))
        .add_plugins(SickleUiPlugin)
        .add_plugins(CobwebUiPlugin)
        .add_plugins(ComponentThemePlugin::<RadioButton>::new())
        .load_sheet("examples/radio_buttons.load.json")
        .add_systems(PreStartup, setup)
        .add_systems(OnEnter(LoadProgress::Loading), init_loading_display)
        .add_systems(OnEnter(LoadProgress::Done), build_ui)
        .run();
}

//-------------------------------------------------------------------------------------------------------------------

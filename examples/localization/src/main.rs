//! Demonstrates localization of text, fonts, and images.

use bevy::prelude::*;
use bevy::window::WindowTheme;
use bevy_cobweb::prelude::*;
use bevy_cobweb_ui::prelude::*;

//-------------------------------------------------------------------------------------------------------------------

fn build_ui(mut c: Commands, mut s: ResMut<SceneLoader>)
{
    let scene = ("localization", "root");
    c.ui_root().load_scene_and_edit(scene, &mut s, |l| {
        // Header
        // - Localized image from file (see `assets/main.cob`).

        // Content
        l.edit("content", |l| {
            // Language selection list.
            l.get("selection_section::selection_box")
                // Update the selection whenever the manifest is updated with a new base language list.
                .update_on(
                    broadcast::<LocalizationManifestUpdated>(),
                    move |//
                        id: UpdateId,
                        mut c: Commands,
                        mut s: ResMut<SceneLoader>,
                        manifest: Res<LocalizationManifest>//
                    | {
                        // Despawn existing buttons.
                        c.entity(*id).despawn_descendants();

                        // Spawn new buttons for everything in the manifest.
                        let mut n = c.ui_builder(*id);
                        //let manager_entity = RadioButtonManager::insert(&mut n);
                        let current_lang = &manifest.negotiated()[0];
                        let button_scene = SceneRef::new("localization", "selection_button");

                        for language in manifest.languages() {
                            let name = language.display_name();
                            let lang = language.id.clone();

                            n.load_scene_and_edit(button_scene.clone(), &mut s, |l| {
                                l.on_select(move |mut locale: ResMut<Locale>| {
                                    *locale = Locale::new_from_id(lang.clone());
                                });

                                l.get("text").update_text(name);

                                // Select the current locale.
                                if language.id == *current_lang {
                                    let button_id = l.id();
                                    l.react().entity_event(button_id, Select);
                                }
                            });
                        }
                    },
                );

            l.edit("text_section", |l| {
                // Unlocalized text.
                l.get("unlocalized")
                    .update_text("This text is not localized.");

                // Untranslated text (only localized in the default language).
                l.get("untranslated")
                    .insert(LocalizedText::default())
                    .update_text("untranslated");

                // Localized and partly translated text (localized in only some, but not all, alternate
                // languages).
                l.get("partially_translated")
                    .insert(LocalizedText::default())
                    .update_text("partly-translated");

                // Localized and fully translated text.
                l.get("fully_translated")
                    .insert(LocalizedText::default())
                    .update_text("fully-translated");

                // Localized text with different font fallbacks for different languages.
                l.get("font_fallbacks")
                    .insert(LocalizedText::default())
                    .apply(TextLine::from_text("font-fallbacks").with_font(FontFamily::new("Fira Sans").bold()));

                // Localized dynamic text.
                l.get("dynamic").insert(LocalizedText::default()).update_on(
                    broadcast::<RelocalizeApp>(),
                    move |id: UpdateId, mut count: Local<usize>, mut e: TextEditor| {
                        // Displays count for the number of times the app was localized.
                        write_text!(e, *id, "locale-counter?count={:?}", *count);
                        *count += 1;
                    },
                );

                // Localized text from file (see `assets/main.cob`).
            });
        });
    });
}

//-------------------------------------------------------------------------------------------------------------------

fn setup(mut c: Commands)
{
    c.spawn(Camera2d);
}

//-------------------------------------------------------------------------------------------------------------------

fn main()
{
    App::new()
        .add_plugins(bevy::DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { window_theme: Some(WindowTheme::Dark), ..default() }),
            ..default()
        }))
        .add_plugins(CobwebUiPlugin)
        .load("main.cob")
        .add_systems(PreStartup, setup)
        .add_systems(OnEnter(LoadState::Done), build_ui)
        .run();
}

//-------------------------------------------------------------------------------------------------------------------

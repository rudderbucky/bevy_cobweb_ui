Instructions for adding a new embedded widget:

- Add a `WIDGET_NAME.cob` file to the widget directory.
- Add `"#manifest": { "", "builtin.widgets.WIDGET_NAME" }` to the `WIDGET_NAME.cob` file.
- Add `load_embedded_scene_file!(app, "bevy_cobweb_ui", "src/builtin/widgets/WIDGET_NAME", "WIDGET_NAME.cob");` to the plugin.
- Add a docs entry to `src/widgets/mod.rs` for the new widget.

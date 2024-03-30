#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use std::{
    f32::consts::PI,
    time::{Duration, Instant},
};

use bevy::{
    app::AppExit,
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
    input::mouse::MouseWheel,
    prelude::*,
    render::{
        render_resource::WgpuFeatures,
        settings::{RenderCreation, WgpuSettings},
        RenderPlugin,
    },
    transform,
    window::{CompositeAlphaMode, Cursor, WindowLevel, WindowMode},
};

use bevy_egui::{
    egui::{self, Checkbox},
    EguiContexts, EguiPlugin,
};
use bevy_infinite_grid::InfiniteGridPlugin;
use bevy_mod_picking::prelude::*;
use picking_ext::{PickingExtPlugin, PointerEvent};
use rand::Rng;
use references::{LineArtGizmo, ReferencePlugin, References};
use wrapping_cursor::{Wrap, WrappingCursorPlugin, WrappingCursorState};

mod outline;
mod picking_ext;
mod references;
mod wrapping_cursor;

fn main() {
    // enable wireframe rendering
    let mut wgpu_settings = WgpuSettings::default();
    wgpu_settings.features |= WgpuFeatures::POLYGON_MODE_LINE;

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(wgpu_settings),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Shapes".to_string(),
                        // composite_alpha_mode: CompositeAlphaMode::PostMultiplied,
                        position: WindowPosition::Centered(MonitorSelection::Index(2)),
                        // transparent: true,
                        // cursor: Cursor {
                        //     hit_test: false,
                        //     ..default()
                        // },
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::rgba(0.1, 0.1, 0.1, 0.)))
        .add_plugins((
            EguiPlugin,
            FrameTimeDiagnosticsPlugin,
            EntityCountDiagnosticsPlugin,
            DefaultPickingPlugins::build(DefaultPickingPlugins)
                .disable::<DefaultHighlightingPlugin>(),
            InfiniteGridPlugin,
        ))
        .add_plugins((ReferencePlugin, PickingExtPlugin, WrappingCursorPlugin))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                zoom,
                ui_active_references,
                close_on_esc,
                // change_transparency_mode,
            ),
        )
        .run();
}

fn change_transparency_mode(
    mut window_query: Query<&mut Window>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let mut window = window_query.single_mut();

    if keyboard_input.just_pressed(KeyCode::KeyF) {
        window.window_level = match window.window_level {
            WindowLevel::Normal => WindowLevel::AlwaysOnTop,
            WindowLevel::AlwaysOnTop => WindowLevel::Normal,
            _ => WindowLevel::Normal,
        };
        window.mode = WindowMode::BorderlessFullscreen;
    }

    if keyboard_input.just_pressed(KeyCode::KeyD) {
        window.cursor.hit_test = !window.cursor.hit_test;
    }
}

fn close_on_esc(mut keyboard_input: ResMut<ButtonInput<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        exit.send(AppExit);
    }
}

fn ui_active_references(mut contexts: EguiContexts, mut refs: ResMut<References>) {
    egui::Window::new("References").show(contexts.ctx_mut(), |ui| {
        let references = refs.references.clone();
        for (i, reference) in references.iter().enumerate() {
            ui.horizontal(|ui| {
                let mut current = Some(i) == refs.current_reference;
                let before = current;
                ui.add(Checkbox::without_text(&mut current));
                if current != before && !before {
                    refs.set_current(i);
                }
                // button to disable reference
                let mut active = !refs.disabled_references.contains(&i);
                ui.checkbox(&mut active, reference.name.as_str());
                refs.set_active(i, active);
            });
        }
    });
}

#[derive(Component)]
pub struct MainCamera;

/// set up a simple 3D scene
fn setup(mut commands: Commands) {
    // light
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_xyz(20.0, 40.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        directional_light: DirectionalLight {
            illuminance: 3000.,
            shadows_enabled: false,
            ..default()
        },
        ..Default::default()
    });
    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        },
        MainCamera,
    ));
}

fn zoom(mut input: EventReader<MouseWheel>, mut camera: Query<&mut Transform, With<MainCamera>>) {
    for event in input.read() {
        for mut transform in camera.iter_mut() {
            transform.translation.z -= event.y * 0.5;
        }
    }
}

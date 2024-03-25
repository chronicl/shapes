#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use std::{
    f32::consts::PI,
    time::{Duration, Instant},
};

use bevy::{
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
    input::mouse::MouseWheel,
    prelude::*,
    render::{
        render_resource::WgpuFeatures,
        settings::{RenderCreation, WgpuSettings},
        RenderPlugin,
    },
    transform,
};
use bevy_editor_pls::{
    default_windows::{
        cameras::camera_3d_panorbit::{ModifierAndMouseButton, PanOrbitCamera},
        hierarchy::HierarchyWindow,
    },
    editor::Editor,
    prelude::*,
};
use bevy_infinite_grid::InfiniteGridPlugin;
use bevy_inspector_egui::{
    bevy_egui::{egui, EguiContexts},
    bevy_inspector::hierarchy::SelectionMode,
};
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
        .add_plugins(DefaultPlugins.set(RenderPlugin {
            render_creation: RenderCreation::Automatic(wgpu_settings),
            ..default()
        }))
        .add_plugins((
            // EditorPlugin::new(),
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
                auto_add_picking_to_meshes,
                deselect_all_on_esc,
                zoom,
                ui_example_system,
            ),
        )
        .run();
}

fn ui_example_system(mut contexts: EguiContexts) {
    egui::Window::new("Hello").show(contexts.ctx_mut(), |ui| {
        ui.label("world");
    });
}

#[derive(Component)]
pub struct MainCamera;

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut editor_camera: Query<&mut PanOrbitCamera>,
    mut clear_color: ResMut<ClearColor>,
) {
    clear_color.0 = Color::rgb(0.1, 0.1, 0.1);

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

    let mut editor_controls = editor_camera.single_mut();
    editor_controls.pan_button = ModifierAndMouseButton {
        modifier: Some(KeyCode::Space),
        mouse_button: MouseButton::Left,
    };
    editor_controls.orbit_button = ModifierAndMouseButton {
        modifier: Some(KeyCode::AltLeft),
        mouse_button: MouseButton::Left,
    };
}

fn auto_add_picking_to_meshes(
    mut commands: Commands,
    new_meshes: Query<Entity, Added<Handle<Mesh>>>,
) {
    fn pick(
        event: Listener<Pointer<Click>>,
        mut editor: ResMut<Editor>,
        input: Res<ButtonInput<KeyCode>>,
    ) {
        if event.button != PointerButton::Primary {
            return;
        }

        let entity = event.target();
        let state = editor.window_state_mut::<HierarchyWindow>().unwrap();
        // Simulating blender here. In blender's object mode or edit mode Shift works as Ctrl would in the file explorer.
        let selection_mode =
            SelectionMode::from_ctrl_shift(input.pressed(KeyCode::ShiftLeft), false);
        state
            .selected
            .select(selection_mode, entity, |_, _| std::iter::once(entity));
    }

    for entity in new_meshes.iter() {
        commands
            .entity(entity)
            .insert((PickableBundle::default(), On::<Pointer<Click>>::run(pick)));
    }
}

fn deselect_all_on_esc(input: Res<ButtonInput<KeyCode>>, mut editor: ResMut<Editor>) {
    if input.just_pressed(KeyCode::Escape) {
        let state = editor.window_state_mut::<HierarchyWindow>().unwrap();
        state.selected.clear();
    }
}

fn zoom(mut input: EventReader<MouseWheel>, mut camera: Query<&mut Transform, With<MainCamera>>) {
    for event in input.read() {
        for mut transform in camera.iter_mut() {
            transform.translation.z -= event.y * 0.5;
        }
    }
}

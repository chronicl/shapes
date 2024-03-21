use std::{
    f32::consts::PI,
    time::{Duration, Instant},
};

use bevy::{
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
    ecs::{entity, event},
    input::keyboard::KeyboardInput,
    prelude::*,
    render::{
        render_resource::WgpuFeatures,
        settings::{RenderCreation, WgpuSettings},
        RenderPlugin,
    },
};
use bevy_editor_pls::{
    default_windows::{
        cameras::{camera_3d_panorbit::ModifierAndMouseButton, EditorCamera},
        hierarchy::HierarchyWindow,
    },
    editor::Editor,
    prelude::*,
};
use bevy_infinite_grid::{InfiniteGridBundle, InfiniteGridPlugin, InfiniteGridSettings};
use bevy_inspector_egui::bevy_inspector::hierarchy::SelectionMode;
use bevy_mod_picking::{
    backend::PointerHits,
    pointer::{InputPress, PressDirection},
    prelude::*,
};
use picking_ext::{PickingExtPlugin, PointerEvent};
use wrapping_cursor::{Wrap, WrappingCursor, WrappingCursorPlugin, WrappingCursorState};

mod picking_ext;
mod wrapping_cursor;

const TIMER_INTERVAL: f32 = 3.0;
const DOUBLE_CLICK_THRESHOLD_SECS: f32 = 0.2;

fn main() {
    // enable wireframe rendering
    let mut wgpu_settings = WgpuSettings::default();
    wgpu_settings.features |= WgpuFeatures::POLYGON_MODE_LINE;

    App::new()
        .add_event::<TimerEvent>()
        .add_plugins(DefaultPlugins.set(RenderPlugin {
            render_creation: RenderCreation::Automatic(wgpu_settings),
            ..default()
        }))
        .add_plugins((
            EditorPlugin::new(),
            FrameTimeDiagnosticsPlugin,
            EntityCountDiagnosticsPlugin,
            DefaultPickingPlugins::build(DefaultPickingPlugins)
                .disable::<DefaultHighlightingPlugin>(),
            InfiniteGridPlugin,
        ))
        .add_plugins((PickingExtPlugin, WrappingCursorPlugin))
        .add_systems(Startup, (setup, setup_timer))
        .add_systems(
            Update,
            (
                auto_add_picking_to_meshes,
                deselect_all_on_esc,
                (update_timer, update_reference).chain(),
            ),
        )
        .run();
}

fn shapes(meshes: &mut Assets<Mesh>) -> Vec<Handle<Mesh>> {
    vec![
        meshes.add(Cuboid::default()),
        meshes.add(Capsule3d::default()),
        meshes.add(Torus::default()),
        meshes.add(Cylinder::default()),
        meshes.add(Sphere::default().mesh().ico(5).unwrap()),
        meshes.add(Sphere::default().mesh().uv(32, 18)),
    ]
}

#[derive(Resource)]
struct Timer {
    text_entity: Entity,
    start: Instant,
    interval: Duration,
    paused: Option<Duration>,
    hide: bool,
    adjusting_interval: bool,
}

impl Timer {
    fn time(&mut self) -> (Duration, bool) {
        if self.adjusting_interval {
            return (self.interval, false);
        }

        if let Some(paused) = self.paused {
            (paused, false)
        } else {
            let mut elapsed = self.start.elapsed();
            let reset = elapsed >= self.interval;
            if reset {
                self.start = self.start + self.interval;
                elapsed = self.start.elapsed();
            }

            (elapsed, reset)
        }
    }

    fn toggle_hide(&mut self) {
        self.hide = !self.hide;
    }

    fn is_paused(&self) -> bool {
        self.paused.is_some()
    }

    fn toggle_pause(&mut self) {
        self.set_pause(!self.is_paused());
    }

    fn set_pause(&mut self, paused: bool) {
        match (self.paused, paused) {
            (None, true) => {
                self.paused = Some(self.start.elapsed());
            }
            (Some(paused), false) => {
                self.start = Instant::now() - paused;
                self.paused = None;
            }
            _ => {}
        }
    }
}

#[derive(Component)]
struct TimerText;

#[derive(Event)]
struct TimerEvent;

fn update_timer(
    mut timer: ResMut<Timer>,
    mut query: Query<&mut Text, With<TimerText>>,
    mut timer_writer: EventWriter<TimerEvent>,
) {
    let (elapsed, reset) = timer.time();
    if reset {
        timer_writer.send(TimerEvent);
    }

    let text = if timer.hide {
        "".to_string()
    } else {
        format!("{:05.2}", elapsed.as_secs_f32())
    };
    query.get_mut(timer.text_entity).unwrap().sections[0].value = text;
}

fn setup_timer(mut commands: Commands, asset_server: Res<AssetServer>) {
    // ui camera
    commands.spawn(Camera2dBundle {
        camera: Camera {
            order: 10000,
            ..default()
        },
        ..default()
    });

    let mut text_entity = Entity::PLACEHOLDER;

    commands
        .spawn((NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::End,
                justify_content: JustifyContent::End,
                ..default()
            },
            ..default()
        },))
        .with_children(|parent| {
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(150.0),
                            height: Val::Px(65.0),
                            border: UiRect::all(Val::Px(5.0)),
                            // horizontally center child text
                            justify_content: JustifyContent::Center,
                            // vertically center child text
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        border_color: BorderColor(Color::BLACK),
                        background_color: Color::rgb(0.15, 0.15, 0.15).into(),

                        ..default()
                    },
                    On::<PointerEvent>::run(timer_interaction),
                ))
                .with_children(|parent| {
                    text_entity = parent
                        .spawn((
                            TextBundle::from_section(
                                "",
                                TextStyle {
                                    font: Handle::default(),
                                    font_size: 40.0,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                },
                            ),
                            TimerText,
                        ))
                        .id();
                });
        });

    commands.insert_resource(Timer {
        text_entity,
        start: Instant::now(),
        paused: None,
        interval: std::time::Duration::from_secs_f32(TIMER_INTERVAL),
        hide: false,
        adjusting_interval: false,
    });
}

#[derive(Default)]
struct TimerInteraction {
    // is_drag: bool,
}

fn timer_interaction(
    mut local: Local<TimerInteraction>,
    event: Listener<PointerEvent>,
    mut timer: ResMut<Timer>,
    mut wrapping_cursor: ResMut<NextState<WrappingCursorState>>,
    mut wrap_events: EventReader<Wrap>,
) {
    match &**event {
        PointerEvent::DragStart(e) => {
            timer.adjusting_interval = true;
            timer.set_pause(true);
            wrapping_cursor.set(WrappingCursorState::On);
        }
        PointerEvent::Drag(e) => {
            // ignoring pointer wrapping. this is not an ideal solution as one could imagine that there is
            // multiple Drag events in a single frame, but in practice that isn't the case in the current version
            // of bevy_mod_picking.
            if wrap_events.read().len() == 0 {
                timer.interval = Duration::from_secs_f32(
                    (timer.interval.as_secs_f32() + e.delta.x * 0.01).max(0.1),
                );
            }
        }
        PointerEvent::DragEnd(_) => {
            timer.adjusting_interval = false;
            timer.set_pause(false);
            wrapping_cursor.set(WrappingCursorState::Off);
        }
        PointerEvent::Up(e) => {
            if !timer.adjusting_interval {
                match e.button {
                    PointerButton::Primary => {
                        timer.toggle_pause();
                    }
                    PointerButton::Secondary => {
                        timer.toggle_hide();
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

#[derive(Resource)]
struct ReferenceManager {
    current_reference: Option<usize>,
    references: Vec<Entity>,
}

impl ReferenceManager {
    fn new(references: Vec<Entity>) -> Self {
        Self {
            current_reference: None,
            references,
        }
    }

    fn next_reference(&mut self, commands: &mut Commands) {
        let next_reference = match self.current_reference {
            Some(index) => index.wrapping_add(1),
            None => 0,
        };
        self.set_reference(next_reference, commands);
    }

    fn set_reference(&mut self, index: usize, commands: &mut Commands) {
        if let Some(current_reference) = self.current_reference {
            commands
                .get_entity(self.references[current_reference])
                .unwrap()
                .insert(Visibility::Hidden);
        };

        let next_reference = index % self.references.len();
        self.current_reference = Some(next_reference);

        commands
            .get_entity(self.references[next_reference])
            .unwrap()
            .insert(Visibility::Visible);
    }
}

#[derive(Component, Default)]
struct Reference;

#[derive(Bundle, Default)]
struct ReferenceBundle {
    pbr: PbrBundle,
    marker: Reference,
}

fn update_reference(
    mut commands: Commands,
    mut reference_manager: ResMut<ReferenceManager>,
    mut timer_event: EventReader<TimerEvent>,
) {
    if timer_event.read().count() > 0 {
        reference_manager.next_reference(&mut commands);
    }
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut editor_camera: Query<
        &mut bevy_editor_pls::default_windows::cameras::camera_3d_panorbit::PanOrbitCamera,
    >,
) {
    let mut references = Vec::new();

    let shapes = shapes(&mut meshes);
    for shape in shapes.into_iter() {
        let entity = commands
            .spawn(ReferenceBundle {
                pbr: PbrBundle {
                    mesh: shape,
                    material: materials.add(Color::rgb(0.7, 0.7, 0.7)),
                    visibility: Visibility::Hidden,
                    ..default()
                },
                ..default()
            })
            .id();
        references.push(entity);
    }

    let mut reference_manager = ReferenceManager::new(references);
    reference_manager.set_reference(0, &mut commands);
    commands.insert_resource(reference_manager);

    // plane
    // commands.spawn(InfiniteGridBundle {
    //     settings: InfiniteGridSettings { ..default() },
    //     ..default()
    // });

    // light
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        point_light: PointLight {
            shadows_enabled: true,
            ..default()
        },
        ..Default::default()
    });
    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        },
        EditorCamera,
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

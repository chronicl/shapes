use std::time::{Duration, Instant};

use bevy::render::mesh::PrimitiveTopology;
use bevy::render::render_resource::Face;
use bevy::render::view::RenderLayers;
use bevy::utils::{FloatOrd, HashMap, HashSet};
use bevy::{asset::LoadedFolder, gltf::Gltf, prelude::*};
use bevy_mod_picking::prelude::*;
use rand::Rng;

use crate::outline::generate_outline_mesh;
use crate::picking_ext::PointerEvent;
use crate::wrapping_cursor::{Wrap, WrappingCursorState};
use crate::MainCamera;

const LINE_ART_THICKNESS: f32 = 0.02;
const TIMER_INTERVAL: f32 = 3.0;
/// Could consider not hardcoding this path.
const REFERNCE_FOLDER: &str = "references";

pub struct ReferencePlugin;

impl Plugin for ReferencePlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<LineArtGizmo>()
            .add_event::<TimerEvent>()
            .add_systems(
                Startup,
                (insert_reference_manager, setup_timer, setup_gizmo_config),
            )
            .add_systems(
                Update,
                (
                    listen_for_loaded_folder,
                    (update_timer, update_reference).chain(),
                ),
            );
    }
}

fn insert_reference_manager(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(References::new(&asset_server));
}

fn setup_gizmo_config(mut config_store: ResMut<GizmoConfigStore>) {
    let (mut config, _) = config_store.config_mut::<LineArtGizmo>();
    config.line_width = LINE_ART_THICKNESS * 900.;
    config.line_perspective = true;
    // config.depth_bias = -10.;
}

fn update_reference(
    mut gizmo: Gizmos<LineArtGizmo>,
    mut commands: Commands,
    mut refs: ResMut<References>,
    mut timer_events: EventReader<TimerEvent>,
    transform_query: Query<&Transform>,
) {
    if refs.references.is_empty() {
        return;
    }

    if let Some(current) = refs.current_reference {
        let Reference { entity, edges, .. } = &refs.references[current];

        let transform = *transform_query.get(*entity).unwrap();

        for edge in edges.iter() {
            gizmo.line(transform * edge.0, transform * edge.1, Color::WHITE);
        }
    }

    // if there is no current reference set yet we do run this function despite the timer not having expired.
    if timer_events.read().count() == 0 && refs.current_reference.is_some() {
        return;
    }

    if let Some(current) = refs.current_reference {
        commands
            .entity(refs.references[current].entity)
            .insert(Visibility::Hidden);
    };

    if let Some(next) = refs.next_reference() {
        refs.current_reference = Some(next);
        commands.entity(refs.references[next].entity).insert((
            Visibility::Visible,
            Transform::from_rotation(random_rotation()),
        ));
    }
}

#[derive(Resource)]
pub struct References {
    pub references: Vec<Reference>,
    pub disabled_references: HashSet<usize>,
    pub current_reference: Option<usize>,
    pub loading_folder: Handle<LoadedFolder>,
}

#[derive(Debug, Clone)]
pub struct Reference {
    pub name: Name,
    pub entity: Entity,
    pub edges: Vec<(Vec3, Vec3)>,
}

/// Marker
#[derive(Component, Default)]
pub struct ReferenceMarker;

impl References {
    fn new(asset_server: &AssetServer) -> Self {
        Self {
            references: Vec::new(),
            disabled_references: default(),
            current_reference: None,
            loading_folder: asset_server.load_folder(REFERNCE_FOLDER),
        }
    }

    pub fn next_reference(&self) -> Option<usize> {
        let start = match self.current_reference {
            Some(current) => current + 1,
            None => {
                if self.disabled_references.len() == self.references.len() {
                    return None;
                } else {
                    0
                }
            }
        };

        // We are guaranteed to find a reference because the above match statement ensures it.
        for i in start.. {
            let i = i % self.references.len();
            if !self.disabled_references.contains(&i) {
                return Some(i);
            }
        }

        unreachable!()
    }

    pub fn set_current(&mut self, index: usize) {
        self.current_reference = Some(index);
    }

    pub fn set_active(&mut self, index: usize, active: bool) {
        if active {
            self.disabled_references.remove(&index);
        } else {
            self.disabled_references.insert(index);
        }
    }

    /// LoadedFolder must be loaded before calling this function.
    fn setup_references(
        &mut self,
        commands: &mut Commands,
        folders: &Assets<LoadedFolder>,
        gltfs: &Assets<Gltf>,
        scenes: &mut Assets<Scene>,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        camera_transform: &Transform,
    ) {
        let folder = folders.get(&self.loading_folder).unwrap();
        for reference in folder.handles.iter() {
            match reference.clone().try_typed::<Gltf>() {
                Ok(handle) => {
                    for scene_handle in gltfs.get(&handle).unwrap().scenes.clone() {
                        let scene = scenes.get_mut(&scene_handle).unwrap();
                        let world = &mut scene.world;

                        let mut q = world
                            .query::<(&Name, &Handle<Mesh>, &Handle<StandardMaterial>, &Parent)>();

                        let mut edges = Vec::new();
                        let mut outline_meshes = Vec::new();
                        // awkward workaround to get the name of the object
                        // (assuming a bunch of things like that there is only one object and only one mesh).
                        let mut name = None;
                        for (n, mesh_handle, material, parent) in q.iter(world) {
                            name = Some(n.clone());
                            let mesh = meshes.get(mesh_handle).unwrap();
                            if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
                                warn!("Mesh is not a triangle list: {:?}", mesh_handle);
                                continue;
                            }
                            edges.extend(sharp_edge_lines(
                                mesh,
                                (45.0f32.to_radians(), 135.0f32.to_radians()),
                            ));

                            let outline_mesh =
                                generate_outline_mesh(mesh, LINE_ART_THICKNESS).unwrap();
                            let outline_mesh_handle = meshes.add(outline_mesh);

                            outline_meshes.push((parent.get(), outline_mesh_handle));

                            let material = materials.get_mut(material).unwrap();
                            material.base_color = material.base_color.with_a(0.2);
                            material.alpha_mode = AlphaMode::Blend;
                            material.cull_mode = Some(Face::Back);
                        }

                        let material = materials.add(StandardMaterial {
                            base_color: Color::WHITE,
                            unlit: true,
                            cull_mode: Some(Face::Front),
                            ..Default::default()
                        });

                        for (parent, outline_mesh_handle) in outline_meshes {
                            world.entity_mut(parent).with_children(|parent| {
                                parent.spawn(PbrBundle {
                                    mesh: outline_mesh_handle,
                                    material: material.clone(),
                                    ..default()
                                });
                            });
                        }

                        let reference_entity = commands
                            .spawn((
                                SceneBundle {
                                    scene: scene_handle,
                                    visibility: Visibility::Hidden,
                                    ..default()
                                },
                                ReferenceMarker,
                            ))
                            .id();
                        self.references.push(Reference {
                            name: name.unwrap_or_default(),
                            entity: reference_entity,
                            edges,
                        });
                    }
                }
                Err(_) => {
                    warn!("Reference is not a scene: {:?}", reference);
                }
            }
        }
    }
}

fn listen_for_loaded_folder(
    mut commands: Commands,
    mut reference_manager: ResMut<References>,
    mut events: EventReader<AssetEvent<LoadedFolder>>,
    folders: Res<Assets<LoadedFolder>>,
    gltfs: Res<Assets<Gltf>>,
    mut scenes: ResMut<Assets<Scene>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera_query: Query<&Transform, With<MainCamera>>,
) {
    for e in events.read() {
        if let AssetEvent::LoadedWithDependencies { id } = e {
            if *id == reference_manager.loading_folder.id() {
                reference_manager.setup_references(
                    &mut commands,
                    &folders,
                    &gltfs,
                    &mut scenes,
                    &mut meshes,
                    &mut materials,
                    camera_query.single(),
                );
            }
        }
    }
}

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct LineArtGizmo;

fn sharp_edge_lines(mesh: &Mesh, radian_range: (f32, f32)) -> Vec<(Vec3, Vec3)> {
    let edge_angles = edge_angles(mesh);
    // println!("{:?}", edge_angles);

    let mut lines = Vec::new();
    for (a, b, angle) in edge_angles {
        let angle = angle.unwrap_or(0.0);
        if radian_range.0 < angle && angle < radian_range.1 {
            lines.push((a, b));
        }
    }

    lines
}

/// Panics if the mesh is not a triangle list.
/// Returns a list of all edges and the angle between the connected faces (in radians).
/// If the edge is only connected to one face the angle is None.
fn edge_angles(mesh: &Mesh) -> Vec<(Vec3, Vec3, Option<f32>)> {
    assert!(mesh.primitive_topology() == PrimitiveTopology::TriangleList);

    let vertices = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap();
    let mut indices_iter = mesh.indices().unwrap().iter();
    // println!("{}", mesh.indices().unwrap().len());

    #[derive(Debug, Eq, PartialEq, Hash)]
    struct Edge([FloatOrd; 3], [FloatOrd; 3]);
    // The two points of the edge mapped to the other vertices of the triangles the edge is part of.
    // The two points of the edge are ordered by x, y, z.
    let mut edges = HashMap::<Edge, (Vec3, Option<Vec3>)>::new();

    while let (Some(a), Some(b), Some(c)) = (
        indices_iter.next(),
        indices_iter.next(),
        indices_iter.next(),
    ) {
        let abc = [
            vertices[a].map(FloatOrd),
            vertices[b].map(FloatOrd),
            vertices[c].map(FloatOrd),
        ];
        for i in 0..3 {
            let a = abc[i];
            let b = abc[(i + 1) % 3];
            let edge = Edge(a.min(b), a.max(b));
            let c = abc[(i + 2) % 3];
            let c = Vec3::new(c[0].0, c[1].0, c[2].0);
            // println!("{:?}", edge);

            if let Some(other_points) = edges.get_mut(&edge) {
                assert!(other_points.1.is_none());
                if other_points.0 != c {
                    other_points.1 = Some(c);
                }
            } else {
                edges.insert(edge, (c, None));
            }
        }
    }

    // println!("{:#?}", edges);

    edges
        .into_iter()
        .map(|(Edge(a, b), (c, d))| {
            let (a, b) = (
                Vec3::new(a[0].0, a[1].0, a[2].0),
                Vec3::new(b[0].0, b[1].0, b[2].0),
            );

            let angle = if let Some(d) = d {
                let tangent = tangent_of_edge((a, b), c);
                let tangent2 = tangent_of_edge((a, b), d);
                Some(tangent.angle_between(tangent2))
            } else {
                None
            };

            (a, b, angle)
        })
        .collect()
}

fn tangent_of_edge(edge: (Vec3, Vec3), other_point: Vec3) -> Vec3 {
    let (a, b) = edge;
    let ab = b - a;
    let ao = other_point - a;
    let normal = ab.cross(ao).normalize();
    normal.cross(ab).normalize()
}

fn tangent_of_edge2(edge: (Vec3, Vec3), other_point: Vec3) -> Vec3 {
    let (a, b) = edge;
    let c = other_point;
    let t = (c - a).dot(b - a) / (b - a).length_squared();
    let d = a + (b - a) * t;
    (c - d).normalize()
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
                self.start += self.interval;
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

const UI_RENDER_LAYER: RenderLayers = RenderLayers::layer(1);

fn setup_timer(mut commands: Commands) {
    // ui camera
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                order: 10000,
                ..default()
            },
            ..default()
        },
        UI_RENDER_LAYER,
    ));

    let mut text_entity = Entity::PLACEHOLDER;

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::End,
                    justify_content: JustifyContent::End,
                    ..default()
                },
                ..default()
            },
            UI_RENDER_LAYER,
        ))
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

fn timer_interaction(
    mut timer: ResMut<Timer>,
    mut wrapping_cursor: ResMut<NextState<WrappingCursorState>>,
    mut wrap_events: EventReader<Wrap>,
    event: Listener<PointerEvent>,
) {
    match &**event {
        PointerEvent::DragStart(_) => {
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

fn random_rotation() -> Quat {
    Quat::from_euler(
        EulerRot::XYZ,
        rand::random::<f32>() * std::f32::consts::PI * 2.0,
        rand::random::<f32>() * std::f32::consts::PI * 2.0,
        rand::random::<f32>() * std::f32::consts::PI * 2.0,
    )
}

#[test]
fn test_camera() {
    let mut camera = Transform::default();
}

const SCALING_BOUND_LOWER_LOG: f32 = -1.2;
const SCALING_BOUND_UPPER_LOG: f32 = 1.2;

fn random_scale(rng: &mut impl Rng) -> Vec3 {
    let x_factor_log = rng.gen::<f32>() * (SCALING_BOUND_UPPER_LOG - SCALING_BOUND_LOWER_LOG)
        + SCALING_BOUND_LOWER_LOG;
    let y_factor_log = rng.gen::<f32>() * (SCALING_BOUND_UPPER_LOG - SCALING_BOUND_LOWER_LOG)
        + SCALING_BOUND_LOWER_LOG;
    let z_factor_log = rng.gen::<f32>() * (SCALING_BOUND_UPPER_LOG - SCALING_BOUND_LOWER_LOG)
        + SCALING_BOUND_LOWER_LOG;

    Vec3::new(
        x_factor_log.exp2(),
        y_factor_log.exp2(),
        z_factor_log.exp2(),
    )
}

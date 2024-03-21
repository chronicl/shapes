use bevy::{audio::Sample, prelude::*, window::PrimaryWindow};

pub struct WrappingCursorPlugin;

impl Plugin for WrappingCursorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_state(WrappingCursorState::Off)
            .insert_resource(WrappingCursor {
                threshold: 1.0,
                padding_for_new_pos: 1.0,
            })
            .add_event::<Wrap>()
            .add_systems(OnEnter(WrappingCursorState::On), start_wrapping_cursor)
            .add_systems(OnExit(WrappingCursorState::On), end_wrapping_cursor)
            .add_systems(PreUpdate, calculate_wrapping_cursor_position);
    }
}

#[derive(States, Debug, Clone, Hash, PartialEq, Eq)]
pub enum WrappingCursorState {
    Off,
    On,
}

#[derive(Event)]
pub struct Wrap;

/// For now only wrapping around the whole windows is supported but this could easily be changed.
#[derive(Resource)]
pub struct WrappingCursor {
    /// At this distance to the edge the cursor will wrap around. Must be greater than 0.0
    pub threshold: f32,
    /// When wrapping around threshold + this is the distance to the edge the new position will have.
    pub padding_for_new_pos: f32,
}

fn start_wrapping_cursor(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    for mut window in windows.iter_mut() {
        window.cursor.grab_mode = bevy::window::CursorGrabMode::Confined;
    }
}

fn end_wrapping_cursor(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    for mut window in windows.iter_mut() {
        window.cursor.grab_mode = bevy::window::CursorGrabMode::None;
    }
}

fn calculate_wrapping_cursor_position(
    wrap_state: Res<State<WrappingCursorState>>,
    wrap: Res<WrappingCursor>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut wrap_writer: EventWriter<Wrap>,
) {
    if wrap_state.get() != &WrappingCursorState::On {
        return;
    }

    // Query returns one window typically.
    for mut window in windows.iter_mut() {
        if let Some(cursor_pos) = window.cursor_position() {
            let (w, h) = (window.width(), window.height());
            let (x, y) = (cursor_pos.x, cursor_pos.y);

            let new_x = if (w - x) < wrap.threshold {
                wrap.threshold + wrap.padding_for_new_pos
            } else if x < wrap.threshold {
                w - wrap.threshold - wrap.padding_for_new_pos
            } else {
                x
            };
            let new_y = if (h - y) < wrap.threshold {
                wrap.threshold + wrap.padding_for_new_pos
            } else if y < wrap.threshold {
                h - wrap.threshold - wrap.padding_for_new_pos
            } else {
                y
            };

            if new_x != x || new_y != y {
                window.set_cursor_position(Some(Vec2::new(new_x, new_y)));
                wrap_writer.send(Wrap);
            }
        } else {
            println!("No cursor position");
        }
    }
}

use bevy::prelude::*;
use bevy_mod_picking::{picking_core::PickSet, prelude::*};

pub struct PickingExtPlugin;

impl Plugin for PickingExtPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EventListenerPlugin::<PointerEvent>::default())
            .add_systems(PreUpdate, proxy_pointer_event.in_set(PickSet::PostFocus));
    }
}

// Optimization: We could wrap this in a generic struct PointerEventSelected<E> where E can be any tuple like
// (Up, Down, Drag) and only those events are sent to the entity listener.
#[derive(Event, Debug, Clone)]
pub enum PointerEvent {
    Up(Pointer<Up>),
    Down(Pointer<Down>),
    Move(Pointer<Move>),
    Over(Pointer<Over>),
    Out(Pointer<Out>),

    Drag(Pointer<Drag>),
    DragStart(Pointer<DragStart>),
    DragEnd(Pointer<DragEnd>),
    DragEnter(Pointer<DragEnter>),
    DragLeave(Pointer<DragLeave>),
    DragOver(Pointer<DragOver>),
    Drop(Pointer<Drop>),
}

impl EntityEvent for PointerEvent {
    fn target(&self) -> Entity {
        match self {
            PointerEvent::Up(e) => e.target,
            PointerEvent::Down(e) => e.target,
            PointerEvent::Move(e) => e.target,
            PointerEvent::Over(e) => e.target,
            PointerEvent::Out(e) => e.target,

            PointerEvent::Drag(e) => e.target,
            PointerEvent::DragStart(e) => e.target,
            PointerEvent::DragEnd(e) => e.target,
            PointerEvent::DragEnter(e) => e.target,
            PointerEvent::DragLeave(e) => e.target,
            PointerEvent::DragOver(e) => e.target,
            PointerEvent::Drop(e) => e.target,
        }
    }

    fn can_bubble(&self) -> bool {
        match self {
            PointerEvent::Up(e) => e.can_bubble(),
            PointerEvent::Down(e) => e.can_bubble(),
            PointerEvent::Move(e) => e.can_bubble(),
            PointerEvent::Over(e) => e.can_bubble(),
            PointerEvent::Out(e) => e.can_bubble(),

            PointerEvent::Drag(e) => e.can_bubble(),
            PointerEvent::DragStart(e) => e.can_bubble(),
            PointerEvent::DragEnd(e) => e.can_bubble(),
            PointerEvent::DragEnter(e) => e.can_bubble(),
            PointerEvent::DragLeave(e) => e.can_bubble(),
            PointerEvent::DragOver(e) => e.can_bubble(),
            PointerEvent::Drop(e) => e.can_bubble(),
        }
    }
}

fn proxy_pointer_event(
    mut event_writer: EventWriter<PointerEvent>,
    mut up_reader: EventReader<Pointer<Up>>,
    mut down_reader: EventReader<Pointer<Down>>,
    mut move_reader: EventReader<Pointer<Move>>,
    mut over_reader: EventReader<Pointer<Over>>,
    mut out_reader: EventReader<Pointer<Out>>,

    mut drag_reader: EventReader<Pointer<Drag>>,
    mut drag_start_reader: EventReader<Pointer<DragStart>>,
    mut drag_end_reader: EventReader<Pointer<DragEnd>>,
    mut drag_enter_reader: EventReader<Pointer<DragEnter>>,
    mut drag_leave_reader: EventReader<Pointer<DragLeave>>,
    mut drag_over_reader: EventReader<Pointer<DragOver>>,
    mut drop_reader: EventReader<Pointer<Drop>>,
) {
    for event in up_reader.read() {
        event_writer.send(PointerEvent::Up(event.clone()));
    }
    for event in down_reader.read() {
        event_writer.send(PointerEvent::Down(event.clone()));
    }
    for event in move_reader.read() {
        event_writer.send(PointerEvent::Move(event.clone()));
    }
    for event in over_reader.read() {
        event_writer.send(PointerEvent::Over(event.clone()));
    }
    for event in out_reader.read() {
        event_writer.send(PointerEvent::Out(event.clone()));
    }

    for event in drag_reader.read() {
        event_writer.send(PointerEvent::Drag(event.clone()));
    }
    for event in drag_start_reader.read() {
        event_writer.send(PointerEvent::DragStart(event.clone()));
    }
    for event in drag_end_reader.read() {
        event_writer.send(PointerEvent::DragEnd(event.clone()));
    }
    for event in drag_enter_reader.read() {
        event_writer.send(PointerEvent::DragEnter(event.clone()));
    }
    for event in drag_leave_reader.read() {
        event_writer.send(PointerEvent::DragLeave(event.clone()));
    }
    for event in drag_over_reader.read() {
        event_writer.send(PointerEvent::DragOver(event.clone()));
    }
    for event in drop_reader.read() {
        event_writer.send(PointerEvent::Drop(event.clone()));
    }
}

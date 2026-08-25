//! Wayland foreign-toplevel listing (wlr + ext) and wlr activate.

use super::app_resolve::resolve_app_id;
use crate::window_match::titles_equal;
use crate::{window_matches_process, CaptureError, WindowInfo};
use sqyre_ports::AutomationError;
use std::collections::HashMap;
use wayland_client::protocol::{wl_output, wl_registry, wl_seat};
use wayland_client::{event_created_child, Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::{
    ext_foreign_toplevel_handle_v1, ext_foreign_toplevel_list_v1,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1, zwlr_foreign_toplevel_manager_v1,
};

#[derive(Default, Clone)]
struct Draft {
    title: String,
    app_id: String,
    identifier: String,
    activated: bool,
    closed: bool,
}

#[derive(Default)]
struct State {
    drafts: HashMap<u32, Draft>,
    wlr_handles: HashMap<u32, zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1>,
    ext_handles: HashMap<u32, ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1>,
    wlr_mgr: Option<zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1>,
    ext_list: Option<ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1>,
    seat: Option<wl_seat::WlSeat>,
}

impl State {
    fn draft(&mut self, id: u32) -> &mut Draft {
        self.drafts.entry(id).or_default()
    }
}

pub(crate) fn list_windows() -> Result<Vec<WindowInfo>, CaptureError> {
    let (_conn, mut queue, mut state) = bind()?;
    pump(&mut queue, &mut state)?;
    if state.wlr_mgr.is_none() && state.ext_list.is_none() {
        return Err(CaptureError::Message(
            "compositor does not advertise foreign-toplevel".into(),
        ));
    }
    Ok(collect_infos(&state))
}

pub(crate) fn active_window() -> Result<Option<WindowInfo>, CaptureError> {
    let infos = list_raw()?;
    Ok(infos.into_iter().find(|(_, d)| d.activated).map(|(w, _)| w))
}

pub(crate) fn activate(process_path: &str, window_title: &str) -> Result<bool, AutomationError> {
    let (conn, mut queue, mut state) =
        bind().map_err(|e| AutomationError::Backend(e.to_string()))?;
    pump(&mut queue, &mut state).map_err(|e| AutomationError::Backend(e.to_string()))?;
    let Some(seat) = state.seat.clone() else {
        return Ok(false);
    };
    let mut target: Option<zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1> = None;
    for (id, draft) in &state.drafts {
        if draft.closed {
            continue;
        }
        let info = info_from_draft(draft);
        if !titles_equal(&info.title, window_title) {
            continue;
        }
        if !window_matches_process(&info, process_path) && !info.process_path.is_empty() {
            continue;
        }
        if let Some(handle) = state.wlr_handles.get(id) {
            target = Some(handle.clone());
            break;
        }
    }
    let Some(handle) = target else {
        return Ok(false);
    };
    handle.activate(&seat);
    let _ = conn.flush();
    queue
        .roundtrip(&mut state)
        .map_err(|e| AutomationError::Backend(format!("activate roundtrip: {e}")))?;
    Ok(true)
}

fn list_raw() -> Result<Vec<(WindowInfo, Draft)>, CaptureError> {
    let (_conn, mut queue, mut state) = bind()?;
    pump(&mut queue, &mut state)?;
    if state.wlr_mgr.is_none() && state.ext_list.is_none() {
        return Err(CaptureError::Message(
            "compositor does not advertise foreign-toplevel".into(),
        ));
    }
    Ok(state
        .drafts
        .values()
        .filter(|d| !d.closed && !d.title.trim().is_empty())
        .map(|d| (info_from_draft(d), Draft { ..d.clone() }))
        .collect())
}

fn collect_infos(state: &State) -> Vec<WindowInfo> {
    state
        .drafts
        .values()
        .filter(|d| !d.closed && !d.title.trim().is_empty())
        .map(info_from_draft)
        .collect()
}

fn info_from_draft(draft: &Draft) -> WindowInfo {
    let (process_name, process_path) = resolve_app_id(&draft.app_id);
    WindowInfo {
        title: draft.title.clone(),
        process_name,
        process_path,
        icon: None,
    }
}

fn bind() -> Result<(Connection, EventQueue<State>, State), CaptureError> {
    let conn = Connection::connect_to_env()
        .map_err(|e| CaptureError::Message(format!("wayland connect: {e}")))?;
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    let display = conn.display();
    let _registry = display.get_registry(&qh, ());
    let mut state = State::default();
    queue
        .roundtrip(&mut state)
        .map_err(|e| CaptureError::Message(format!("registry roundtrip: {e}")))?;
    Ok((conn, queue, state))
}

fn pump(queue: &mut EventQueue<State>, state: &mut State) -> Result<(), CaptureError> {
    if state.wlr_mgr.is_none() && state.ext_list.is_none() {
        return Ok(());
    }
    for _ in 0..4 {
        queue
            .roundtrip(state)
            .map_err(|e| CaptureError::Message(format!("toplevel roundtrip: {e}")))?;
    }
    Ok(())
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        else {
            return;
        };
        if interface
            == zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1::interface().name
            && state.wlr_mgr.is_none()
        {
            let mgr = registry
                .bind::<zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1, _, _>(
                    name,
                    version.min(2),
                    qh,
                    (),
                );
            state.wlr_mgr = Some(mgr);
        } else if interface
            == ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1::interface().name
            && state.ext_list.is_none()
        {
            let list = registry
                .bind::<ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1, _, _>(
                    name,
                    version.min(1),
                    qh,
                    (),
                );
            state.ext_list = Some(list);
        } else if interface == wl_seat::WlSeat::interface().name && state.seat.is_none() {
            state.seat = Some(registry.bind::<wl_seat::WlSeat, _, _>(name, version.min(1), qh, ()));
        }
    }
}

impl Dispatch<zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1,
        event: zwlr_foreign_toplevel_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel } = event {
            let id = toplevel.id().protocol_id();
            state.draft(id);
            state.wlr_handles.insert(id, toplevel);
        }
    }

    event_created_child!(State, zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1, [
        zwlr_foreign_toplevel_manager_v1::EVT_TOPLEVEL_OPCODE => (zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1, ())
    ]);
}

impl Dispatch<zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1, ()> for State {
    fn event(
        state: &mut Self,
        handle: &zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1,
        event: zwlr_foreign_toplevel_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let id = handle.id().protocol_id();
        match event {
            zwlr_foreign_toplevel_handle_v1::Event::Title { title } => {
                state.draft(id).title = title;
            }
            zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                state.draft(id).app_id = app_id;
            }
            zwlr_foreign_toplevel_handle_v1::Event::State { state: bytes } => {
                state.draft(id).activated = wlr_activated(&bytes);
            }
            zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                state.draft(id).closed = true;
            }
            _ => {}
        }
    }

    event_created_child!(State, zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1, [
        zwlr_foreign_toplevel_handle_v1::EVT_OUTPUT_ENTER_OPCODE => (wl_output::WlOutput, ())
    ]);
}

impl Dispatch<ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1,
        event: ext_foreign_toplevel_list_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let ext_foreign_toplevel_list_v1::Event::Toplevel { toplevel } = event {
            let id = toplevel.id().protocol_id();
            state.draft(id);
            state.ext_handles.insert(id, toplevel);
        }
    }

    event_created_child!(State, ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1, [
        ext_foreign_toplevel_list_v1::EVT_TOPLEVEL_OPCODE => (ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, ())
    ]);
}

impl Dispatch<ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, ()> for State {
    fn event(
        state: &mut Self,
        handle: &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
        event: ext_foreign_toplevel_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let id = handle.id().protocol_id();
        match event {
            ext_foreign_toplevel_handle_v1::Event::Title { title } => {
                state.draft(id).title = title;
            }
            ext_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                state.draft(id).app_id = app_id;
            }
            ext_foreign_toplevel_handle_v1::Event::Identifier { identifier } => {
                state.draft(id).identifier = identifier;
            }
            ext_foreign_toplevel_handle_v1::Event::Closed => {
                state.draft(id).closed = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_output::WlOutput, ()> for State {
    fn event(
        _: &mut Self,
        _: &wl_output::WlOutput,
        _: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

fn wlr_activated(bytes: &[u8]) -> bool {
    const ACTIVATED: u32 = 2;
    bytes
        .chunks_exact(4)
        .any(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]) == ACTIVATED)
}

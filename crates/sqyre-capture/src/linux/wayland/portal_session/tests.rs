//! Unit tests for portal session helpers.

use super::eis::{eis_wait_decision, reseat_target, EisWaitDecision, EisWaitSnapshot};
use super::tokens::{read_restore_token_at, write_restore_token_at};
use super::*;

#[test]
fn host_register_keeps_a_cached_id_and_retries_a_missing_desktop_file() {
    assert!(matches!(
        host_register_outcome(
            "org.freedesktop.portal.Error.Failed: Connection already associated with an application ID"
        ),
        HostRegister::Ready
    ));
    assert!(matches!(
        host_register_outcome("App info not found for 'com.sqyre.app'"),
        HostRegister::Retry
    ));
    assert!(matches!(
        host_register_outcome("org.freedesktop.DBus.Error.ServiceUnknown"),
        HostRegister::Stop
    ));
}

#[test]
fn eis_wait_keeps_polling_while_the_handshake_is_in_flight() {
    let snap = EisWaitSnapshot {
        ready: false,
        connecting: true,
        exhausted: false,
        failure: None,
        granted: true,
    };
    assert!(matches!(eis_wait_decision(&snap), EisWaitDecision::Poll));
}

#[test]
fn eis_wait_reports_grant_once_the_handshake_budget_is_spent() {
    let snap = EisWaitSnapshot {
        ready: false,
        connecting: true,
        exhausted: true,
        failure: None,
        granted: true,
    };
    match eis_wait_decision(&snap) {
        EisWaitDecision::Failed(err) => {
            assert!(err.contains("EIS is not ready"), "{err}");
        }
        EisWaitDecision::Ready | EisWaitDecision::Poll => panic!("expected failure"),
    }
}

#[test]
fn eis_wait_surfaces_the_handshake_error() {
    let snap = EisWaitSnapshot {
        ready: false,
        connecting: false,
        exhausted: false,
        failure: Some("ConnectToEIS: EIS sender not available".into()),
        granted: true,
    };
    match eis_wait_decision(&snap) {
        EisWaitDecision::Failed(err) => assert!(err.contains("EIS sender not available")),
        EisWaitDecision::Ready | EisWaitDecision::Poll => panic!("expected failure"),
    }
}

#[test]
fn eis_wait_ready_wins_over_a_stale_failure() {
    let snap = EisWaitSnapshot {
        ready: true,
        connecting: false,
        exhausted: true,
        failure: Some("old".into()),
        granted: true,
    };
    assert!(matches!(eis_wait_decision(&snap), EisWaitDecision::Ready));
}

#[test]
fn restore_token_roundtrip() {
    let dir = std::env::temp_dir().join(format!(
        "sqyre-screencast-token-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("wayland-screencast.token");
    write_restore_token_at(&path, Some(" token-value \n"));
    assert_eq!(read_restore_token_at(&path).as_deref(), Some("token-value"));
    write_restore_token_at(&path, None);
    assert!(read_restore_token_at(&path).is_none());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn click_reseat_uses_live_cursor_after_an_old_move() {
    assert_eq!(
        reseat_target(Some((10, 20)), Some((30, 40)), false),
        (Some((30, 40)), "cursor")
    );
    assert_eq!(
        reseat_target(Some((10, 20)), Some((30, 40)), true),
        (Some((10, 20)), "warp")
    );
    assert_eq!(
        reseat_target(Some((10, 20)), Some((10, 20)), false),
        (Some((10, 20)), "cursor")
    );
    assert_eq!(
        reseat_target(None, Some((5, 6)), false),
        (Some((5, 6)), "cursor")
    );
    assert_eq!(
        reseat_target(Some((1, 2)), None, true),
        (Some((1, 2)), "warp")
    );
    // A stale warp must not move the pointer when no live cursor is known.
    assert_eq!(reseat_target(Some((1, 2)), None, false), (None, "none"));
    assert_eq!(reseat_target(None, None, false), (None, "none"));
}

#[test]
fn stream_cursor_maps_into_dest_rect() {
    let dest = DesktopRect {
        x: 1920,
        y: 0,
        w: 2560,
        h: 1440,
    };
    assert_eq!(
        stream_cursor_to_desktop(dest, 100, 50, 2560, 1440),
        (2020, 50)
    );
    assert_eq!(
        stream_cursor_to_desktop(dest, 1280, 720, 1280, 720),
        (4480, 1440)
    );
    assert_eq!(stream_cursor_to_desktop(dest, 10, 20, 0, 0), (1930, 20));
}

#[test]
fn assign_streams_uses_x11_y_offsets() {
    let mut streams = vec![
        PwStreamSetup {
            node_id: 10,
            rect: DesktopRect {
                x: 0,
                y: 0,
                w: 1920,
                h: 1080,
            },
        },
        PwStreamSetup {
            node_id: 11,
            rect: DesktopRect {
                x: 1920,
                y: 0,
                w: 2560,
                h: 1440,
            },
        },
    ];
    let layout = [
        DesktopRect {
            x: 0,
            y: 360,
            w: 1920,
            h: 1080,
        },
        DesktopRect {
            x: 1920,
            y: 0,
            w: 2560,
            h: 1440,
        },
    ];
    assign_streams_to_layout(&mut streams, &layout);
    assert_eq!(streams[0].rect.y, 360);
    assert_eq!(streams[1].rect.x, 1920);
    assert_eq!(streams[1].rect.y, 0);
}

#[test]
fn shared_rects_follow_streams_not_full_x11_layout() {
    let mut streams = vec![PwStreamSetup {
        node_id: 11,
        rect: DesktopRect {
            x: 0,
            y: 0,
            w: 2560,
            h: 1440,
        },
    }];
    let layout = [
        DesktopRect {
            x: 0,
            y: 360,
            w: 1920,
            h: 1080,
        },
        DesktopRect {
            x: 1920,
            y: 0,
            w: 2560,
            h: 1440,
        },
    ];
    let rects = shared_monitor_rects(&mut streams, &layout);
    assert_eq!(rects.len(), 1);
    assert_eq!(rects[0].w, 2560);
    assert_eq!(rects[0].h, 1440);
    assert_eq!(rects[0].x, 1920);
}

#[test]
fn cosmic_vertical_pair_keeps_physical_rows() {
    use super::super::compositor_kick::WlOutputGeom;
    let outputs = [
        WlOutputGeom {
            x: 0,
            y: 0,
            width: 3840,
            height: 2160,
            scale: 2,
        },
        WlOutputGeom {
            x: 0,
            y: 1080,
            width: 3840,
            height: 2160,
            scale: 2,
        },
    ];
    let mut streams = vec![
        PwStreamSetup {
            node_id: 1,
            rect: DesktopRect {
                x: 0,
                y: 0,
                w: 3840,
                h: 2160,
            },
        },
        PwStreamSetup {
            node_id: 2,
            rect: DesktopRect {
                x: 0,
                y: 1080,
                w: 3840,
                h: 2160,
            },
        },
    ];
    place_cosmic_streams(&mut streams, &outputs);
    assert_eq!(streams[1].rect.y, 2160);
    // Logical positions overlap once the size is physical; after the
    // scale shift they stack and must not be packed onto one row.
    layout_stream_rects(&mut streams);
    assert_eq!(streams[0].rect.y, 0);
    assert_eq!(streams[1].rect.y, 2160);
    assert_eq!(streams[1].rect.x, 0);

    // A logical-sized stream (not the hardware mode) is not shifted.
    let logical = DesktopRect {
        x: 0,
        y: 1080,
        w: 1920,
        h: 1080,
    };
    assert_eq!(place_cosmic_stream(logical, &outputs), logical);
}

#[test]
fn overlapping_streams_layout_ltr() {
    let mut streams = vec![
        PwStreamSetup {
            node_id: 1,
            rect: DesktopRect {
                x: 0,
                y: 0,
                w: 1920,
                h: 1080,
            },
        },
        PwStreamSetup {
            node_id: 2,
            rect: DesktopRect {
                x: 0,
                y: 0,
                w: 1920,
                h: 1080,
            },
        },
    ];
    layout_stream_rects(&mut streams);
    assert_eq!(streams[0].rect.x, 0);
    assert_eq!(streams[1].rect.x, 1920);
}

#[test]
fn union_monitor_rects() {
    let a = DesktopRect {
        x: 0,
        y: 0,
        w: 1920,
        h: 1080,
    };
    let b = DesktopRect {
        x: 1920,
        y: 0,
        w: 1920,
        h: 1080,
    };
    let u = union_rect(a, b);
    assert_eq!(u.w, 3840);
    assert_eq!(u.h, 1080);
}

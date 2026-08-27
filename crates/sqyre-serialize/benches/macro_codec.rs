//! Encode/decode a small Wait+Loop+Click macro.
//!
//! Run: `cargo bench -p sqyre-serialize` or `make bench`.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sqyre_domain::{Action, ActionId, ActionKind, Macro, MouseButton, PressState, ScalarValue};
use sqyre_serialize::{decode_macro_from_yaml, encode_macro_to_yaml};
use std::time::Duration;

fn sample_macro() -> Macro {
    let mut m = Macro::new("bench", 0, vec![]);
    m.root = sqyre_domain::root_loop(vec![
        Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(25),
            },
        },
        Action {
            id: ActionId::new(),
            kind: ActionKind::Loop {
                name: "inner".into(),
                count: ScalarValue::Int(3),
                subactions: vec![Action {
                    id: ActionId::new(),
                    kind: ActionKind::Click {
                        button: MouseButton::Left,
                        state: PressState::Tap,
                    },
                }],
            },
        },
    ]);
    m
}

fn bench_codec(c: &mut Criterion) {
    let macro_ = sample_macro();
    let yaml = encode_macro_to_yaml(&macro_).expect("encode");
    c.bench_function("encode_macro_wait_loop_click", |b| {
        b.iter(|| encode_macro_to_yaml(black_box(&macro_)).unwrap());
    });
    c.bench_function("decode_macro_wait_loop_click", |b| {
        b.iter(|| decode_macro_from_yaml(black_box(&yaml)).unwrap());
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(1));
    targets = bench_codec
}
criterion_main!(benches);

//! SPA pod / PipeWire stream parameter builders.

use crate::error::CaptureError;
use pipewire as pw;
use pw::spa::buffer::{
    meta::{MetaCursor, MetaHeader, Metadata},
    DataType,
};
use pw::spa::param::video::VideoFormat;
use pw::spa::pod::Pod;
use sqyre_ports::cap_log;

pub(super) fn pw_video_enum_format_bytes() -> Result<Vec<u8>, CaptureError> {
    let obj = pw::spa::pod::object!(
        pw::spa::utils::SpaTypes::ObjectParamFormat,
        pw::spa::param::ParamType::EnumFormat,
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::MediaType,
            Id,
            pw::spa::param::format::MediaType::Video
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::MediaSubtype,
            Id,
            pw::spa::param::format::MediaSubtype::Raw
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            VideoFormat::RGBA,
            VideoFormat::RGBx,
            VideoFormat::BGRx,
            VideoFormat::BGRA,
        ),
        // Mutter fixates VideoFramerate at 0/1 (emit-on-damage); COSMIC fixates
        // 60/1. The range must include 0/1 or Mutter fails to negotiate.
        // Periodic frames come from VideoMaxFramerate (same as xdg-desktop-portal-wlr).
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::VideoFramerate,
            Choice,
            Range,
            Fraction,
            pw::spa::utils::Fraction { num: 0, denom: 1 },
            pw::spa::utils::Fraction { num: 0, denom: 1 },
            pw::spa::utils::Fraction {
                num: 1000,
                denom: 1
            }
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::VideoMaxFramerate,
            Choice,
            Range,
            Fraction,
            pw::spa::utils::Fraction { num: 30, denom: 1 },
            pw::spa::utils::Fraction { num: 1, denom: 1 },
            pw::spa::utils::Fraction { num: 30, denom: 1 }
        ),
    );
    pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(obj),
    )
    .map(|v| v.0.into_inner())
    .map_err(|e| CaptureError::Message(format!("PipeWire format pod: {e}")))
}

/// `SPA_PARAM_Meta` keys (`enum spa_param_meta` in spa/param/buffers.h).
pub(super) struct ParamMetaKey(u32);

impl ParamMetaKey {
    const TYPE: Self = Self(1);
    const SIZE: Self = Self(2);
    const fn as_raw(&self) -> u32 {
        self.0
    }
}

/// `SPA_PARAM_BUFFERS_*` keys (`enum spa_param_buffers`).
pub(super) struct ParamBuffersKey(u32);

impl ParamBuffersKey {
    const BUFFERS: Self = Self(1);
    const BLOCKS: Self = Self(2);
    const SIZE: Self = Self(3);
    const STRIDE: Self = Self(4);
    const DATA_TYPE: Self = Self(6);
    const fn as_raw(&self) -> u32 {
        self.0
    }
}

pub(super) struct SpaId(u32);

impl SpaId {
    const fn as_raw(&self) -> u32 {
        self.0
    }
}

pub(super) fn cursor_meta_size(w: u32, h: u32) -> i32 {
    let base = std::mem::size_of::<pw::spa::sys::spa_meta_cursor>()
        + std::mem::size_of::<pw::spa::sys::spa_meta_bitmap>();
    (base + (w as usize) * (h as usize) * 4) as i32
}

pub(super) fn serialize_spa_object(obj: pw::spa::pod::Object) -> Result<Vec<u8>, CaptureError> {
    pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(obj),
    )
    .map(|v| v.0.into_inner())
    .map_err(|e| CaptureError::Message(format!("PipeWire pod: {e}")))
}

pub(super) fn int_choice_range(
    key: u32,
    default: i32,
    min: i32,
    max: i32,
) -> pw::spa::pod::Property {
    pw::spa::pod::Property::new(
        key,
        pw::spa::pod::Value::Choice(pw::spa::pod::ChoiceValue::Int(pw::spa::utils::Choice(
            pw::spa::utils::ChoiceFlags::empty(),
            pw::spa::utils::ChoiceEnum::Range { default, min, max },
        ))),
    )
}

pub(super) fn int_choice_flags(key: u32, default: i32, flags: Vec<i32>) -> pw::spa::pod::Property {
    pw::spa::pod::Property::new(
        key,
        pw::spa::pod::Value::Choice(pw::spa::pod::ChoiceValue::Int(pw::spa::utils::Choice(
            pw::spa::utils::ChoiceFlags::empty(),
            pw::spa::utils::ChoiceEnum::Flags { default, flags },
        ))),
    )
}

pub(super) fn pw_cursor_meta_bytes() -> Result<Vec<u8>, CaptureError> {
    let obj = pw::spa::pod::Object {
        type_: pw::spa::utils::SpaTypes::ObjectParamMeta.as_raw(),
        id: pw::spa::param::ParamType::Meta.as_raw(),
        properties: vec![
            pw::spa::pod::property!(ParamMetaKey::TYPE, Id, SpaId(MetaCursor::META_TYPE)),
            int_choice_range(
                ParamMetaKey::SIZE.as_raw(),
                cursor_meta_size(64, 64),
                cursor_meta_size(1, 1),
                cursor_meta_size(384, 384),
            ),
        ],
    };
    serialize_spa_object(obj)
}

pub(super) fn pw_header_meta_bytes() -> Result<Vec<u8>, CaptureError> {
    let obj = pw::spa::pod::object!(
        pw::spa::utils::SpaTypes::ObjectParamMeta,
        pw::spa::param::ParamType::Meta,
        pw::spa::pod::property!(ParamMetaKey::TYPE, Id, SpaId(MetaHeader::META_TYPE)),
        pw::spa::pod::property!(
            ParamMetaKey::SIZE,
            Int,
            std::mem::size_of::<pw::spa::sys::spa_meta_header>() as i32
        ),
    );
    serialize_spa_object(obj)
}

/// CPU-readable buffer types. COSMIC's DMA-BUF frames set `maxsize` to 0 and
/// are not mapped; requesting `SPA_DATA_DmaBuf` makes that path win on some
/// streams so those frames never copy, while a MemFd stream in the same
/// session still works. Mutter and wlroots deliver MemFd when DMA-BUF is absent.
pub(super) fn cpu_buffer_type_mask() -> i32 {
    (1 << DataType::MemPtr.as_raw()) | (1 << DataType::MemFd.as_raw())
}

pub(super) fn pw_buffers_param_bytes(size: i32, stride: i32) -> Result<Vec<u8>, CaptureError> {
    let data_mask = cpu_buffer_type_mask();
    let obj = pw::spa::pod::Object {
        type_: pw::spa::utils::SpaTypes::ObjectParamBuffers.as_raw(),
        id: pw::spa::param::ParamType::Buffers.as_raw(),
        properties: vec![
            int_choice_range(ParamBuffersKey::BUFFERS.as_raw(), 8, 1, 32),
            pw::spa::pod::property!(ParamBuffersKey::BLOCKS, Int, 1),
            pw::spa::pod::Property::new(
                ParamBuffersKey::SIZE.as_raw(),
                pw::spa::pod::Value::Int(size),
            ),
            pw::spa::pod::Property::new(
                ParamBuffersKey::STRIDE.as_raw(),
                pw::spa::pod::Value::Int(stride),
            ),
            int_choice_flags(
                ParamBuffersKey::DATA_TYPE.as_raw(),
                data_mask,
                vec![data_mask],
            ),
        ],
    };
    serialize_spa_object(obj)
}

/// Mutter only attaches `SPA_META_Cursor` after the client finishes negotiation
/// with `pw_stream_update_params` (connect-time Meta is not enough).
pub(super) fn apply_stream_buffer_params(
    stream: &pw::stream::Stream,
    size: i32,
    stride: i32,
) -> Result<(), CaptureError> {
    let buffers = pw_buffers_param_bytes(size, stride)?;
    let header = pw_header_meta_bytes()?;
    let cursor = pw_cursor_meta_bytes()?;
    let buffers_pod = Pod::from_bytes(&buffers)
        .ok_or_else(|| CaptureError::Message("PipeWire buffers pod invalid".into()))?;
    let header_pod = Pod::from_bytes(&header)
        .ok_or_else(|| CaptureError::Message("PipeWire header meta pod invalid".into()))?;
    let cursor_pod = Pod::from_bytes(&cursor)
        .ok_or_else(|| CaptureError::Message("PipeWire cursor meta pod invalid".into()))?;
    let mut params = [buffers_pod, header_pod, cursor_pod];
    stream
        .update_params(&mut params)
        .map_err(|e| CaptureError::Message(format!("PipeWire update_params: {e}")))
}

/// PipeWire loads SPA plugins from the host; bundled `libpipewire` in AppImage/bundle breaks this.
pub(super) fn ensure_spa_plugin_dir() {
    if std::env::var_os("SPA_PLUGIN_DIR").is_some() {
        return;
    }
    const CANDIDATES: &[&str] = &[
        "/usr/lib/x86_64-linux-gnu/spa-0.2",
        "/usr/lib64/spa-0.2",
        "/usr/lib/spa-0.2",
    ];
    for dir in CANDIDATES {
        let support = std::path::Path::new(dir).join("support/libspa-support.so");
        if support.exists() {
            // SAFETY: called on the portal thread before PipeWire worker threads exist.
            unsafe { std::env::set_var("SPA_PLUGIN_DIR", dir) };
            cap_log("PORTAL", "spa", &format!("SPA_PLUGIN_DIR={dir}"));
            return;
        }
    }
}

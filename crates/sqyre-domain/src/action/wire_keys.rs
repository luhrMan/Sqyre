//! Single registry for ActionKind wire `type` strings.
//!
//! Generates serde discriminant tags, [`ActionKind::type_key`], and
//! [`WIRE_TYPE_KEYS`] so those cannot drift apart.

use super::ActionKind;
use serde::{Deserialize, Serialize};

/// Wire `type` keys in enum / serde order (not picker order).
pub const WIRE_TYPE_KEYS: &[&str] = WIRE_TYPE_KEYS_INNER;

macro_rules! define_action_wire_keys {
    (
        $(fields $Fields:ident => $fkey:literal / $FTag:ident,)+
        $(newtype $Newtype:ident => $nkey:literal / $NTag:ident,)+
        $(unit $Unit:ident => $ukey:literal / $UTag:ident,)+
    ) => {
        $(
            #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
            pub(super) enum $FTag {
                #[serde(rename = $fkey)]
                Tag,
            }
        )+
        $(
            #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
            pub(super) enum $NTag {
                #[serde(rename = $nkey)]
                Tag,
            }
        )+
        $(
            #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
            pub(super) enum $UTag {
                #[serde(rename = $ukey)]
                Tag,
            }
        )+

        const WIRE_TYPE_KEYS_INNER: &[&str] = &[
            $($fkey,)+
            $($nkey,)+
            $($ukey,)+
        ];

        impl ActionKind {
            pub fn type_key(&self) -> &'static str {
                match self {
                    $(Self::$Fields { .. } => $fkey,)+
                    $(Self::$Newtype(_) => $nkey,)+
                    $(Self::$Unit => $ukey,)+
                }
            }
        }
    };
}

define_action_wire_keys! {
    fields Loop => "loop" / TagLoop,
    fields While => "while" / TagWhile,
    fields Conditional => "conditional" / TagConditional,
    fields ImageSearch => "imagesearch" / TagImageSearch,
    fields Ocr => "ocr" / TagOcr,
    fields FindPixel => "findpixel" / TagFindPixel,
    fields ForEachRow => "foreachrow" / TagForEachRow,
    fields Wait => "wait" / TagWait,
    fields Pause => "pause" / TagPause,
    fields Move => "move" / TagMove,
    fields Click => "click" / TagClick,
    fields Key => "key" / TagKey,
    fields Type => "type" / TagType,
    fields SetVariable => "setvariable" / TagSetVariable,
    fields SaveVariable => "savevariable" / TagSaveVariable,
    fields FocusWindow => "focuswindow" / TagFocusWindow,
    fields RunMacro => "runmacro" / TagRunMacro,
    fields NavigateKey => "navigatekey" / TagNavigateKey,
    newtype NavigateSelect => "navigateselect" / TagNavigateSelect,
    unit Break => "break" / TagBreak,
    unit Continue => "continue" / TagContinue,
}

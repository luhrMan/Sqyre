//! Data Editor image-preview view state: zoom/pan per preview, manual refresh, mask grid.

use super::mask_preview::{MaskItemsFilter, MaskScrimCache};
use super::tabs::EditorTab;
use crate::image_view::ImageViewTransform;

/// Zoom/pan transform that resets when its identity key changes.
#[derive(Clone, Debug)]
pub(crate) struct KeyedPreview<K> {
    pub transform: ImageViewTransform,
    pub key: Option<K>,
}

impl<K> Default for KeyedPreview<K> {
    fn default() -> Self {
        Self {
            transform: ImageViewTransform::default(),
            key: None,
        }
    }
}

impl<K: PartialEq> KeyedPreview<K> {
    /// Reset the transform when `key` differs from the last synced value.
    pub(crate) fn sync_key(&mut self, key: K) {
        if self.key.as_ref() != Some(&key) {
            self.transform.reset();
            self.key = Some(key);
        }
    }

    pub(crate) fn clear(&mut self) {
        self.transform.reset();
        self.key = None;
    }
}

/// Preview view state owned by the Data Editor (session-only, never persisted).
#[derive(Default)]
pub(crate) struct PreviewState {
    /// Zoom/pan for the collections-tab image preview (keyed by program+collection).
    pub collection: KeyedPreview<(String, String)>,
    /// Zoom/pan for the atlas plane preview (keyed by program+atlas).
    pub atlas: KeyedPreview<(String, String)>,
    /// Zoom/pan for point / search-area / ScreenCap panels (keyed by tab+program+entity).
    pub coord: KeyedPreview<(EditorTab, String, String)>,
    /// One-frame `force` armed by the preview's floating refresh button.
    force: bool,
    /// Masks-tab items grid filter.
    pub mask_items_filter: MaskItemsFilter,
    /// Cached scrim texture for the Masks items preview.
    pub mask_scrim_cache: Option<MaskScrimCache>,
}

impl PreviewState {
    /// Request a hard refresh on the next preview paint.
    pub(crate) fn request_force(&mut self) {
        self.force = true;
    }

    /// Consume a pending hard-refresh request.
    pub(crate) fn take_force(&mut self) -> bool {
        std::mem::take(&mut self.force)
    }

    /// True while the user is panning the coordinate preview.
    pub(crate) fn coord_panning(&self) -> bool {
        self.coord.transform.pan_drag.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_view::IMAGE_ZOOM_DEFAULT;

    #[test]
    fn keyed_preview_resets_transform_only_when_key_changes() {
        let mut preview = KeyedPreview::<(String, String)>::default();
        preview.transform.zoom = 2.0;
        preview.sync_key(("a".into(), "b".into()));
        assert_eq!(
            preview.key.as_ref().map(|(a, b)| (a.as_str(), b.as_str())),
            Some(("a", "b"))
        );
        // First sync always resets.
        assert!((preview.transform.zoom - IMAGE_ZOOM_DEFAULT).abs() < f32::EPSILON);

        preview.transform.zoom = 3.0;
        preview.sync_key(("a".into(), "b".into()));
        assert!((preview.transform.zoom - 3.0).abs() < f32::EPSILON);

        preview.sync_key(("a".into(), "c".into()));
        assert!((preview.transform.zoom - IMAGE_ZOOM_DEFAULT).abs() < f32::EPSILON);
        preview.clear();
        assert!(preview.key.is_none());
    }

    #[test]
    fn force_is_one_shot() {
        let mut p = PreviewState::default();
        assert!(!p.take_force());
        p.request_force();
        assert!(p.take_force());
        assert!(!p.take_force());
    }

    #[test]
    fn previews_are_independent() {
        let mut p = PreviewState::default();
        p.coord.transform.zoom = 3.0;
        p.collection.sync_key(("prog".into(), "col".into()));
        assert!((p.coord.transform.zoom - 3.0).abs() < f32::EPSILON);
        p.coord.clear();
        assert!((p.coord.transform.zoom - IMAGE_ZOOM_DEFAULT).abs() < f32::EPSILON);
        assert!(p.collection.key.is_some());
    }
}

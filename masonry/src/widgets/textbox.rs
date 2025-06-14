// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use smallvec::{SmallVec, smallvec};
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Insets, Point, Rect, Size};

use crate::core::{
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, PaintCtx, PointerEvent,
    PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, TextEvent, Update, UpdateCtx, Widget,
    WidgetId, WidgetMut, WidgetPod,
};
use crate::peniko::Color;
use crate::properties::Padding;
use crate::util::stroke;
use crate::widgets::TextArea;

// TODO - Replace with Padding property.
/// Added padding between each horizontal edge of the widget
/// and the text in logical pixels.
///
/// This makes it so that the surrounding box isn't crowding out the text.
const TEXTBOX_PADDING: Padding = Padding::all(5.0);

/// The margin added around textboxes to allow the boundaries to be visible inside the window edge.
const TEXTBOX_MARGIN: Padding = Padding::horizontal(2.0);

/// The textbox widget displays text which can be edited by the user,
/// inside a surrounding box.
///
/// This currently does not support newlines entered by the user,
/// although pre-existing newlines are handled correctly.
///
/// This widget itself does not emit any actions.
/// However, the child widget will do so, as it is user editable.
/// The ID of the child can be accessed using [`area_pod`](Self::area_pod).
///
/// At runtime, most properties of the text will be set using [`text_mut`](Self::text_mut).
/// This is because `Textbox` largely serves as a wrapper around a [`TextArea`].
pub struct Textbox {
    text: WidgetPod<TextArea<true>>,

    /// Whether to clip the contained text.
    clip: bool,
}

impl Textbox {
    /// Create a new `Textbox` with the given text.
    ///
    /// To use non-default text properties, use [`from_text_area`](Self::from_text_area) instead.
    pub fn new(text: &str) -> Self {
        Self::from_text_area(TextArea::new_editable(text))
    }

    /// Create a new `Textbox` from a styled text area.
    pub fn from_text_area(text: TextArea<true>) -> Self {
        Self {
            text: WidgetPod::new(text),
            clip: false,
        }
    }

    /// Create a new `Textbox` from a styled text area in a [`WidgetPod`].
    ///
    /// Note that the default padding used for textbox will not apply.
    pub fn from_text_area_pod(text: WidgetPod<TextArea<true>>) -> Self {
        Self { text, clip: false }
    }

    /// Whether to clip the text to the drawn boundaries.
    ///
    /// If this is set to true, it is recommended, but not required, that this
    /// wraps a text area with [word wrapping](TextArea::with_word_wrap) enabled.
    ///
    /// To modify this on active textbox, use [`set_clip`](Self::set_clip).
    pub fn with_clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    /// Read the underlying text area.
    ///
    /// Useful for getting its ID, as most actions from the textbox will be sent by the child.
    pub fn area_pod(&self) -> &WidgetPod<TextArea<true>> {
        &self.text
    }
}

// --- MARK: WIDGETMUT
impl Textbox {
    /// Edit the underlying text area.
    ///
    /// Used to modify most properties of the text.
    pub fn text_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, TextArea<true>> {
        this.ctx.get_mut(&mut this.widget.text)
    }

    /// Whether to clip the text to the drawn boundaries.
    ///
    /// If this is set to true, it is recommended, but not required, that this
    /// wraps a text area with [word wrapping](TextArea::set_word_wrap) enabled.
    ///
    /// The runtime requivalent of [`with_clip`](Self::with_clip).
    pub fn set_clip(this: &mut WidgetMut<'_, Self>, clip: bool) {
        this.widget.clip = clip;
        this.ctx.request_layout();
    }
}

// --- MARK: IMPL WIDGET
impl Widget for Textbox {
    fn on_pointer_event(
        &mut self,
        _: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _: &PointerEvent,
    ) {
    }

    fn on_text_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }

    fn on_access_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.text);
    }

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &Update,
    ) {
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let margin = TEXTBOX_MARGIN;
        let padding = TEXTBOX_PADDING;
        // Shrink constraints by padding inset
        let margin_size = Size::new(margin.left + margin.left, margin.top + margin.bottom);
        let padding_size = Size::new(padding.left + padding.left, padding.top + padding.bottom);
        let child_bc = bc.shrink(margin_size);
        let child_bc = child_bc.shrink(padding_size);
        // TODO: Set minimum to deal with alignment
        let size = ctx.run_layout(&mut self.text, &child_bc);
        ctx.place_child(
            &mut self.text,
            Point::new(margin.left + padding.left, margin.top + padding.top),
        );
        if self.clip {
            ctx.set_clip_path(Rect::from_origin_size(Point::ORIGIN, size));
        }
        size + margin_size + padding_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let border_width = 1.0;
        let outline_rect = size.to_rect().inset(Insets::new(
            -TEXTBOX_MARGIN.left - border_width / 2.,
            -TEXTBOX_MARGIN.top - border_width / 2.,
            -TEXTBOX_MARGIN.left - border_width / 2.,
            -TEXTBOX_MARGIN.bottom - border_width / 2.,
        ));
        stroke(scene, &outline_rect, Color::WHITE, border_width);
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![self.text.id()]
    }

    fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> Span {
        trace_span!("Prose", id = ctx.widget_id().trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        self.clip.then(|| "(clip)".into())
    }
}

// TODO - Add more tests
#[cfg(test)]
mod tests {
    use vello::kurbo::Size;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::core::StyleProperty;
    use crate::testing::TestHarness;
    use crate::theme::default_property_set;
    use crate::widgets::TextArea;

    #[test]
    fn textbox_outline() {
        let textbox = Textbox::from_text_area(
            TextArea::new_editable("Textbox contents").with_style(StyleProperty::FontSize(14.0)),
        );
        let mut harness =
            TestHarness::create_with_size(default_property_set(), textbox, Size::new(150.0, 40.0));

        assert_render_snapshot!(harness, "textbox_outline");

        let mut text_area_id = None;
        harness.edit_root_widget(|mut textbox| {
            let mut textbox = textbox.downcast::<Textbox>();
            let mut textbox = Textbox::text_mut(&mut textbox);
            text_area_id = Some(textbox.ctx.widget_id());

            TextArea::select_text(&mut textbox, "contents");
        });
        harness.focus_on(text_area_id);

        assert_render_snapshot!(harness, "textbox_selection");
    }
}

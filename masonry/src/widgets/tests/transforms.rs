// Copyright 2025 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Tests related to transforms.

use std::f64::consts::PI;

use vello::kurbo::{Affine, Vec2};
use vello::peniko::color::palette;

use crate::assert_render_snapshot;
use crate::core::{PointerButton, Widget, WidgetOptions, WidgetPod};
use crate::testing::TestHarness;
use crate::theme::default_property_set;
use crate::widgets::{Alignment, Button, ChildAlignment, Label, SizedBox, ZStack};

fn blue_box(inner: impl Widget) -> Box<SizedBox> {
    Box::new(
        SizedBox::new(inner)
            .width(200.)
            .height(100.)
            .background(palette::css::BLUE)
            .border(palette::css::TEAL, 2.),
    )
}

#[test]
fn transforms_translation_rotation() {
    let translation = Vec2::new(100.0, 50.0);
    let transformed_widget = WidgetPod::new_with_options(
        blue_box(Label::new("Background")),
        // Currently there's no support for changing the transform-origin, which is currently at the top left.
        // This rotates around the center of the widget
        WidgetOptions {
            transform: Affine::translate(-translation)
                .then_rotate(PI * 0.25)
                .then_translate(translation),
            ..Default::default()
        },
    )
    .erased();
    let widget = ZStack::new().with_child_pod(transformed_widget, ChildAlignment::ParentAligned);

    let mut harness = TestHarness::create(default_property_set(), widget);
    assert_render_snapshot!(harness, "transforms_translation_rotation");
}

#[test]
fn transforms_pointer_events() {
    let transformed_widget = WidgetPod::new_with_options(
        blue_box(
            ZStack::new().with_child(Button::new("Should be pressed"), Alignment::BottomRight),
        ),
        WidgetOptions {
            transform: Affine::rotate(PI * 0.125).then_translate(Vec2::new(100.0, 50.0)),
            ..Default::default()
        },
    )
    .erased();
    let widget = ZStack::new().with_child_pod(transformed_widget, ChildAlignment::ParentAligned);

    let mut harness = TestHarness::create(default_property_set(), widget);
    harness.mouse_move((335.0, 350.0)); // Should hit the last "d" of the button text
    harness.mouse_button_press(PointerButton::Primary);
    assert_render_snapshot!(harness, "transforms_pointer_events");
}

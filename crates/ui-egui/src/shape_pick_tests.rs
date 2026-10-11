//! Path Selection and Direct Selection with "Select: All Layers", as in Photoshop: a click on any
//! shape selects its layer, so it can be moved or reshaped without going to the Layers panel.

use egui::Modifiers;
use photocraft_doc::{LayerContent, LayerId};
use serde_json::json;

use crate::PhotocraftApp;
use crate::canvas::{ToolEvent, tool_event};
use crate::state::Tool;

fn app(tool: Tool) -> PhotocraftApp {
    let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
    app.session.execute("file.new", json!({"width": 200, "height": 200})).unwrap();
    app.sync_views();
    app.ui.tool = tool;
    app
}

fn shape(app: &mut PhotocraftApp, rect: [i32; 4]) -> LayerId {
    LayerId(app.run("shape.create", json!({"kind": "rect", "rect": rect, "fill": "#ff0000"})).unwrap()["layer"].as_u64().unwrap())
}

/// The shape's path bounds (x0, y0, x1, y1), rounded.
fn bounds(app: &PhotocraftApp, id: LayerId) -> [i64; 4] {
    let l = app.session.active().unwrap().doc.layer(id).unwrap();
    let LayerContent::Shape(sh) = &l.content else { panic!("not a shape layer") };
    let (x0, y0, x1, y1) = sh.path.control_bounds().unwrap();
    [x0, y0, x1, y1].map(|v| v.round() as i64)
}

fn active(app: &PhotocraftApp) -> Option<LayerId> {
    app.session.active().unwrap().active_layer
}

fn drag(app: &mut PhotocraftApp, from: [f64; 2], to: [f64; 2]) {
    let m = Modifiers::NONE;
    tool_event(app, ToolEvent::Down { x: from[0], y: from[1], pressure: 1.0 }, m);
    let mid = [(from[0] + to[0]) / 2.0, (from[1] + to[1]) / 2.0];
    tool_event(app, ToolEvent::Move { x: mid[0], y: mid[1], pressure: 1.0 }, m);
    tool_event(app, ToolEvent::Move { x: to[0], y: to[1], pressure: 1.0 }, m);
    tool_event(app, ToolEvent::Up { x: to[0], y: to[1] }, m);
}

#[test]
fn path_selection_all_layers_moves_the_clicked_shape() {
    let mut app = app(Tool::PathSelection);
    let a = shape(&mut app, [10, 10, 40, 40]);
    let b = shape(&mut app, [120, 120, 40, 40]);
    assert_eq!(active(&app), Some(b));
    app.ui.tool_options.path_select_all = true;
    drag(&mut app, [30.0, 30.0], [40.0, 50.0]);
    assert_eq!(active(&app), Some(a), "the click selected the shape under it");
    assert_eq!(bounds(&app, a), [20, 30, 60, 70]);
    assert_eq!(bounds(&app, b), [120, 120, 160, 160], "the other shape stays put");
}

#[test]
fn path_selection_active_layers_moves_the_active_shape() {
    let mut app = app(Tool::PathSelection);
    let a = shape(&mut app, [10, 10, 40, 40]);
    let b = shape(&mut app, [120, 120, 40, 40]);
    assert!(app.ui.tool_options.path_select_all, "All Layers is this build's default");
    app.ui.tool_options.path_select_all = false;
    drag(&mut app, [30.0, 30.0], [40.0, 30.0]);
    assert_eq!(active(&app), Some(b));
    assert_eq!(bounds(&app, a), [10, 10, 50, 50]);
    assert_eq!(bounds(&app, b), [130, 120, 170, 160], "the drag moved the active shape");
}

#[test]
fn path_selection_drag_from_empty_canvas_moves_nothing_and_selects_no_layer() {
    let mut app = app(Tool::PathSelection);
    let b = shape(&mut app, [120, 120, 40, 40]);
    app.ui.tool_options.path_select_all = true;
    drag(&mut app, [5.0, 5.0], [25.0, 25.0]);
    assert_eq!(active(&app), Some(b), "the layer selection stays");
    assert_eq!(bounds(&app, b), [120, 120, 160, 160], "a press that hit no shape drags nothing");
}

#[test]
fn direct_selection_all_layers_reshapes_another_shape() {
    let mut app = app(Tool::DirectSelection);
    let a = shape(&mut app, [10, 10, 40, 40]);
    let b = shape(&mut app, [120, 120, 40, 40]);
    app.ui.tool_options.path_select_all = true;
    // A's bottom-right anchor at (50, 50) is dragged out to (70, 60).
    drag(&mut app, [50.0, 50.0], [70.0, 60.0]);
    assert_eq!(active(&app), Some(a));
    assert_eq!(bounds(&app, a), [10, 10, 70, 60]);
    assert_eq!(bounds(&app, b), [120, 120, 160, 160]);
}

#[test]
fn direct_selection_prefers_the_shown_path_over_a_shape_on_top() {
    let mut app = app(Tool::DirectSelection);
    let below = shape(&mut app, [10, 10, 50, 50]);
    let top = shape(&mut app, [40, 40, 50, 50]);
    app.run("layer.select", json!({"layer": below.0})).unwrap();
    app.ui.tool_options.path_select_all = true;
    // `below`'s corner at (60, 60) lies inside `top`'s fill: the shown path still takes the press.
    drag(&mut app, [60.0, 60.0], [70.0, 70.0]);
    assert_eq!(active(&app), Some(below));
    assert_eq!(bounds(&app, below), [10, 10, 70, 70]);
    assert_eq!(bounds(&app, top), [40, 40, 90, 90]);
}

#[test]
fn reset_tool_restores_all_layers() {
    let mut app = app(Tool::PathSelection);
    app.ui.tool_options.path_select_all = false;
    crate::tool_reset::reset_options(&mut app.ui.tool_options, Tool::DirectSelection);
    assert!(app.ui.tool_options.path_select_all);
}

fn selected(app: &PhotocraftApp) -> Vec<LayerId> {
    let mut v = app.session.active().unwrap().selected_layers();
    v.sort_by_key(|l| l.0);
    v
}

fn drag_with(app: &mut PhotocraftApp, from: [f64; 2], to: [f64; 2], m: Modifiers) {
    tool_event(app, ToolEvent::Down { x: from[0], y: from[1], pressure: 1.0 }, m);
    tool_event(app, ToolEvent::Move { x: to[0], y: to[1], pressure: 1.0 }, m);
    tool_event(app, ToolEvent::Up { x: to[0], y: to[1] }, m);
}

#[test]
fn marquee_from_empty_canvas_selects_shapes_only_and_they_move_together() {
    let mut app = app(Tool::PathSelection);
    let a = shape(&mut app, [10, 10, 30, 30]);
    let b = shape(&mut app, [60, 10, 30, 30]);
    let far = shape(&mut app, [150, 150, 30, 30]);
    // A pixel layer inside the box is left out.
    app.run("layer.new.layer", json!({"name": "Paint"})).unwrap();
    app.run("select.rect", json!({"x": 20, "y": 50, "width": 10, "height": 10})).unwrap();
    app.run("edit.fill", json!({"color": "#00ff00"})).unwrap();
    app.run("select.deselect", json!({})).unwrap();
    drag_with(&mut app, [5.0, 5.0], [100.0, 70.0], Modifiers::NONE);
    let mut want = vec![a, b];
    want.sort_by_key(|l| l.0);
    assert_eq!(selected(&app), want);
    // A press on one of them keeps both selected, and the drag moves both in one step.
    drag(&mut app, [20.0, 20.0], [25.0, 30.0]);
    assert_eq!(selected(&app), want);
    assert_eq!(bounds(&app, a), [15, 20, 45, 50]);
    assert_eq!(bounds(&app, b), [65, 20, 95, 50]);
    assert_eq!(bounds(&app, far), [150, 150, 180, 180]);
}

#[test]
fn shift_click_adds_a_shape() {
    let mut app = app(Tool::PathSelection);
    let a = shape(&mut app, [10, 10, 30, 30]);
    let b = shape(&mut app, [60, 10, 30, 30]);
    drag(&mut app, [20.0, 20.0], [20.0, 20.0]);
    assert_eq!(selected(&app), vec![a]);
    drag_with(&mut app, [70.0, 20.0], [70.0, 20.0], Modifiers { shift: true, ..Default::default() });
    let mut want = vec![a, b];
    want.sort_by_key(|l| l.0);
    assert_eq!(selected(&app), want);
}

#[test]
fn double_click_edits_points_and_esc_returns() {
    let mut app = app(Tool::PathSelection);
    let a = shape(&mut app, [10, 10, 30, 30]);
    shape(&mut app, [60, 10, 30, 30]);
    crate::vector_ui::enter_point_edit(&mut app, [20.0, 20.0]);
    assert_eq!(app.ui.tool, Tool::DirectSelection);
    assert_eq!(active(&app), Some(a));
    assert_eq!(app.ui.direct_selection.target, Some(crate::direct_select::PathRef::Layer(a.0)));
    assert!(app.ui.point_edit_from_path_select);
    crate::vector_ui::leave_point_edit(&mut app);
    assert_eq!(app.ui.tool, Tool::PathSelection);
    assert_eq!(active(&app), Some(a));
    // A double-click on empty canvas stays in Path Selection.
    crate::vector_ui::enter_point_edit(&mut app, [150.0, 150.0]);
    assert_eq!(app.ui.tool, Tool::PathSelection);
}

//! Fireworks-style point editing through the real canvas: a double-click on a shape with Path
//! Selection switches to Direct Selection on that shape, and Esc comes back.

use egui::{Key, Modifiers, PointerButton, Pos2, vec2};
use egui_kittest::Harness;
use photocraft_doc::LayerId;
use serde_json::json;

use crate::PhotocraftApp;
use crate::canvas::ViewXform;
use crate::state::Tool;

fn harness() -> (Harness<'static, PhotocraftApp>, LayerId) {
    let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
    app.run("file.new", json!({"width": 400, "height": 300})).unwrap();
    let a = LayerId(app.run("shape.create", json!({"kind": "rect", "rect": [50, 50, 100, 100], "fill": "#ff0000"})).unwrap()["layer"].as_u64().unwrap());
    app.run("shape.create", json!({"kind": "rect", "rect": [250, 50, 100, 100], "fill": "#0000ff"})).unwrap();
    app.sync_views();
    app.ui.extras.rulers = false;
    app.ui.tool = Tool::PathSelection;
    // 60 fps, so a double-click's two clicks fall inside egui's 0.3 s window.
    let mut h = Harness::builder().with_size(vec2(1000.0, 700.0)).with_step_dt(1.0 / 60.0).build_ui_state(
        |ui, app: &mut PhotocraftApp| {
            let ctx = ui.ctx().clone();
            if !ctx.fonts(|f| f.families().contains(&egui::FontFamily::Name("medium".into()))) {
                return;
            }
            crate::shortcuts::handle(app, &ctx);
            egui::CentralPanel::default().show(ui, |ui| crate::canvas::document_area(app, ui));
        },
        app,
    );
    PhotocraftApp::setup_context(&h.ctx, crate::theme::ThemeKind::ALL[0]);
    h.run_steps(4);
    let v = &mut h.state_mut().ui.views[0];
    v.zoom = 1.0;
    v.center = [200.0, 150.0];
    v.fit_pending = false;
    h.run_steps(2);
    (h, a)
}

fn screen(h: &Harness<'static, PhotocraftApp>, x: f32, y: f32) -> Pos2 {
    let app = h.state();
    let v = &app.ui.views[0];
    let xf = ViewXform {
        rect: crate::rulers::content_rect(app, app.last_canvas_rect),
        zoom: v.zoom,
        center: v.center,
        flip: app.ui.view.flip_horizontal,
        rotation: v.rotation,
        aspect: app.ui.view.display_aspect(),
    };
    xf.to_screen(x, y)
}

fn click(h: &mut Harness<'static, PhotocraftApp>, x: f32, y: f32) {
    let p = screen(h, x, y);
    h.event(egui::Event::PointerMoved(p));
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        h.run_steps(1);
    }
}

#[test]
fn double_click_a_shape_edits_its_points_and_esc_returns() {
    let (mut h, a) = harness();
    click(&mut h, 100.0, 100.0);
    click(&mut h, 100.0, 100.0);
    h.run_steps(2);
    assert_eq!(h.state().ui.tool, Tool::DirectSelection, "the double-click entered point editing");
    assert_eq!(h.state().session.active().unwrap().active_layer, Some(a));
    assert_eq!(h.state().ui.direct_selection.target, Some(crate::direct_select::PathRef::Layer(a.0)));
    h.event(egui::Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    h.event(egui::Event::Key { key: Key::Escape, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
    assert_eq!(h.state().ui.tool, Tool::PathSelection, "Esc went back to Path Selection");
    assert_eq!(h.state().session.active().unwrap().active_layer, Some(a), "the shape stays selected");
}

#[test]
fn single_clicks_stay_in_path_selection() {
    let (mut h, _) = harness();
    click(&mut h, 100.0, 100.0);
    h.run_steps(30);
    click(&mut h, 300.0, 100.0);
    h.run_steps(2);
    assert_eq!(h.state().ui.tool, Tool::PathSelection);
}

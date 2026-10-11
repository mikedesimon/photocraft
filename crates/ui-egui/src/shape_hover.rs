//! Shape rollover for Path Selection and Direct Selection with "Select: All Layers": hovering the
//! canvas outlines the shape a click there would pick (`pick_cmds::shape_at`, the click's own hit
//! test), as Fireworks did for every object. The answer is cached per document revision, pointer
//! position and zoom, so a resting pointer costs nothing per frame.

use egui::{Pos2, Stroke};
use photocraft_doc::{DocId, LayerContent, LayerId};

use crate::PhotocraftApp;
use crate::canvas::ViewXform;
use crate::state::Tool;

/// The last answer: the shape a click at `at` on that revision of the document would pick.
pub(crate) struct Hover {
    doc: DocId,
    revision: u64,
    at: [i64; 2],
    zoom: u32,
    layer: Option<LayerId>,
}

/// The shape layer to outline with the pointer at `at` (document px): `None` when another tool is
/// current, "Select: All Layers" is off, a button is down, a drag or transform runs, or the shape
/// there is already the active one (its path is drawn anyway).
pub(crate) fn target(app: &mut PhotocraftApp, at: Option<[f64; 2]>, pressed: bool) -> Option<LayerId> {
    if !matches!(app.ui.tool, Tool::PathSelection | Tool::DirectSelection)
        || !app.ui.tool_options.path_select_all
        || app.ui.vector_mask_target
        || pressed
        || app.drag.is_some()
        || app.layer_box.is_some()
        || app.ui.direct_selection.drag.is_some()
        || app.ui.transform.is_some()
    {
        return None;
    }
    let at = at.filter(|p| p[0].is_finite() && p[1].is_finite())?;
    let zoom = app.point_zoom().max(0.01);
    let tol = 6.0 / f64::from(zoom);
    // A quarter-pixel grid: fine enough for hairlines when zoomed in, coarse enough to cache.
    let key = [(at[0] * 4.0).floor() as i64, (at[1] * 4.0).floor() as i64];
    let st = app.session.active()?;
    let (doc, revision, active) = (st.doc.id, st.revision, st.active_layer);
    let layer = match app.shape_hover.as_ref().filter(|h| h.doc == doc && h.revision == revision && h.at == key && h.zoom == zoom.to_bits()) {
        Some(h) => h.layer,
        None => {
            let layer = photocraft_engine::pick_cmds::shape_at(&st.doc, at[0], at[1], tol);
            app.shape_hover = Some(Hover { doc, revision, at: key, zoom: zoom.to_bits(), layer });
            layer
        }
    };
    layer.filter(|id| Some(*id) != active)
}

/// Draws the rollover outline on the active document's canvas, a little heavier than the paths
/// the tools edit so it reads as "click to select".
pub(crate) fn draw(app: &mut PhotocraftApp, painter: &egui::Painter, xf: &ViewXform, idx: usize, hover: Option<Pos2>) {
    if app.session.active_index() != Some(idx) {
        return;
    }
    let pressed = painter.ctx().input(|i| i.pointer.any_down());
    let at = hover.map(|p| xf.to_doc(p));
    let Some(id) = target(app, at, pressed) else { return };
    let Some(path) = app.session.active().and_then(|st| st.doc.layer(id)).and_then(|l| match &l.content {
        LayerContent::Shape(sh) => Some(sh.path.clone()),
        _ => None,
    }) else {
        return;
    };
    let accent = crate::theme::Tokens::get(painter.ctx()).accent;
    let to_scr = |q: [f64; 2]| xf.to_screen(q[0] as f32, q[1] as f32);
    for (pts, closed) in crate::vector_ui::path_lines(&path, &to_scr) {
        let stroke = Stroke::new(2.0, accent);
        painter.add(if closed { egui::Shape::closed_line(pts, stroke) } else { egui::Shape::line(pts, stroke) });
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn rollover_outlines_the_shape_a_click_would_pick() {
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
        app.session.execute("file.new", json!({"width": 100, "height": 100})).unwrap();
        app.sync_views();
        let a = LayerId(
            app.session.execute("shape.create", json!({"kind": "rect", "rect": [10, 10, 30, 30], "fill": "#ff0000"})).unwrap()["layer"].as_u64().unwrap(),
        );
        let b = LayerId(
            app.session.execute("shape.create", json!({"kind": "rect", "rect": [60, 60, 30, 30], "fill": "#0000ff"})).unwrap()["layer"].as_u64().unwrap(),
        );
        app.ui.tool = Tool::PathSelection;
        app.ui.tool_options.path_select_all = true;
        assert_eq!(target(&mut app, Some([20.0, 20.0]), false), Some(a));
        // The active shape (b, created last) isn't outlined again; empty canvas outlines nothing.
        assert_eq!(target(&mut app, Some([70.0, 70.0]), false), None);
        assert_eq!(target(&mut app, Some([50.0, 5.0]), false), None);
        // Nothing while a button is down, with Active Layers, or with another tool.
        assert_eq!(target(&mut app, Some([20.0, 20.0]), true), None);
        app.ui.tool_options.path_select_all = false;
        assert_eq!(target(&mut app, Some([20.0, 20.0]), false), None);
        app.ui.tool_options.path_select_all = true;
        app.ui.tool = Tool::Brush;
        assert_eq!(target(&mut app, Some([20.0, 20.0]), false), None);
        app.ui.tool = Tool::DirectSelection;
        assert_eq!(target(&mut app, Some([20.0, 20.0]), false), Some(a));
        let _ = b;
    }
}

//! Crop mode driven with the mouse (#823): Picture Format ▸ Crop ▸ Crop, then the black crop
//! marks crop a side and dragging the picture moves it under the frame.

use deckcraft_engine::Session;
use egui::{Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use serde_json::json;

use crate::{Services, SlideApp};

fn drag(h: &mut Harness<'_, SlideApp>, from: Pos2, to: Pos2) {
    h.hover_at(from);
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: from, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    for i in 1..=8 {
        h.event(egui::Event::PointerMoved(from + (to - from) * (i as f32 / 8.0)));
        h.run_steps(1);
    }
    h.event(egui::Event::PointerButton { pos: to, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

#[test]
fn crop_a_picture_with_the_mouse() {
    // A 64 × 32 gradient, so the crop shows in a screenshot.
    let mut px = Vec::with_capacity(64 * 32 * 4);
    for y in 0..32u32 {
        for x in 0..64u32 {
            px.extend_from_slice(&[(x * 4) as u8, (y * 8) as u8, 160, 255]);
        }
    }
    let png = deckcraft_render::Image { width: 64, height: 32, pixels: px }.to_png();
    let mut s = Session::with_new();
    s.execute("slide.new", &json!({"layout": "blank"})).unwrap();
    let r = s
        .execute(
            "insert.picture",
            &json!({"data": deckcraft_engine::cmd::base64_encode(&png), "name": "g.png", "rect": [200.0, 150.0, 320.0, 160.0]}),
        )
        .unwrap();
    let id = deckcraft_model::ShapeId(r["id"].as_u64().unwrap() as u32);
    let app = SlideApp::new(s, Services::default());
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_step_dt(1.0 / 60.0).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(3);
    h.state_mut().run("picture.cropMode", json!({"on": true})).unwrap();
    h.run_steps(2);
    let (slide, k) = (h.state().slide_rect.unwrap(), h.state().canvas_scale);
    let at = |x: f32, y: f32| slide.min + egui::vec2(x * k, y * k);
    let state = |h: &Harness<'_, SlideApp>| {
        let st = h.state().session.active().unwrap();
        let sh = st.shape(id).unwrap();
        let deckcraft_model::ShapeKind::Picture { fill } = &sh.kind else { panic!("not a picture") };
        (sh.xfrm.unwrap(), fill.crop)
    };

    // The right-hand crop mark, 80 pt to the left: a quarter of the picture is cropped away.
    drag(&mut h, at(520.0, 230.0), at(440.0, 230.0));
    let (x, c) = state(&h);
    assert!((x.x - 200.0).abs() < 0.5 && (x.w - 240.0).abs() < 0.5, "{x:?}");
    assert!((c[2] - 0.25).abs() < 0.01 && c[0].abs() < 1e-9, "{c:?}");
    assert_eq!(h.state().session.tool.crop, Some(id), "still in crop mode");

    // Dragging the picture 40 pt left moves it under the frame.
    drag(&mut h, at(300.0, 230.0), at(260.0, 230.0));
    let (x2, c) = state(&h);
    assert_eq!(x2, x);
    assert!((c[0] - 0.125).abs() < 0.01 && (c[2] - 0.125).abs() < 0.01, "{c:?}");
    // `DECKCRAFT_SHOTS=<dir>` saves what crop mode looks like.
    if let Ok(dir) = std::env::var("DECKCRAFT_SHOTS")
        && let Ok(img) = h.render()
    {
        let _ = img.save(format!("{dir}/crop-mode.png"));
    }

    // Esc leaves crop mode and keeps the crop.
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert!(h.state().session.tool.crop.is_none());
    assert_eq!(state(&h).1, c);
}

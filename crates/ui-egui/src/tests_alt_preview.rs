//! ⌥-dragging a tone slider shows what it clips, and Detail ▸ Masking where sharpening applies,
//! instead of the photo (Lightroom's Option-drag previews).

use std::time::Duration;

use lightcraft_pipeline::Overlay;
use serde_json::json;

use crate::headless::Headless;
use crate::panels::detail::view_overlay;
use crate::state::RightPanel;
use crate::{LightcraftApp, Services};

const T: Duration = Duration::from_secs(20);
const SETTLE: Duration = Duration::from_secs(120);

/// The demo library in Detail with the Edit panel's Light section open.
fn edit_panel() -> Headless {
    let app = LightcraftApp::new(lightcraft_engine::Session::with_demo(), Services { png: None, ..Default::default() });
    let mut h = Headless::new(app, [1400.0, 900.0], 1.0);
    assert_eq!(h.request("ui.set", json!({"view": "detail", "right": "edit"}), T)["ok"], true);
    h.app.ui.open_sections = vec!["light".to_string()];
    h.settle(SETTLE);
    h
}

fn overlay(h: &Headless) -> Overlay {
    let id = h.app.session.active().expect("active photo");
    view_overlay(&h.app, &h.app.session.develop_of(id).unwrap_or_default())
}

/// Press on the slider's track and move along it, without letting go.
fn press_and_drag(h: &mut Headless, control: &str, modifiers: egui::Modifiers) {
    let id = format!("slider:{control}");
    let track = h.app.widgets.iter().find(|(w, _)| *w == id).map(|(_, r)| *r).unwrap_or_else(|| panic!("no widget {id}"));
    let from = track.center();
    h.app.synthetic.push(egui::Event::PointerMoved(from));
    h.app.synthetic.push(egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers });
    for step in 1..=6 {
        h.app.synthetic.push(egui::Event::PointerMoved(from + egui::vec2(6.0 * step as f32, 0.0)));
    }
    drain(h);
}

fn release(h: &mut Headless, control: &str, modifiers: egui::Modifiers) {
    let id = format!("slider:{control}");
    let pos = h.app.widgets.iter().find(|(w, _)| *w == id).map(|(_, r)| r.center() + egui::vec2(36.0, 0.0)).expect("the slider");
    h.app.synthetic.push(egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: false, modifiers });
    drain(h);
    // the modifiers go back up on the frame after the release
    h.step();
    h.step();
}

fn drain(h: &mut Headless) {
    for _ in 0..200 {
        if h.app.synthetic.is_empty() {
            break;
        }
        h.step();
    }
    assert!(h.app.synthetic.is_empty(), "the injected input was not consumed");
    h.step();
}

#[test]
fn option_dragging_whites_shows_highlight_clipping_until_release() {
    let mut h = edit_panel();
    assert_eq!(overlay(&h), Overlay::None);
    press_and_drag(&mut h, "light.whites", egui::Modifiers::ALT);
    assert_eq!(h.app.ui.dragging_control.as_deref(), Some("light.whites"), "the slider is being dragged");
    assert_eq!(overlay(&h), Overlay::ClipHighlights);
    release(&mut h, "light.whites", egui::Modifiers::ALT);
    assert!(h.app.ui.dragging_control.is_none());
    assert_eq!(overlay(&h), Overlay::None, "the photo is back once the slider is let go");
    let id = h.app.session.active().expect("active photo");
    assert!(h.app.session.develop_of(id).unwrap_or_default().light.whites > 0.0, "the drag still moved Whites");
    h.settle(SETTLE);
}

#[test]
fn dragging_without_option_keeps_showing_the_photo() {
    let mut h = edit_panel();
    press_and_drag(&mut h, "light.whites", egui::Modifiers::NONE);
    assert_eq!(h.app.ui.dragging_control.as_deref(), Some("light.whites"));
    assert_eq!(overlay(&h), Overlay::None);
    release(&mut h, "light.whites", egui::Modifiers::NONE);
    h.settle(SETTLE);
}

#[test]
fn each_slider_has_its_own_preview_and_the_others_have_none() {
    let mut h = edit_panel();
    h.app.ui.option_held = true;
    for (control, shown) in [
        ("light.exposure", Overlay::ClipHighlights),
        ("light.highlights", Overlay::ClipHighlights),
        ("light.whites", Overlay::ClipHighlights),
        ("light.shadows", Overlay::ClipShadows),
        ("light.blacks", Overlay::ClipShadows),
        ("detail.sharpenMasking", Overlay::SharpenMask),
        ("light.contrast", Overlay::None),
        ("detail.sharpenAmount", Overlay::None),
        // a mask's own Whites slider (Masking panel) is named without a section
        ("whites", Overlay::None),
    ] {
        assert!(control == "whites" || lightcraft_develop::controls::find(control).is_some(), "{control} is a develop control");
        h.app.ui.dragging_control = Some(control.to_string());
        assert_eq!(overlay(&h), shown, "{control}");
    }
    // only in the Edit panel, only with ⌥, never in fullscreen
    h.app.ui.dragging_control = Some("light.whites".to_string());
    h.app.ui.right = RightPanel::Masking;
    assert_eq!(overlay(&h), Overlay::None);
    h.app.ui.right = RightPanel::Edit;
    h.app.ui.option_held = false;
    assert_eq!(overlay(&h), Overlay::None);
    h.app.ui.option_held = true;
    h.app.ui.fullscreen = true;
    assert_eq!(overlay(&h), Overlay::None);
    h.app.ui.fullscreen = false;
    assert_eq!(overlay(&h), Overlay::ClipHighlights);
}

#[test]
fn clipping_previews_render_as_zoom_windows_and_the_sharpen_mask_does_not() {
    use crate::panels::detail::overlay_supports_window;
    assert!(overlay_supports_window(Overlay::ClipHighlights) && overlay_supports_window(Overlay::ClipShadows));
    assert!(!overlay_supports_window(Overlay::SharpenMask));
}

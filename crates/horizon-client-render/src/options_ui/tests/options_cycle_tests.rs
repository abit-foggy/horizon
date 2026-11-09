use super::*;

#[test]
fn options_cycle_button_cycles_forward_and_shift_click_cycles_backward_with_wrapping() {
    let atlas = atlas();
    let mut s = Settings::default();
    let mut ui = OptionsUi::default();
    ui.show(Page::Chat);
    let size = (480.0, 480.0);
    let (widgets, _, _, _) = ui.layout(&s, size.0, size.1);
    let button = widgets
        .iter()
        .find(|wd| {
            matches!(
                wd.kind,
                WKind::Button {
                    active: true,
                    act: Act::Cycle(Opt::ChatVisibility),
                    ..
                }
            )
        })
        .expect("ChatVisibility must be a cycle button");
    let (x, y) = (button.x + 1.0, button.y + 1.0);

    // Initial chat_visibility is 0
    assert_eq!(s.chat_visibility, 0);

    // 1. Feature without shift (normal click): cycles forward (+1)
    assert_eq!(
        ui.click(&mut s, &atlas, x, y, size, [854, 480], false, false),
        vec![Event::ButtonClick, Event::Changed]
    );
    assert_eq!(s.chat_visibility, 1);

    assert_eq!(
        ui.click(&mut s, &atlas, x, y, size, [854, 480], false, false),
        vec![Event::ButtonClick, Event::Changed]
    );
    assert_eq!(s.chat_visibility, 2);

    // 3. Wrap-around forward (at max value, normal click wraps to min / 0)
    assert_eq!(
        ui.click(&mut s, &atlas, x, y, size, [854, 480], false, false),
        vec![Event::ButtonClick, Event::Changed]
    );
    assert_eq!(s.chat_visibility, 0);

    // 4. Wrap-around backward (at min / 0, shift-click wraps to max)
    ui.shift = true;
    assert_eq!(
        ui.click(&mut s, &atlas, x, y, size, [854, 480], false, false),
        vec![Event::ButtonClick, Event::Changed]
    );
    assert_eq!(s.chat_visibility, 2);

    // 2. Feature with shift: cycles backward (-1)
    assert_eq!(
        ui.click(&mut s, &atlas, x, y, size, [854, 480], false, false),
        vec![Event::ButtonClick, Event::Changed]
    );
    assert_eq!(s.chat_visibility, 1);

    assert_eq!(
        ui.click(&mut s, &atlas, x, y, size, [854, 480], false, false),
        vec![Event::ButtonClick, Event::Changed]
    );
    assert_eq!(s.chat_visibility, 0);
    ui.shift = false;

    // Also test Opt::Particles on Page::Video
    ui.show(Page::Video);
    let (widgets, _, _, _) = ui.layout(&s, size.0, size.1);
    let button = widgets
        .iter()
        .find(|wd| {
            matches!(
                wd.kind,
                WKind::Button {
                    active: true,
                    act: Act::Cycle(Opt::Particles),
                    ..
                }
            )
        })
        .expect("Particles must be a cycle button");
    let (px, py) = (button.x + 1.0, button.y + 1.0);

    // Initial particles is 0
    assert_eq!(s.particles, 0);

    // Forward
    ui.click(&mut s, &atlas, px, py, size, [854, 480], false, false);
    assert_eq!(s.particles, 1);
    ui.click(&mut s, &atlas, px, py, size, [854, 480], false, false);
    assert_eq!(s.particles, 2);
    // Wrap forward
    ui.click(&mut s, &atlas, px, py, size, [854, 480], false, false);
    assert_eq!(s.particles, 0);

    // Wrap backward with shift
    ui.shift = true;
    ui.click(&mut s, &atlas, px, py, size, [854, 480], false, false);
    assert_eq!(s.particles, 2);
    // Backward with shift
    ui.click(&mut s, &atlas, px, py, size, [854, 480], false, false);
    assert_eq!(s.particles, 1);
    ui.click(&mut s, &atlas, px, py, size, [854, 480], false, false);
    assert_eq!(s.particles, 0);
    ui.shift = false;
}

use eframe::egui::{Color32, Sense, Stroke, Vec2, Widget};

pub struct Knob<'a> {
    min: u8,
    max: u8,
    value: &'a mut u8,
    hand_color: Option<Color32>,
    fill: Option<Color32>,
    stroke: Option<Stroke>,
}

impl<'a> Knob<'a> {
    pub const fn new(value: &'a mut u8, min: u8, max: u8) -> Self {
        Self {
            min,
            max,
            value,
            hand_color: None,
            fill: None,
            stroke: None,
        }
    }
}
impl Widget for Knob<'_> {
    // TODO: parameterize size, colors (and maybe step, its bit more complicated)

    // partially from https://github.com/obsqrbtz/egui_knob and https://codeberg.org/pintariching/egui_timepicker
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let (rect, mut responce) =
            ui.allocate_exact_size(Vec2::splat(40.), Sense::click_and_drag());
        // how many different values there are
        let parts = self.max - self.min + 1;
        // the angle (degrees) for each part
        let part_angle = 360. / f32::from(parts);
        if responce.dragged() || responce.clicked() {
            if let Some(new_value) = responce.interact_pointer_pos() {
                // inverse of the math for drawing the point (see below) from a value
                // since we are converting the point into a value
                let angle = ((new_value - rect.center()).angle().to_degrees() + 90.)
                    .rem_euclid(360.)
                    / part_angle;
                *self.value = angle.floor() as u8;

                responce.mark_changed();
            }
        }
        let visuals = ui.style().interact(&responce);
        ui.painter()
            .circle_filled(rect.center(), 20., self.fill.unwrap_or(visuals.bg_fill));
        ui.painter()
            .circle_stroke(rect.center(), 20., self.stroke.unwrap_or(visuals.fg_stroke));
        // the angle of the current value
        // how many rotations of the of the part angle
        // we subtract 90 at the end to get the first value to be at the top
        let angle = (part_angle * f32::from(*self.value)) - 90.;
        let pointer = rect.center() + Vec2::angled(angle.to_radians()) * 20.;
        let pointer1 = rect.center() + Vec2::angled(angle.to_radians()) * 19.;
        let mut stroke = visuals.fg_stroke;
        if let Some(color) = self.hand_color {
            stroke.color = color;
        }

        ui.painter().line_segment([rect.center(), pointer], stroke);
        ui.painter().circle_filled(
            pointer1,
            2.,
            self.hand_color.unwrap_or(visuals.fg_stroke.color),
        );
        responce
    }
}

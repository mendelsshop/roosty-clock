use std::ops::{Add, Sub};

use eframe::egui::{emath::Numeric, Color32, Sense, Stroke, Vec2, Widget};

pub struct Knob<'a, N> {
    min: N,
    max: N,
    value: &'a mut N,
    hand_color: Option<Color32>,
    fill: Option<Color32>,
    stroke: Option<Stroke>,
    radius: Option<f32>,
}

impl<'a, N> Knob<'a, N> {
    pub const fn new(value: &'a mut N, min: N, max: N) -> Self {
        Self {
            min,
            max,
            value,
            hand_color: None,
            fill: None,
            stroke: None,
            radius: None,
        }
    }
}
impl<N> Widget for Knob<'_, N>
where
    N: Sub<Output = N> + Add<Output = N> + Numeric,
    f32: From<N>,
{
    // TODO: maybe parameterize step, its bit complicated

    // partially from https://github.com/obsqrbtz/egui_knob and https://codeberg.org/pintariching/egui_timepicker
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let Self {
            min,
            max,
            value,
            hand_color,
            fill,
            stroke,
            radius: _,
        } = self;

        let desired_radius = self
            .radius
            .unwrap_or_else(|| ui.spacing().slider_width / 2.);
        let (rect, mut responce) =
            ui.allocate_exact_size(Vec2::splat(desired_radius * 2.), Sense::click_and_drag());
        // how many different values there are
        // the angle (degrees) for each part
        let part_angle = 360. / (f32::from(max - min) + 1.);
        if responce.dragged() || responce.clicked() {
            if let Some(new_value) = responce.interact_pointer_pos() {
                // inverse of the math for drawing the point (see below) from a value
                // since we are converting the point into a value
                let angle = ((new_value - rect.center()).angle().to_degrees() + 90.)
                    .rem_euclid(360.)
                    / part_angle;
                *value = N::from_f64(f64::from(angle.floor())) + min;

                responce.mark_changed();
            }
        }
        let visuals = ui.style().interact(&responce);
        ui.painter().circle_filled(
            rect.center(),
            desired_radius,
            fill.unwrap_or(visuals.bg_fill),
        );
        let border_stroke = stroke.unwrap_or(visuals.fg_stroke);
        ui.painter()
            .circle_stroke(rect.center(), desired_radius, border_stroke);
        // the angle of the current value
        // how many rotations of the of the part angle
        // we subtract 90 at the end to get the first value to be at the top
        let angle = (part_angle * f32::from(*value)) - 90.;
        let pointer = rect.center() + Vec2::angled(angle.to_radians()) * desired_radius;
        let mut hand_stroke = visuals.fg_stroke;
        if let Some(color) = hand_color {
            hand_stroke.color = color;
        }

        let pointer1 = rect.center()
            + Vec2::angled(angle.to_radians()) * border_stroke.width.mul_add(-2., desired_radius);
        ui.painter()
            .line_segment([rect.center(), pointer], hand_stroke);
        ui.painter().circle_filled(
            pointer1,
            hand_stroke.width * 2.,
            hand_color.unwrap_or(visuals.fg_stroke.color),
        );
        responce
    }
}

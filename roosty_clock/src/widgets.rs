use eframe::egui::{Align2, Color32, Sense, Stroke, TextStyle, Vec2, Widget};

pub struct Value<N> {
    pub value: N,
    pub show: bool,
}
pub struct Knob<'a, N, I> {
    value: &'a mut N,
    hand_color: Option<Color32>,
    fill: Option<Color32>,
    stroke: Option<Stroke>,
    radius: Option<f32>,
    values: I,
    index: usize,
}
// TODO: (custom) index trait instead of sticking to iterator

impl<'a, N: std::cmp::PartialEq, I: Iterator<Item = Value<N>> + Clone> Knob<'a, N, I> {
    pub fn new(value: &'a mut N, values: I) -> Self {
        Self {
            index: values.clone().position(|n| n.value == *value).unwrap(),
            value,
            hand_color: None,
            fill: None,
            stroke: None,
            radius: None,
            values,
        }
    }

    /// Set how big the knob should be relative to its radius
    pub const fn radius(mut self, radius: Option<f32>) -> Self {
        self.radius = radius;
        self
    }

    // set the outline color of the knob
    pub const fn stroke(mut self, stroke: Option<Stroke>) -> Self {
        self.stroke = stroke;
        self
    }

    // set the background color of the knob
    pub const fn fill(mut self, fill: Option<Color32>) -> Self {
        self.fill = fill;
        self
    }

    // set the hand color of the knob
    pub const fn hand_color(mut self, hand_color: Option<Color32>) -> Self {
        self.hand_color = hand_color;
        self
    }
}
impl<N, I> Widget for Knob<'_, N, I>
where
    N: ToString,
    I: Iterator<Item = Value<N>> + Clone,
{
    // TODO: maybe parameterize step, its bit complicated

    // partially from https://github.com/obsqrbtz/egui_knob and https://codeberg.org/pintariching/egui_timepicker
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let Self {
            value,
            hand_color,
            fill,
            stroke,
            radius,
            values,
            mut index,
        } = self;

        let desired_radius = radius.unwrap_or_else(|| ui.spacing().slider_width / 2.);
        let (rect, mut responce) =
            ui.allocate_exact_size(Vec2::splat(desired_radius * 2.), Sense::click_and_drag());
        // how many different values there are
        // the angle (degrees) for each part
        let part_angle = 360. / (values.clone().count() as f32);
        if (responce.dragged() || responce.clicked())
            && let Some(new_value) = responce.interact_pointer_pos()
        {
            // inverse of the math for drawing the point (see below) from the (index of a) value
            // since we are converting the point into (the index of a) value
            let angle = ((new_value - rect.center()).angle().to_degrees() + 90.).rem_euclid(360.)
                / part_angle;
            let n = angle.floor() as usize;
            index = n;
            *value = values.clone().nth(n).unwrap().value;

            responce.mark_changed();
        }
        let visuals = ui.style().interact(&responce);
        ui.painter().circle_filled(
            rect.center(),
            desired_radius,
            fill.unwrap_or(visuals.bg_fill),
        );
        let border_stroke = stroke.unwrap_or(visuals.fg_stroke);
        if true {
            for (i, Value { value, show: _ }) in
                values.enumerate().filter(|(_, Value { show, .. })| *show)
            {
                let angle = (part_angle * i as f32) - 90.;
                let pointer = rect.center()
                    + Vec2::angled(angle.to_radians())
                        * (desired_radius - ui.style().spacing.icon_width_inner);
                ui.painter().text(
                    pointer,
                    Align2::CENTER_CENTER,
                    value.to_string(),
                    TextStyle::Monospace.resolve(ui.style()),
                    ui.style().visuals.text_color(),
                );
            }
        }
        ui.painter()
            .circle_stroke(rect.center(), desired_radius, border_stroke);
        // the angle of (the index of) the current value
        // how many rotations of the of the part angle
        // we subtract 90 at the end to get the first value to be at the top
        let angle = (part_angle * index as f32) - 90.;
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

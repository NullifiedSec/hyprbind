//! Interactive Hyprland animation curve editors (bezier graph + spring preview).

use gtk4::cairo::{Context, LineCap, LineJoin};
use gtk4::gdk::RGBA;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, DrawingArea, DropDown, Entry, GestureDrag, Label, Orientation,
};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Cubic bezier control points (P1, P2). Endpoints are fixed at (0,0) and (1,1).
#[derive(Debug, Clone, Copy)]
pub struct BezierPoints {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl Default for BezierPoints {
    fn default() -> Self {
        Self {
            x0: 0.23,
            y0: 1.0,
            x1: 0.32,
            y1: 1.0,
        }
    }
}

impl BezierPoints {
    pub fn from_fields(fields: &std::collections::BTreeMap<String, Value>) -> Self {
        let pts = fields.get("points").and_then(Value::as_array);
        let mut out = Self::default();
        if let Some(arr) = pts {
            if let Some(p0) = arr.first().and_then(Value::as_array) {
                out.x0 = p0.first().and_then(Value::as_f64).unwrap_or(out.x0);
                out.y0 = p0.get(1).and_then(Value::as_f64).unwrap_or(out.y0);
            }
            if let Some(p1) = arr.get(1).and_then(Value::as_array) {
                out.x1 = p1.first().and_then(Value::as_f64).unwrap_or(out.x1);
                out.y1 = p1.get(1).and_then(Value::as_f64).unwrap_or(out.y1);
            } else if arr.len() >= 4 {
                // Flat form: x0,y0,x1,y1
                out.x0 = arr[0].as_f64().unwrap_or(out.x0);
                out.y0 = arr[1].as_f64().unwrap_or(out.y0);
                out.x1 = arr[2].as_f64().unwrap_or(out.x1);
                out.y1 = arr[3].as_f64().unwrap_or(out.y1);
            }
        }
        out
    }

    pub fn to_json(&self) -> Value {
        json!([[self.x0, self.y0], [self.x1, self.y1]])
    }

    pub fn label(&self) -> String {
        format!(
            "P1 ({:.2}, {:.2})  ·  P2 ({:.2}, {:.2})",
            self.x0, self.y0, self.x1, self.y1
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SpringParams {
    pub mass: f64,
    pub stiffness: f64,
    pub dampening: f64,
}

impl Default for SpringParams {
    fn default() -> Self {
        Self {
            mass: 1.0,
            stiffness: 71.2633,
            dampening: 15.8273644,
        }
    }
}

impl SpringParams {
    pub fn from_fields(fields: &std::collections::BTreeMap<String, Value>) -> Self {
        let d = Self::default();
        Self {
            mass: fields
                .get("mass")
                .and_then(Value::as_f64)
                .unwrap_or(d.mass)
                .max(0.01),
            stiffness: fields
                .get("stiffness")
                .and_then(Value::as_f64)
                .unwrap_or(d.stiffness)
                .max(0.01),
            dampening: fields
                .get("dampening")
                .and_then(Value::as_f64)
                .unwrap_or(d.dampening)
                .max(0.0),
        }
    }
}

const PRESETS: &[(&str, BezierPoints)] = &[
    (
        "easeOutQuint",
        BezierPoints {
            x0: 0.23,
            y0: 1.0,
            x1: 0.32,
            y1: 1.0,
        },
    ),
    (
        "linear",
        BezierPoints {
            x0: 0.0,
            y0: 0.0,
            x1: 1.0,
            y1: 1.0,
        },
    ),
    (
        "easeInOut",
        BezierPoints {
            x0: 0.42,
            y0: 0.0,
            x1: 0.58,
            y1: 1.0,
        },
    ),
    (
        "easeOut",
        BezierPoints {
            x0: 0.0,
            y0: 0.0,
            x1: 0.58,
            y1: 1.0,
        },
    ),
    (
        "easeIn",
        BezierPoints {
            x0: 0.42,
            y0: 0.0,
            x1: 1.0,
            y1: 1.0,
        },
    ),
    (
        "overshoot",
        BezierPoints {
            x0: 0.5,
            y0: 0.9,
            x1: 0.1,
            y1: 1.1,
        },
    ),
    (
        "quick",
        BezierPoints {
            x0: 0.15,
            y0: 0.0,
            x1: 0.1,
            y1: 1.0,
        },
    ),
    (
        "almostLinear",
        BezierPoints {
            x0: 0.5,
            y0: 0.5,
            x1: 0.75,
            y1: 1.0,
        },
    ),
];

/// Bezier graph with two draggable control handles.
pub struct BezierEditor {
    root: GtkBox,
    points: Rc<RefCell<BezierPoints>>,
    area: DrawingArea,
    coords: Label,
    x0: Entry,
    y0: Entry,
    x1: Entry,
    y1: Entry,
    suppress: Rc<Cell<bool>>,
}

impl BezierEditor {
    pub fn new(initial: BezierPoints) -> Self {
        let points = Rc::new(RefCell::new(initial));
        let suppress = Rc::new(Cell::new(false));

        let area = DrawingArea::builder()
            .content_width(420)
            .content_height(320)
            .hexpand(true)
            .vexpand(true)
            .css_classes(["curve-graph"])
            .build();
        area.set_draw_func({
            let points = Rc::clone(&points);
            move |_, cr, w, h| draw_bezier(cr, w as f64, h as f64, *points.borrow())
        });

        let coords = Label::builder()
            .label(&initial.label())
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label", "monospace", "caption"])
            .build();

        let x0 = Entry::builder().text(format!("{:.3}", initial.x0)).build();
        let y0 = Entry::builder().text(format!("{:.3}", initial.y0)).build();
        let x1 = Entry::builder().text(format!("{:.3}", initial.x1)).build();
        let y1 = Entry::builder().text(format!("{:.3}", initial.y1)).build();

        let preset_labels: Vec<&str> = PRESETS.iter().map(|(n, _)| *n).collect();
        let mut labels = vec!["Presets…"];
        labels.extend(preset_labels);
        let presets = DropDown::from_strings(&labels);

        let root = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .build();
        root.append(
            &Label::builder()
                .label("Bézier curve")
                .halign(gtk4::Align::Start)
                .css_classes(["heading"])
                .build(),
        );
        root.append(&area);
        root.append(&coords);

        let nums = GtkBox::new(Orientation::Horizontal, 8);
        nums.append(&num_field("x0", &x0));
        nums.append(&num_field("y0", &y0));
        nums.append(&num_field("x1", &x1));
        nums.append(&num_field("y1", &y1));
        root.append(&nums);
        root.append(&presets);
        root.append(
            &Label::builder()
                .label("Drag the teal handles. Endpoints stay at (0,0) and (1,1). Y may overshoot above 1.")
                .wrap(true)
                .css_classes(["dim-label", "caption"])
                .halign(gtk4::Align::Start)
                .build(),
        );

        let sync_entries = {
            let points = Rc::clone(&points);
            let x0 = x0.clone();
            let y0 = y0.clone();
            let x1 = x1.clone();
            let y1 = y1.clone();
            let coords = coords.clone();
            let suppress = Rc::clone(&suppress);
            let area = area.clone();
            Rc::new(move || {
                let p = *points.borrow();
                suppress.set(true);
                x0.set_text(&format!("{:.3}", p.x0));
                y0.set_text(&format!("{:.3}", p.y0));
                x1.set_text(&format!("{:.3}", p.x1));
                y1.set_text(&format!("{:.3}", p.y1));
                coords.set_text(&p.label());
                suppress.set(false);
                area.queue_draw();
            })
        };

        let apply_entries = {
            let points = Rc::clone(&points);
            let x0 = x0.clone();
            let y0 = y0.clone();
            let x1 = x1.clone();
            let y1 = y1.clone();
            let coords = coords.clone();
            let suppress = Rc::clone(&suppress);
            let area = area.clone();
            Rc::new(move || {
                if suppress.get() {
                    return;
                }
                let mut p = *points.borrow();
                if let Ok(v) = x0.text().parse::<f64>() {
                    p.x0 = v.clamp(0.0, 1.0);
                }
                if let Ok(v) = y0.text().parse::<f64>() {
                    p.y0 = v.clamp(-0.5, 2.0);
                }
                if let Ok(v) = x1.text().parse::<f64>() {
                    p.x1 = v.clamp(0.0, 1.0);
                }
                if let Ok(v) = y1.text().parse::<f64>() {
                    p.y1 = v.clamp(-0.5, 2.0);
                }
                *points.borrow_mut() = p;
                coords.set_text(&p.label());
                area.queue_draw();
            })
        };

        for e in [&x0, &y0, &x1, &y1] {
            e.connect_changed({
                let apply_entries = Rc::clone(&apply_entries);
                move |_| apply_entries()
            });
        }

        presets.connect_selected_notify({
            let points = Rc::clone(&points);
            let sync_entries = Rc::clone(&sync_entries);
            move |dd| {
                let i = dd.selected() as usize;
                if i == 0 || i > PRESETS.len() {
                    return;
                }
                *points.borrow_mut() = PRESETS[i - 1].1;
                sync_entries();
            }
        });

        // Drag handles
        let drag_target = Rc::new(Cell::new(0u8)); // 0=none, 1=P1, 2=P2
        let gesture = GestureDrag::new();
        gesture.set_button(1);
        gesture.connect_drag_begin({
            let points = Rc::clone(&points);
            let drag_target = Rc::clone(&drag_target);
            let area = area.clone();
            move |g, x, y| {
                let w = area.width() as f64;
                let h = area.height() as f64;
                let plot = PlotGeom::new(w, h);
                let p = *points.borrow();
                let (px0, py0) = plot.to_screen(p.x0, p.y0);
                let (px1, py1) = plot.to_screen(p.x1, p.y1);
                const HIT: f64 = 14.0;
                if (x - px0).hypot(y - py0) <= HIT {
                    drag_target.set(1);
                } else if (x - px1).hypot(y - py1) <= HIT {
                    drag_target.set(2);
                } else {
                    drag_target.set(0);
                    g.set_state(gtk4::EventSequenceState::Denied);
                }
            }
        });
        gesture.connect_drag_update({
            let points = Rc::clone(&points);
            let drag_target = Rc::clone(&drag_target);
            let area = area.clone();
            let sync_entries = Rc::clone(&sync_entries);
            move |g, dx, dy| {
                let t = drag_target.get();
                if t == 0 {
                    return;
                }
                let Some((ox, oy)) = g.start_point() else {
                    return;
                };
                let w = area.width() as f64;
                let h = area.height() as f64;
                let plot = PlotGeom::new(w, h);
                let (nx, ny) = plot.from_screen(ox + dx, oy + dy);
                let mut p = *points.borrow();
                if t == 1 {
                    p.x0 = nx.clamp(0.0, 1.0);
                    p.y0 = ny.clamp(-0.5, 2.0);
                } else {
                    p.x1 = nx.clamp(0.0, 1.0);
                    p.y1 = ny.clamp(-0.5, 2.0);
                }
                *points.borrow_mut() = p;
                sync_entries();
            }
        });
        area.add_controller(gesture);

        Self {
            root,
            points,
            area,
            coords,
            x0,
            y0,
            x1,
            y1,
            suppress,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.root
    }

    pub fn points(&self) -> BezierPoints {
        *self.points.borrow()
    }

    pub fn set_points(&self, p: BezierPoints) {
        *self.points.borrow_mut() = p;
        self.suppress.set(true);
        self.x0.set_text(&format!("{:.3}", p.x0));
        self.y0.set_text(&format!("{:.3}", p.y0));
        self.x1.set_text(&format!("{:.3}", p.x1));
        self.y1.set_text(&format!("{:.3}", p.y1));
        self.coords.set_text(&p.label());
        self.suppress.set(false);
        self.area.queue_draw();
    }

    pub fn connect_changed<F: Fn() + 'static>(&self, f: F) {
        let f = Rc::new(f);
        for e in [&self.x0, &self.y0, &self.x1, &self.y1] {
            e.connect_changed({
                let f = Rc::clone(&f);
                let suppress = Rc::clone(&self.suppress);
                move |_| {
                    if !suppress.get() {
                        f();
                    }
                }
            });
        }
    }
}

/// Spring mass/stiffness/dampening with time-response graph.
pub struct SpringEditor {
    root: GtkBox,
    params: Rc<RefCell<SpringParams>>,
    area: DrawingArea,
    mass: Entry,
    stiffness: Entry,
    dampening: Entry,
    suppress: Rc<Cell<bool>>,
}

impl SpringEditor {
    pub fn new(initial: SpringParams) -> Self {
        let params = Rc::new(RefCell::new(initial));
        let suppress = Rc::new(Cell::new(false));

        let area = DrawingArea::builder()
            .content_width(420)
            .content_height(220)
            .hexpand(true)
            .css_classes(["curve-graph"])
            .build();
        area.set_draw_func({
            let params = Rc::clone(&params);
            move |_, cr, w, h| draw_spring(cr, w as f64, h as f64, *params.borrow())
        });

        let mass = Entry::builder()
            .text(format!("{:.4}", initial.mass))
            .build();
        let stiffness = Entry::builder()
            .text(format!("{:.4}", initial.stiffness))
            .build();
        let dampening = Entry::builder()
            .text(format!("{:.4}", initial.dampening))
            .build();

        let root = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .build();
        root.append(
            &Label::builder()
                .label("Spring curve")
                .halign(gtk4::Align::Start)
                .css_classes(["heading"])
                .build(),
        );
        root.append(&area);
        let nums = GtkBox::new(Orientation::Horizontal, 8);
        nums.append(&num_field("mass", &mass));
        nums.append(&num_field("stiffness", &stiffness));
        nums.append(&num_field("dampening", &dampening));
        root.append(&nums);
        root.append(
            &Label::builder()
                .label("Keep mass ≈ 1. Higher stiffness = snappier; higher dampening = less bounce.")
                .wrap(true)
                .css_classes(["dim-label", "caption"])
                .halign(gtk4::Align::Start)
                .build(),
        );

        let apply = {
            let params = Rc::clone(&params);
            let mass = mass.clone();
            let stiffness = stiffness.clone();
            let dampening = dampening.clone();
            let suppress = Rc::clone(&suppress);
            let area = area.clone();
            Rc::new(move || {
                if suppress.get() {
                    return;
                }
                let mut p = *params.borrow();
                if let Ok(v) = mass.text().parse::<f64>() {
                    p.mass = v.max(0.01);
                }
                if let Ok(v) = stiffness.text().parse::<f64>() {
                    p.stiffness = v.max(0.01);
                }
                if let Ok(v) = dampening.text().parse::<f64>() {
                    p.dampening = v.max(0.0);
                }
                *params.borrow_mut() = p;
                area.queue_draw();
            })
        };
        for e in [&mass, &stiffness, &dampening] {
            e.connect_changed({
                let apply = Rc::clone(&apply);
                move |_| apply()
            });
        }

        Self {
            root,
            params,
            area,
            mass,
            stiffness,
            dampening,
            suppress,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.root
    }

    pub fn params(&self) -> SpringParams {
        *self.params.borrow()
    }

    pub fn set_params(&self, p: SpringParams) {
        *self.params.borrow_mut() = p;
        self.suppress.set(true);
        self.mass.set_text(&format!("{:.4}", p.mass));
        self.stiffness.set_text(&format!("{:.4}", p.stiffness));
        self.dampening.set_text(&format!("{:.4}", p.dampening));
        self.suppress.set(false);
        self.area.queue_draw();
    }

    pub fn connect_changed<F: Fn() + 'static>(&self, f: F) {
        let f = Rc::new(f);
        for e in [&self.mass, &self.stiffness, &self.dampening] {
            e.connect_changed({
                let f = Rc::clone(&f);
                let suppress = Rc::clone(&self.suppress);
                move |_| {
                    if !suppress.get() {
                        f();
                    }
                }
            });
        }
    }
}

/// Style suggestions keyed by animation leaf family.
pub fn style_suggestions_for_leaf(leaf: &str) -> &'static [&'static str] {
    let l = leaf.to_lowercase();
    if l.starts_with("windows") {
        &[
            "",
            "slide",
            "slide left",
            "slide right",
            "slide top",
            "slide bottom",
            "popin",
            "popin 80%",
            "popin 87%",
            "gnomed",
        ]
    } else if l.starts_with("layers") {
        &[
            "",
            "slide",
            "slide left",
            "slide right",
            "slide top",
            "slide bottom",
            "popin",
            "popin 80%",
            "fade",
        ]
    } else if l.starts_with("workspaces") || l.starts_with("specialworkspace") {
        &[
            "",
            "slide",
            "slidevert",
            "slidefade",
            "slidefadevert",
            "fade",
        ]
    } else if l.contains("borderangle") {
        &["", "loop", "once"]
    } else {
        &["", "slide", "popin", "fade"]
    }
}

pub fn common_animation_leaves() -> &'static [&'static str] {
    &[
        "global",
        "windows",
        "windowsIn",
        "windowsOut",
        "windowsMove",
        "layers",
        "layersIn",
        "layersOut",
        "fade",
        "fadeIn",
        "fadeOut",
        "fadeSwitch",
        "fadeShadow",
        "fadeDim",
        "border",
        "borderangle",
        "workspaces",
        "workspacesIn",
        "workspacesOut",
        "specialWorkspace",
        "specialWorkspaceIn",
        "specialWorkspaceOut",
        "zoomFactor",
    ]
}

fn num_field(title: &str, entry: &Entry) -> GtkBox {
    let b = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    b.append(
        &Label::builder()
            .label(title)
            .halign(gtk4::Align::Start)
            .css_classes(["caption"])
            .build(),
    );
    b.append(entry);
    b
}

struct PlotGeom {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    y_min: f64,
    y_max: f64,
}

impl PlotGeom {
    fn new(w: f64, h: f64) -> Self {
        let pad = 28.0;
        Self {
            left: pad,
            top: pad,
            width: (w - pad * 2.0).max(10.0),
            height: (h - pad * 2.0).max(10.0),
            y_min: -0.2,
            y_max: 1.4,
        }
    }

    fn to_screen(&self, x: f64, y: f64) -> (f64, f64) {
        let nx = self.left + x.clamp(0.0, 1.0) * self.width;
        let t = (y - self.y_min) / (self.y_max - self.y_min);
        let ny = self.top + (1.0 - t) * self.height;
        (nx, ny)
    }

    fn from_screen(&self, sx: f64, sy: f64) -> (f64, f64) {
        let x = ((sx - self.left) / self.width).clamp(0.0, 1.0);
        let t = 1.0 - ((sy - self.top) / self.height);
        let y = self.y_min + t * (self.y_max - self.y_min);
        (x, y)
    }
}

fn draw_bezier(cr: &Context, w: f64, h: f64, p: BezierPoints) {
    // Background
    cr.set_source_rgba(0.08, 0.10, 0.12, 1.0);
    cr.rectangle(0.0, 0.0, w, h);
    let _ = cr.fill();

    let plot = PlotGeom::new(w, h);

    // Grid
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.06);
    cr.set_line_width(1.0);
    for i in 0..=4 {
        let t = i as f64 / 4.0;
        let (x, _) = plot.to_screen(t, 0.0);
        cr.move_to(x, plot.top);
        cr.line_to(x, plot.top + plot.height);
        let (_, y) = plot.to_screen(0.0, plot.y_min + t * (plot.y_max - plot.y_min));
        cr.move_to(plot.left, y);
        cr.line_to(plot.left + plot.width, y);
    }
    let _ = cr.stroke();

    // Unit square border
    let (x0, y0) = plot.to_screen(0.0, 0.0);
    let (x1, y1) = plot.to_screen(1.0, 1.0);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.18);
    cr.set_line_width(1.0);
    cr.rectangle(x0, y1, x1 - x0, y0 - y1);
    let _ = cr.stroke();

    // Control lines
    cr.set_source_rgba(0.35, 0.75, 0.72, 0.45);
    cr.set_line_width(1.5);
    cr.set_dash(&[4.0, 4.0], 0.0);
    let (c1x, c1y) = plot.to_screen(p.x0, p.y0);
    let (c2x, c2y) = plot.to_screen(p.x1, p.y1);
    cr.move_to(x0, y0);
    cr.line_to(c1x, c1y);
    cr.move_to(x1, y1);
    cr.line_to(c2x, c2y);
    let _ = cr.stroke();
    cr.set_dash(&[], 0.0);

    // Curve
    cr.set_source_rgb(0.95, 0.55, 0.25);
    cr.set_line_width(2.5);
    cr.set_line_cap(LineCap::Round);
    cr.set_line_join(LineJoin::Round);
    let steps = 80;
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let (bx, by) = cubic_bezier(t, p);
        let (sx, sy) = plot.to_screen(bx, by);
        if i == 0 {
            cr.move_to(sx, sy);
        } else {
            cr.line_to(sx, sy);
        }
    }
    let _ = cr.stroke();

    // Endpoints
    draw_dot(cr, x0, y0, 5.0, &RGBA::new(0.7, 0.7, 0.75, 1.0));
    draw_dot(cr, x1, y1, 5.0, &RGBA::new(0.7, 0.7, 0.75, 1.0));
    // Handles
    draw_dot(cr, c1x, c1y, 8.0, &RGBA::new(0.25, 0.85, 0.78, 1.0));
    draw_dot(cr, c2x, c2y, 8.0, &RGBA::new(0.25, 0.85, 0.78, 1.0));
}

fn cubic_bezier(t: f64, p: BezierPoints) -> (f64, f64) {
    let u = 1.0 - t;
    let uu = u * u;
    let tt = t * t;
    let ttt = tt * t;
    // P0=(0,0) P3=(1,1)
    let x = 3.0 * uu * t * p.x0 + 3.0 * u * tt * p.x1 + ttt;
    let y = 3.0 * uu * t * p.y0 + 3.0 * u * tt * p.y1 + ttt;
    (x, y)
}

fn draw_spring(cr: &Context, w: f64, h: f64, p: SpringParams) {
    cr.set_source_rgba(0.08, 0.10, 0.12, 1.0);
    cr.rectangle(0.0, 0.0, w, h);
    let _ = cr.fill();

    let pad = 28.0;
    let left = pad;
    let top = pad;
    let width = (w - pad * 2.0).max(10.0);
    let height = (h - pad * 2.0).max(10.0);
    let y_min = -0.3;
    let y_max = 1.5;
    let duration = spring_settling_time(&p).clamp(0.4, 3.0);

    // Grid
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.06);
    cr.set_line_width(1.0);
    for i in 0..=4 {
        let t = i as f64 / 4.0;
        cr.move_to(left + t * width, top);
        cr.line_to(left + t * width, top + height);
        cr.move_to(left, top + t * height);
        cr.line_to(left + width, top + t * height);
    }
    let _ = cr.stroke();

    // Target line y=1
    let ty = top + (1.0 - (1.0 - y_min) / (y_max - y_min)) * height;
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.25);
    cr.set_dash(&[3.0, 3.0], 0.0);
    cr.move_to(left, ty);
    cr.line_to(left + width, ty);
    let _ = cr.stroke();
    cr.set_dash(&[], 0.0);

    // Response curve (from 0 → 1)
    cr.set_source_rgb(0.95, 0.55, 0.25);
    cr.set_line_width(2.5);
    let steps = 120;
    for i in 0..=steps {
        let t = duration * (i as f64 / steps as f64);
        let y = spring_response(t, &p);
        let sx = left + (t / duration) * width;
        let ny = (y - y_min) / (y_max - y_min);
        let sy = top + (1.0 - ny) * height;
        if i == 0 {
            cr.move_to(sx, sy);
        } else {
            cr.line_to(sx, sy);
        }
    }
    let _ = cr.stroke();

    // Caption
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.45);
    cr.set_font_size(11.0);
    let _ = cr.move_to(left, h - 8.0);
    let _ = cr.show_text(&format!("position over ~{duration:.2}s"));
}

/// Analytic spring from rest at 0 toward target 1 (velocity 0).
fn spring_response(t: f64, p: &SpringParams) -> f64 {
    let m = p.mass.max(0.01);
    let k = p.stiffness.max(0.01);
    let c = p.dampening.max(0.0);
    let omega0 = (k / m).sqrt();
    let zeta = c / (2.0 * (k * m).sqrt());
    // Displacement from equilibrium: start at -1 (position 0), end at 0 (position 1)
    let x0 = -1.0;
    let v0 = 0.0;
    let x = if zeta < 1.0 {
        let wd = omega0 * (1.0 - zeta * zeta).sqrt();
        let a = x0;
        let b = (v0 + zeta * omega0 * x0) / wd;
        (-zeta * omega0 * t).exp() * (a * (wd * t).cos() + b * (wd * t).sin())
    } else if (zeta - 1.0).abs() < 1e-6 {
        let a = x0;
        let b = v0 + omega0 * x0;
        (-omega0 * t).exp() * (a + b * t)
    } else {
        let s = (zeta * zeta - 1.0).sqrt();
        let r1 = -omega0 * (zeta - s);
        let r2 = -omega0 * (zeta + s);
        let a = x0 - (v0 - r1 * x0) / (r2 - r1);
        let b = (v0 - r1 * x0) / (r2 - r1);
        a * (r1 * t).exp() + b * (r2 * t).exp()
    };
    1.0 + x
}

fn spring_settling_time(p: &SpringParams) -> f64 {
    let m = p.mass.max(0.01);
    let k = p.stiffness.max(0.01);
    let c = p.dampening.max(0.0);
    let omega0 = (k / m).sqrt();
    let zeta = c / (2.0 * (k * m).sqrt());
    if zeta < 1.0 {
        // ~4 time constants of envelope
        4.0 / (zeta * omega0).max(0.1)
    } else {
        4.0 / omega0.max(0.1)
    }
}

fn draw_dot(cr: &Context, x: f64, y: f64, r: f64, color: &RGBA) {
    cr.set_source_rgba(
        color.red() as f64,
        color.green() as f64,
        color.blue() as f64,
        color.alpha() as f64,
    );
    cr.arc(x, y, r, 0.0, std::f64::consts::TAU);
    let _ = cr.fill();
    cr.set_source_rgba(0.05, 0.05, 0.06, 0.9);
    cr.set_line_width(1.5);
    cr.arc(x, y, r, 0.0, std::f64::consts::TAU);
    let _ = cr.stroke();
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn parses_nested_points() {
        let mut fields = BTreeMap::new();
        fields.insert("points".into(), json!([[0.1, 0.2], [0.8, 0.9]]));
        let p = BezierPoints::from_fields(&fields);
        assert!((p.x0 - 0.1).abs() < 1e-9);
        assert!((p.y1 - 0.9).abs() < 1e-9);
    }

    #[test]
    fn spring_settles_near_one() {
        let p = SpringParams::default();
        let y = spring_response(2.0, &p);
        assert!(y > 0.9 && y < 1.1, "got {y}");
    }
}

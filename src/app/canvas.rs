//! Canvas painting, edge routing and automatic layout shapes.
//!
//! Kept separate from the app state and inspector UI so the geometry code can
//! be read and tested without navigating the whole application.

use super::*;
use crate::i18n::tr;
use crate::model::Connection;
use eframe::egui::{Align2, FontId, Pos2, Rect, Shape, Stroke, StrokeKind, Ui};

pub(super) fn layout_positions(
    shape: MeshShape,
    count: usize,
    spacing_x: f32,
    spacing_y: f32,
) -> Vec<(f32, f32)> {
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![(0.0, 0.0)];
    }

    let mut positions: Vec<(f32, f32)> = match shape {
        MeshShape::Circle => {
            let radius = (count as f32 * spacing_x / std::f32::consts::TAU).max(250.0);
            (0..count)
                .map(|index| {
                    let angle = index as f32 / count as f32 * std::f32::consts::TAU
                        - std::f32::consts::FRAC_PI_2;
                    (angle.cos() * radius, angle.sin() * radius)
                })
                .collect()
        }
        MeshShape::Square => {
            let per_side = ((count as f32) / 4.0).ceil().max(1.0) as usize;
            let side = (per_side as f32 * spacing_x).max(400.0);
            let half = side / 2.0;
            let perimeter = side * 4.0;
            (0..count)
                .map(|index| {
                    let distance = index as f32 / count as f32 * perimeter;
                    if distance < side {
                        (-half + distance, -half)
                    } else if distance < side * 2.0 {
                        (half, -half + (distance - side))
                    } else if distance < side * 3.0 {
                        (half - (distance - side * 2.0), half)
                    } else {
                        (-half, half - (distance - side * 3.0))
                    }
                })
                .collect()
        }
        MeshShape::Cards => {
            let columns = (count as f32).sqrt().ceil().max(1.0) as usize;
            let rows = count.div_ceil(columns);
            (0..count)
                .map(|index| {
                    let column = index % columns;
                    let row = index / columns;
                    (
                        (column as f32 - (columns.saturating_sub(1)) as f32 / 2.0) * spacing_x,
                        (row as f32 - (rows.saturating_sub(1)) as f32 / 2.0) * spacing_y,
                    )
                })
                .collect()
        }
        MeshShape::Rows => (0..count)
            .map(|index| {
                (
                    0.0,
                    (index as f32 - (count.saturating_sub(1)) as f32 / 2.0) * spacing_y,
                )
            })
            .collect(),
    };

    for (x, y) in &mut positions {
        if x.abs() < 0.0001 {
            *x = 0.0;
        }
        if y.abs() < 0.0001 {
            *y = 0.0;
        }
    }
    positions
}

pub(super) fn paint_grid(
    painter: &egui::Painter,
    canvas: Rect,
    camera: &Camera,
    colors: &CanvasColors,
) {
    // Keep the grid visually useful at every zoom level.
    let mut step = 40.0 * camera.zoom;
    while step < 18.0 {
        step *= 2.0;
    }
    let origin = camera.world_to_screen(canvas, 0.0, 0.0);

    let start_x = canvas.left() + (origin.x - canvas.left()).rem_euclid(step);
    let start_y = canvas.top() + (origin.y - canvas.top()).rem_euclid(step);

    let mut x = start_x;
    let mut index = ((start_x - canvas.left()) / step).round() as i32;
    while x < canvas.right() {
        let color = if index.rem_euclid(5) == 0 {
            colors.grid_major
        } else {
            colors.grid_minor
        };
        painter.line_segment(
            [Pos2::new(x, canvas.top()), Pos2::new(x, canvas.bottom())],
            Stroke::new(1.0, color),
        );
        x += step;
        index += 1;
    }

    let mut y = start_y;
    let mut index = ((start_y - canvas.top()) / step).round() as i32;
    while y < canvas.bottom() {
        let color = if index.rem_euclid(5) == 0 {
            colors.grid_major
        } else {
            colors.grid_minor
        };
        painter.line_segment(
            [Pos2::new(canvas.left(), y), Pos2::new(canvas.right(), y)],
            Stroke::new(1.0, color),
        );
        y += step;
        index += 1;
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_node(
    painter: &egui::Painter,
    rect: Rect,
    host: &crate::model::Host,
    selected: bool,
    hovered: bool,
    zoom: f32,
    listen_port: u16,
    colors: &CanvasColors,
) {
    let color = host_color(host.id);
    let radius = CornerRadius::same((14.0 * zoom).clamp(4.0, 18.0) as u8);

    // Soft shadow.
    painter.rect_filled(
        rect.translate(Vec2::new(0.0, 4.0 * zoom)),
        radius,
        colors.node_shadow,
    );

    let fill = if selected {
        colors.node_fill_selected
    } else if hovered {
        colors.node_fill_hovered
    } else {
        colors.node_fill
    };
    painter.rect_filled(rect, radius, fill);

    let border = if selected {
        Stroke::new((3.0 * zoom).max(1.5), colors.selected_accent)
    } else if hovered {
        Stroke::new((2.0 * zoom).max(1.5), colors.node_border_hovered)
    } else {
        Stroke::new((1.5 * zoom).max(1.0), colors.node_border)
    };

    // Keep the colour strip inside the card outline. Painting the rounded card
    // shape and clipping it to a narrow vertical band makes the strip follow the
    // card's rounded corners; a narrow rounded rectangle cannot do this because
    // its corner radius gets clamped to half its width.
    let inner = rect.shrink(border.width);
    let strip_width = (6.0 * zoom).max(2.5);
    let strip_clip = Rect::from_min_max(
        inner.left_top(),
        Pos2::new(
            (rect.left() + strip_width).min(inner.right()),
            inner.bottom(),
        ),
    );
    painter
        .with_clip_rect(strip_clip)
        .rect_filled(rect, radius, color);

    painter.rect_stroke(rect, radius, border, StrokeKind::Inside);

    if zoom < 0.38 {
        return;
    }

    let clipped = painter.with_clip_rect(rect.shrink(2.0));
    let text_left = rect.left() + 14.0 * zoom;
    let mut cursor_y = rect.top() + 12.0 * zoom;

    clipped.text(
        Pos2::new(text_left, cursor_y),
        Align2::LEFT_TOP,
        &host.name,
        FontId::proportional((15.0 * zoom).max(7.0)),
        colors.node_text,
    );
    cursor_y += 21.0 * zoom;

    clipped.text(
        Pos2::new(text_left, cursor_y),
        Align2::LEFT_TOP,
        if host.address.trim().is_empty() {
            "no address"
        } else {
            host.address.trim()
        },
        FontId::monospace((11.5 * zoom).max(6.0)),
        if host.address.trim().is_empty() {
            DANGER
        } else {
            colors.node_subtext
        },
    );
    cursor_y += 18.0 * zoom;

    if zoom > 0.52 {
        let endpoint = if host.endpoint.trim().is_empty() {
            "no endpoint".to_owned()
        } else {
            crate::model::format_endpoint(&host.endpoint, listen_port)
        };
        clipped.text(
            Pos2::new(text_left, cursor_y),
            Align2::LEFT_TOP,
            endpoint,
            FontId::monospace((10.0 * zoom).max(6.0)),
            colors.node_subtext,
        );
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct ConnectionInteraction {
    pub clicked: bool,
    pub double_clicked: bool,
    pub delete_requested: bool,
    pub disable_requested: bool,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_connection(
    ui: &Ui,
    painter: &egui::Painter,
    from_rect: Rect,
    to_rect: Rect,
    from_name: &str,
    to_name: &str,
    endpoint: Option<&str>,
    connection: &Connection,
    selected: bool,
    highlighted: bool,
    dimmed: bool,
    zoom: f32,
    occupied_labels: &mut Vec<Rect>,
    colors: &CanvasColors,
) -> ConnectionInteraction {
    let from_center = from_rect.center();
    let to_center = to_rect.center();
    let delta = to_center - from_center;
    let distance = delta.length();
    if distance < 1.0 {
        return ConnectionInteraction::default();
    }

    let direction = delta / distance;
    let perpendicular = Vec2::new(-direction.y, direction.x);
    let curve = (distance * 0.13).clamp(22.0, 86.0);

    let start_guidance = from_center + perpendicular * curve;
    let end_guidance = to_center + perpendicular * curve;
    let start = rect_exit_point(from_rect, start_guidance);
    let end = rect_exit_point(to_rect, end_guidance);

    let ctrl1 = start + (end - start) / 3.0 + perpendicular * curve;
    let ctrl2 = start + (end - start) * (2.0 / 3.0) + perpendicular * curve;
    let points = [start, ctrl1, ctrl2, end];

    let (line_color, width) = if !connection.enabled {
        (colors.edge_disabled, (1.2 * zoom).max(1.0))
    } else if selected || highlighted {
        (colors.selected_accent, (2.6 * zoom).max(1.4))
    } else {
        (colors.edge, (1.6 * zoom).max(1.0))
    };
    let line_color = if dimmed {
        line_color.gamma_multiply(0.22)
    } else {
        line_color
    };

    if connection.enabled {
        painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
            points,
            false,
            Color32::TRANSPARENT,
            Stroke::new(width, line_color),
        ));
    } else {
        let samples: Vec<Pos2> = (0..=40)
            .map(|step| {
                let t = step as f32 / 40.0;
                sample_bezier(points, t)
            })
            .collect();
        painter.add(Shape::dashed_line(
            &samples,
            Stroke::new(width, line_color),
            6.0,
            6.0,
        ));
    }

    // Arrow head at the destination.
    let tangent = (end - ctrl2).normalized();
    if tangent.is_finite() {
        let head_len = (8.0 * zoom).clamp(4.0, 14.0);
        let head_width = (4.5 * zoom).clamp(2.0, 9.0);
        let base = end - tangent * head_len;
        painter.add(Shape::convex_polygon(
            vec![
                end,
                base + perpendicular * head_width,
                base - perpendicular * head_width,
            ],
            line_color,
            Stroke::NONE,
        ));
    }

    if zoom < 0.32 {
        return ConnectionInteraction::default();
    }

    let no_endpoint = tr!("ui.paint_connection.no_endpoint");
    let label_text = if !connection.enabled {
        tr!(
            "ui.paint_connection.disabled_endpoint",
            endpoint = endpoint.unwrap_or(&no_endpoint)
        )
    } else {
        endpoint.unwrap_or(&no_endpoint).to_owned()
    };
    let label_text = truncate_text(&label_text, 38);
    let font = FontId::monospace((11.5 * zoom).max(7.0));
    let galley = painter.layout_no_wrap(label_text.clone(), font, Color32::PLACEHOLDER);
    let label_size = galley.size() + Vec2::new(14.0 * zoom, 7.0 * zoom);

    // Put the label on the curve, but try a few positions so complete-mesh
    // labels do not all stack up in the middle of the canvas.
    let mut label_rect = Rect::from_center_size(sample_bezier(points, 0.5), label_size);
    for t in [0.34, 0.42, 0.5, 0.58, 0.66, 0.26, 0.74, 0.82] {
        let candidate = Rect::from_center_size(sample_bezier(points, t), label_size);
        if !occupied_labels
            .iter()
            .any(|occupied| occupied.expand(4.0).intersects(candidate))
        {
            label_rect = candidate;
            break;
        }
    }
    occupied_labels.push(label_rect.expand(2.0));

    let response = ui.interact(
        label_rect,
        ui.make_persistent_id(("connection_label", connection.from, connection.to)),
        Sense::click(),
    );
    let hovered = response.hovered();

    let label_opacity = if dimmed { 0.32 } else { 1.0 };
    let background = if selected {
        colors.label_bg_selected
    } else if hovered {
        colors.label_bg_hovered
    } else {
        colors.label_bg
    }
    .gamma_multiply(label_opacity);
    let border_color = if selected {
        colors.selected_accent
    } else {
        colors.label_border
    }
    .gamma_multiply(label_opacity);
    let radius = CornerRadius::same((6.0 * zoom).clamp(2.0, 8.0) as u8);
    painter.rect_filled(label_rect, radius, background);
    painter.rect_stroke(
        label_rect,
        radius,
        Stroke::new((1.2 * zoom).max(1.0), border_color),
        StrokeKind::Inside,
    );

    let text_color = if !connection.enabled {
        colors.label_text_disabled
    } else if selected {
        colors.label_text_selected
    } else {
        colors.label_text
    }
    .gamma_multiply(if dimmed { 0.45 } else { 1.0 });
    painter.galley(
        label_rect.center() - galley.size() / 2.0,
        galley,
        text_color,
    );

    let response = response.on_hover_text(tr!(
        "ui.paint_connection.from_to_endpoint_endpoint",
        from = from_name,
        to = to_name,
        endpoint = endpoint.unwrap_or("none")
    ));
    let mut delete_requested = false;
    let mut disable_requested = false;
    response.context_menu(|ui| {
        if connection.enabled && ui.button(tr!("ui.canvas.disable_connection")).clicked() {
            disable_requested = true;
            ui.close();
        }
        ui.separator();
        if ui.button(tr!("ui.canvas.delete_connection")).clicked() {
            delete_requested = true;
            ui.close();
        }
    });

    ConnectionInteraction {
        clicked: response.clicked(),
        double_clicked: response.double_clicked(),
        delete_requested,
        disable_requested,
    }
}

pub(super) fn sample_bezier(points: [Pos2; 4], t: f32) -> Pos2 {
    let u = 1.0 - t;
    let x = u * u * u * points[0].x
        + 3.0 * u * u * t * points[1].x
        + 3.0 * u * t * t * points[2].x
        + t * t * t * points[3].x;
    let y = u * u * u * points[0].y
        + 3.0 * u * u * t * points[1].y
        + 3.0 * u * t * t * points[2].y
        + t * t * t * points[3].y;
    Pos2::new(x, y)
}

/// Intersect the segment from `rect.center()` towards `target` with the rect
/// edge.  This makes arrows start and end at node borders instead of centres.
pub(super) fn rect_exit_point(rect: Rect, target: Pos2) -> Pos2 {
    let direction = target - rect.center();
    if direction.x.abs() < f32::EPSILON && direction.y.abs() < f32::EPSILON {
        return rect.center();
    }

    let half = rect.size() / 2.0;
    let scale_x = if direction.x.abs() > f32::EPSILON {
        half.x / direction.x.abs()
    } else {
        f32::INFINITY
    };
    let scale_y = if direction.y.abs() > f32::EPSILON {
        half.y / direction.y.abs()
    } else {
        f32::INFINITY
    };
    let scale = scale_x.min(scale_y);
    rect.center() + direction * scale
}

pub(super) fn host_color(id: u64) -> Color32 {
    const PALETTE: [Color32; 8] = [
        Color32::from_rgb(86, 204, 242),
        Color32::from_rgb(110, 231, 183),
        Color32::from_rgb(255, 175, 84),
        Color32::from_rgb(196, 148, 255),
        Color32::from_rgb(255, 122, 154),
        Color32::from_rgb(126, 166, 255),
        Color32::from_rgb(240, 219, 114),
        Color32::from_rgb(100, 214, 205),
    ];
    PALETTE[(id as usize) % PALETTE.len()]
}

pub(super) fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_owned()
    } else {
        let mut truncated: String = text.chars().take(max_chars.saturating_sub(1)).collect();
        truncated.push('…');
        truncated
    }
}

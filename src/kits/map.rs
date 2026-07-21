//! Experimental map-kit proof of concept.
//!
//! Geographic data is projected at build time into ordinary Manic polygons
//! and polylines. The renderer and animation vocabulary stay unchanged.

use geojson::{GeoJson, Value};
use macroquad::prelude::Vec2;

use crate::lang::diag::Error;
use crate::lang::lower::{Args, Registry};
use crate::primitives::{Entity, Shape, StrokeStyle};
use crate::scene::Scene;
use crate::style;

const INDIA: &str = include_str!("../../assets/maps/india-ne-110m.geojson");

#[derive(Debug, Clone, Copy)]
pub struct MapViewData {
    pub center: Vec2,
    pub width: f32,
    pub height: f32,
    pub min_lon: f64,
    pub max_lon: f64,
    pub min_lat: f64,
    pub max_lat: f64,
}

impl MapViewData {
    fn project(self, lon: f64, lat: f64) -> Vec2 {
        let left = self.center.x - self.width * 0.5;
        let top = self.center.y - self.height * 0.5;
        let x = (lon - self.min_lon) / (self.max_lon - self.min_lon);
        let y = (self.max_lat - lat) / (self.max_lat - self.min_lat);
        Vec2::new(left + x as f32 * self.width, top + y as f32 * self.height)
    }
}

fn parse_bounds(text: &str, span: manic_lang::Span) -> Result<[f64; 4], Error> {
    let values: Result<Vec<_>, _> = text.split_whitespace().map(str::parse::<f64>).collect();
    let values = values.map_err(|_| {
        Error::new(
            "map bounds must be four numbers: \"min_lon max_lon min_lat max_lat\"",
            span,
        )
    })?;
    if values.len() != 4
        || values.iter().any(|value| !value.is_finite())
        || values[0] >= values[1]
        || values[2] >= values[3]
    {
        return Err(Error::new(
            "map bounds must be ordered as \"min_lon max_lon min_lat max_lat\"",
            span,
        ));
    }
    Ok([values[0], values[1], values[2], values[3]])
}

/// `map(id, center, width, height, "min_lon max_lon min_lat max_lat")`.
fn c_map(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    let id = args.ident(0)?;
    let center = args.pair(1)?;
    let width = args.num(2)?;
    let height = args.num(3)?;
    if width <= 0.0 || height <= 0.0 {
        return Err(Error::new(
            "map width and height must be positive",
            args.span_of(2),
        ));
    }
    let bounds = parse_bounds(&args.text(4)?, args.span_of(4))?;
    let view = MapViewData {
        center,
        width,
        height,
        min_lon: bounds[0],
        max_lon: bounds[1],
        min_lat: bounds[2],
        max_lat: bounds[3],
    };

    let mut frame = Entity::new(
        format!("{id}.frame"),
        Shape::Rect {
            w: width,
            h: height,
        },
        center,
        style::DIM,
    );
    frame.stroke = StrokeStyle {
        fill: false,
        outline: true,
        width: 1.5,
        outline_color: Some(style::DIM),
    };
    frame.tags.push(id.clone());
    frame.z = -5;
    scene.add(frame);
    scene.map_views.insert(id, view);
    Ok(())
}

fn polygon_rings(value: &Value) -> Vec<&Vec<Vec<Vec<f64>>>> {
    match value {
        Value::Polygon(polygon) => vec![polygon],
        Value::MultiPolygon(polygons) => polygons.iter().collect(),
        _ => Vec::new(),
    }
}

/// `border(id, map, "IND")` — project a bundled country outline into a map.
fn c_border(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    let id = args.ident(0)?;
    let view_id = args.ident(1)?;
    let country = args.text(2)?;
    let view = *scene
        .map_views
        .get(&view_id)
        .ok_or_else(|| Error::new(format!("no map named `{view_id}`"), args.span_of(1)))?;
    if !country.eq_ignore_ascii_case("IND") && !country.eq_ignore_ascii_case("India") {
        return Err(Error::new(
            "map PoC currently bundles only `IND` (India)",
            args.span_of(2),
        ));
    }

    let geojson: GeoJson = INDIA.parse().map_err(|error| {
        Error::new(
            format!("bundled IND boundary is invalid: {error}"),
            args.span_of(2),
        )
    })?;
    let geometry = match &geojson {
        GeoJson::Feature(feature) => feature.geometry.as_ref(),
        _ => None,
    }
    .ok_or_else(|| Error::new("bundled IND boundary has no geometry", args.span_of(2)))?;

    let mut count = 0;
    for polygon in polygon_rings(&geometry.value) {
        let Some(exterior) = polygon.first() else {
            continue;
        };
        let points: Vec<Vec2> = exterior
            .iter()
            .filter(|coordinate| coordinate.len() >= 2)
            .map(|coordinate| view.project(coordinate[0], coordinate[1]))
            .collect();
        if points.len() < 3 {
            continue;
        }

        let mut fill = Entity::new(
            format!("{id}.part{count}.fill"),
            Shape::Polygon {
                pts: points.clone(),
            },
            Vec2::ZERO,
            style::CYAN,
        );
        fill.opacity = 0.10;
        fill.stroke = StrokeStyle {
            fill: true,
            outline: false,
            ..Default::default()
        };
        fill.tags.extend([id.clone(), format!("{id}.fill")]);
        fill.z = -3;
        scene.add(fill);

        let mut border = Entity::new(
            format!("{id}.part{count}.border"),
            Shape::Polyline { pts: points },
            Vec2::ZERO,
            style::CYAN,
        );
        border.stroke = StrokeStyle {
            fill: false,
            outline: true,
            width: 3.0,
            outline_color: Some(style::CYAN),
        };
        border.tags.extend([id.clone(), format!("{id}.border")]);
        border.z = 2;
        scene.add(border);
        count += 1;
    }
    if count == 0 {
        return Err(Error::new(
            "IND boundary contains no polygon rings",
            args.span_of(2),
        ));
    }
    Ok(())
}

pub fn register(registry: &mut Registry) {
    registry.ctor("map", c_map);
    registry.ctor("border", c_border);
}

#[cfg(test)]
mod tests {
    #[test]
    fn projects_a_real_named_border_with_animatable_tags() {
        let source = r#"
            map(world, (500, 350), 600, 500, "68 98 6 36");
            border(india, world, "IND");
            untraced(india.border);
            step("highlight") { draw(india.border, 1.2); recolor(india.border, gold, 0.4); }
        "#;
        let movie = crate::parse(source).expect("map PoC should compile");
        assert!(movie.scene.get("world.frame").is_some());
        assert!(movie
            .scene
            .entities
            .iter()
            .any(|entity| entity.tags.iter().any(|tag| tag == "india.border")));
    }

    #[test]
    fn rejects_unsupported_countries_clearly() {
        let source = r#"
            map(world, (500, 350), 600, 500, "68 98 6 36");
            border(country, world, "USA");
        "#;
        let error = match crate::parse(source) {
            Ok(_) => panic!("unsupported country must fail"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("only `IND`"));
    }
}

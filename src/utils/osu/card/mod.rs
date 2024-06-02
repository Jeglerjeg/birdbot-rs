pub mod body;
pub mod header;

use std::sync::Arc;
use crate::Error;
use color_space::{FromRgb, Hsv, Rgb};
use resvg::tiny_skia::Pixmap;
use resvg::usvg::fontdb::Database;
use resvg::usvg::{Transform, Tree};
use resvg::{render, usvg};
use rosu_v2::prelude::UserExtended;
use serenity::all::Colour;
use svg::Document;

pub async fn render_card(osu_user: &UserExtended, color: Colour) -> Result<Pixmap, Error> {
    let svg = load_svg(osu_user, color).await?;
    let mut pixmap = Pixmap::new(1500, 940).unwrap();
    render(&svg, Transform::default(), &mut pixmap.as_mut());
    Ok(pixmap)
}

pub fn load_fonts() -> Database {
    let mut database = Database::new();
    database.load_fonts_dir("src/utils/osu/card/assets/fonts");
    database
}

pub async fn load_svg(osu_user: &UserExtended, color: Colour) -> Result<Tree, Error> {
    Ok(Tree::from_str(
        &generate_svg(osu_user, color).await?,
        &usvg::Options {
            fontdb: Arc::from(load_fonts()),
            ..Default::default()
        },
    )?)
}

fn adjust_saturation_and_brightness(color: Colour, saturation: f64, brightness: f64) -> Colour {
    let rgb = Rgb::new(
        f64::from(color.r()),
        f64::from(color.g()),
        f64::from(color.b()),
    );
    let hsv = Hsv::from_rgb(&rgb);
    let adjusted_hsv = Hsv::new(hsv.h, saturation, brightness);
    let adjusted_rgb = Rgb::from(adjusted_hsv);
    Colour::from_rgb(
        adjusted_rgb.r as u8,
        adjusted_rgb.g as u8,
        adjusted_rgb.b as u8,
    )
}

pub async fn generate_svg(osu_user: &UserExtended, color: Colour) -> Result<String, Error> {
    let color = adjust_saturation_and_brightness(color, 0.45, 0.3);

    let mut document = Document::new()
        .set("viewBox", (0, 0, 375, 235))
        .set("xmlns:xlink", "http://www.w3.org/1999/xlink")
        .set("fill", "none")
        .set("width", 1500)
        .set("height", 940);
    document = header::draw_header(document, osu_user, color).await?;
    document = body::draw_body(document, osu_user).await?;
    Ok(document.to_string())
}

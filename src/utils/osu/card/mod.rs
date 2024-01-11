pub mod body;
pub mod header;

use crate::Error;
use color_space::{FromRgb, Hsv, Rgb};
use resvg::tiny_skia::Pixmap;
use resvg::usvg;
use resvg::usvg::fontdb::Database;
use resvg::usvg::{Size, Transform, Tree, TreeParsing, TreeTextToPath};
use rosu_v2::prelude::UserExtended;
use serenity::all::Colour;
use svg::Document;

pub async fn render_card(osu_user: &UserExtended, color: Colour) -> Result<Pixmap, Error> {
    let mut svg = load_svg(osu_user, color).await?;
    svg.convert_text(&load_fonts());
    let mut renderable = resvg::Tree::from_usvg(&svg);
    renderable.size = Size::from_wh(1500.0, 940.0).unwrap();
    let mut pixmap = Pixmap::new(1500, 940).unwrap();
    renderable.render(Transform::default(), &mut pixmap.as_mut());
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
        &usvg::Options::default(),
    )?)
}

fn adjust_saturation_and_brightness(color: Colour, saturation: f64, brightness: f64) -> Colour {
    let rgb = Rgb::new(color.r() as f64, color.g() as f64, color.b() as f64);
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
        .set("width", 375)
        .set("height", 235);
    document = header::draw_header(document, osu_user, color).await?;
    document = body::draw_body(document, osu_user).await?;
    Ok(document.to_string())
}

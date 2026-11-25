pub mod body;
pub mod header;

use crate::Error;
use color_space::{FromRgb, Hsv, Rgb};
use poise::serenity_prelude::Colour;
use resvg::tiny_skia::Pixmap;
use resvg::usvg::fontdb::Database;
use resvg::usvg::{Transform, Tree};
use resvg::{render, usvg};
use rosu_v2::prelude::UserExtended;
use std::sync::{Arc, LazyLock};
use svg::Document;

static FONT_DATABASE: LazyLock<Arc<Database>> = LazyLock::new(|| {
    let mut database = Database::new();
    database.load_font_data(include_bytes!("assets/fonts/torus/Torus-Regular.otf").to_vec());
    database.load_font_data(include_bytes!("assets/fonts/torus/Torus-SemiBold.otf").to_vec());
    database.load_font_data(include_bytes!("assets/fonts/torus/Torus-Bold.otf").to_vec());
    Arc::new(database)
});

pub async fn render_card(
    client: &reqwest::Client,
    osu_user: &UserExtended,
    color: Colour,
) -> Result<Vec<u8>, Error> {
    let svg = generate_svg(client, osu_user, color).await?;

    tokio::task::spawn_blocking(move || -> Result<Vec<u8>, Error> {
        let tree = Tree::from_str(
            &svg,
            &usvg::Options {
                fontdb: FONT_DATABASE.clone(),
                ..Default::default()
            },
        )?;
        let mut pixmap = Pixmap::new(1500, 940).ok_or("Failed to allocate the card pixmap")?;
        render(&tree, Transform::default(), &mut pixmap.as_mut());
        Ok(pixmap.encode_png()?)
    })
    .await?
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

pub async fn generate_svg(
    client: &reqwest::Client,
    osu_user: &UserExtended,
    color: Colour,
) -> Result<String, Error> {
    let color = adjust_saturation_and_brightness(color, 0.45, 0.3);

    let mut document = Document::new()
        .set("viewBox", (0, 0, 375, 235))
        .set("xmlns:xlink", "http://www.w3.org/1999/xlink")
        .set("fill", "none")
        .set("width", 1500)
        .set("height", 940);
    document = header::draw_header(document, osu_user, color, client).await?;
    document = body::draw_body(document, osu_user).await?;
    Ok(document.to_string())
}

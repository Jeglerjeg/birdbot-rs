use crate::Error;
use base64::engine::general_purpose;
use base64::Engine;
use image::imageops::{crop, resize, FilterType};
use image::EncodableLayout;
use num_format::{Locale, ToFormattedString};
use rosu_v2::prelude::UserExtended;
use serenity::all::Colour;
use std::cmp::min;
use std::io::Cursor;
use std::str::FromStr;
use svg::node::element::{
    Definitions, Image, LinearGradient, Mask, Path, RadialGradient, Rectangle, Stop, Text,
};
use svg::{Document, Node};
use time::OffsetDateTime;

pub async fn draw_header(
    mut document: Document,
    osu_user: &UserExtended,
    color: Colour,
) -> Result<Document, Error> {
    document = draw_avatar_and_cover(document, osu_user, color).await?;
    document = draw_following_pill(document, osu_user);
    document = draw_osu_circle(document);
    document = draw_username(document, osu_user.username.as_str());
    let level = if let Some(statistic) = &osu_user.statistics {
        f32::from_str(&format!(
            "{}.{}",
            statistic.level.current, statistic.level.progress
        ))? as u8
    } else {
        0
    };
    document = draw_level(document, level);
    document = draw_user_group(
        document,
        osu_user.profile_color.clone(),
        osu_user.is_supporter,
    );
    document = draw_join_date(document, osu_user.join_date);
    Ok(document)
}

pub async fn draw_avatar_and_cover(
    document: Document,
    osu_user: &UserExtended,
    color: Colour,
) -> Result<Document, Error> {
    let header_rect = Rectangle::new()
        .set("x", 0)
        .set("y", 0)
        .set("width", "375")
        .set("height", 60)
        .set("rx", 10)
        .set("ry", 10)
        .set("fill", "white");

    let gradient = LinearGradient::new()
        .set("x1", 0)
        .set("y1", 0)
        .set("x2", 375)
        .set("y2", 60)
        .set("gradientUnits", "userSpaceOnUse")
        .set("id", "d1")
        .add(
            Stop::new()
                .set("offset", 0)
                .set("stop-color", format!("#{}", color.hex())),
        )
        .add(
            Stop::new()
                .set("offset", 1)
                .set(
                    "stop-color",
                    format!("#{}", Colour::from(color.0 - 500).hex()),
                )
                .set("stop-opacity", 0.6),
        );

    let header_mask = Mask::new().set("id", "d0").add(header_rect);

    let avatar_rect = Rectangle::new()
        .set("x", 0)
        .set("y", 0)
        .set("width", 60)
        .set("height", 60)
        .set("rx", 10)
        .set("ry", 10)
        .set("fill", "white");

    let avatar_mask = Mask::new().set("id", "d2").add(avatar_rect);

    let definitions = Definitions::new()
        .add(header_mask)
        .add(gradient)
        .add(avatar_mask);

    let body_rect = Rectangle::new()
        .set("x", 0)
        .set("y", 0)
        .set("width", 375)
        .set("height", 235)
        .set("rx", 10)
        .set("ry", 10)
        .set("fill", "#2E3835");

    let reqwest_client = reqwest::Client::new();

    let header_image_bytes = reqwest_client
        .get(&osu_user.cover.url)
        .send()
        .await?
        .bytes()
        .await?;

    let resized_header_image =
        fit_image_to_aspect_ratio(header_image_bytes.as_bytes(), 375 / (235 / 4))?;

    let header_image_base64 = general_purpose::STANDARD.encode(&resized_header_image);

    let header_image = Image::new()
        .set("x", 0)
        .set("y", 0)
        .set("width", 375)
        .set("height", 60)
        .set(
            "xlink:href",
            format!("data:image/png;charset=utf-8;base64,{header_image_base64}"),
        )
        .set("mask", "url(#d0)");

    let header_gradient = Rectangle::new()
        .set("x", 0)
        .set("y", 0)
        .set("width", 375)
        .set("height", 60)
        .set("fill", "url(#d1)")
        .set("rx", 10)
        .set("ry", 10);

    let avatar_image_request = reqwest_client.get(&osu_user.avatar_url).send().await?;

    let avatar_image_bytes = avatar_image_request.bytes().await?;

    let opened_image = image::load_from_memory(avatar_image_bytes.as_bytes())?;

    let mut buffer = Vec::new();

    opened_image.write_to(&mut Cursor::new(&mut buffer), image::ImageOutputFormat::Png)?;

    let avatar_image_base64 = general_purpose::STANDARD.encode(buffer);

    let avatar_image = Image::new()
        .set("x", 0)
        .set("y", 0)
        .set("width", 60)
        .set("height", 60)
        .set(
            "xlink:href",
            format!("data:image/png;base64,{avatar_image_base64}"),
        )
        .set("mask", "url(#d2)");

    Ok(document
        .add(definitions)
        .add(body_rect)
        .add(header_image)
        .add(header_gradient)
        .add(avatar_image))
}

fn fit_image_to_aspect_ratio(image_bytes: &[u8], aspect_ratio: u32) -> Result<Vec<u8>, Error> {
    let mut image = image::load_from_memory(image_bytes)?;
    let image_height = image.height();
    let image_width = image.width();
    let target_width = min(image_width, image_height * aspect_ratio);
    let target_height = min(image_height, image_width / aspect_ratio);

    let cropped_image = crop(
        &mut image,
        (target_width - image_width) / 2,
        (image_height - target_height) / 2,
        target_width,
        target_height,
    )
    .to_image();

    let resized_image = resize(&cropped_image, 1500, 240, FilterType::Lanczos3);

    let mut buffer = Vec::new();

    resized_image.write_to(&mut Cursor::new(&mut buffer), image::ImageOutputFormat::Png)?;

    Ok(buffer)
}

pub fn draw_following_pill(document: Document, osu_user: &UserExtended) -> Document {
    let icon = Path::new()
        .set("id", "following_icon")
        .set("d", "M116 34.5C117.263 34.5 118.286 33.4928 118.286 32.25C118.286 31.0072 117.263 30 116 30C114.738 30 113.714 31.0072 113.714 32.25C113.714 33.4928 114.738 34.5 116 34.5ZM117.6 35.0625H117.302C116.905 35.2418 116.464 35.3438 116 35.3438C115.536 35.3438 115.096 35.2418 114.698 35.0625H114.4C113.075 35.0625 112 36.1207 112 37.425V38.1562C112 38.6221 112.384 39 112.857 39H119.143C119.616 39 120 38.6221 120 38.1562V37.425C120 36.1207 118.925 35.0625 117.6 35.0625Z")
        .set("fill", "white");
    let mut number_of_followers = Text::new()
        .set("x", 123.471)
        .set("y", 38.8)
        .set("fill", "white")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 12)
        .set("letter-spacing", "0em");
    number_of_followers.append(svg::node::Text::new(
        osu_user
            .follower_count
            .unwrap_or(0)
            .to_formatted_string(&Locale::en),
    ));
    document.add(icon).add(number_of_followers)
}

pub fn draw_osu_circle(document: Document) -> Document {
    let inner_circle = Path::new()
        .set("id", "osu_inner_circle")
        .set("d", "M86 39.4255C88.4441 39.4255 90.4255 37.4441 90.4255 35C90.4255 32.5558 88.4441 30.5745 86 30.5745C83.5558 30.5745 81.5745 32.5558 81.5745 35C81.5745 37.4441 83.5558 39.4255 86 39.4255Z")
        .set("fill", "white");
    let middle_circle = Path::new()
        .set("id", "osu_middle_circle")
        .set("d", "M85.9999 31.2553C87.0008 31.2553 87.9404 31.6433 88.6484 32.3514C89.3565 33.0595 89.7446 33.9991 89.7446 34.9999C89.7446 36.0008 89.3565 36.9404 88.6484 37.6484C87.9404 38.3565 87.0008 38.7446 85.9999 38.7446C84.9991 38.7446 84.0595 38.3565 83.3514 37.6484C82.6433 36.9404 82.2553 36.0008 82.2553 34.9999C82.2553 33.9991 82.6433 33.0595 83.3514 32.3514C84.0595 31.6433 84.9991 31.2553 85.9999 31.2553ZM85.9999 29.8936C83.1812 29.8936 80.8936 32.1812 80.8936 34.9999C80.8936 37.8187 83.1812 40.1063 85.9999 40.1063C88.8187 40.1063 91.1063 37.8187 91.1063 34.9999C91.1063 32.1812 88.8187 29.8936 85.9999 29.8936Z")
        .set("fill", "white");
    let outer_circle = Path::new()
        .set("id", "osu_outer_circle")
        .set("d", "M86 28.3617C89.6664 28.3617 92.6383 31.3336 92.6383 35C92.6383 38.6664 89.6664 41.6383 86 41.6383C82.3336 41.6383 79.3617 38.6664 79.3617 35C79.3617 31.3336 82.3336 28.3617 86 28.3617ZM86 27C84.9209 27 83.8723 27.2111 82.8851 27.6298C81.9319 28.0315 81.0774 28.6102 80.3421 29.3421C79.6068 30.0774 79.0315 30.9319 78.6298 31.8851C78.2111 32.8723 78 33.9209 78 35C78 36.0791 78.2111 37.1277 78.6298 38.1149C79.0315 39.0681 79.6102 39.9226 80.3421 40.6579C81.0774 41.3932 81.9319 41.9685 82.8851 42.3702C83.8723 42.7889 84.9209 43 86 43C87.0791 43 88.1277 42.7889 89.1149 42.3702C90.0681 41.9685 90.9226 41.3898 91.6579 40.6579C92.3932 39.9226 92.9685 39.0681 93.3702 38.1149C93.7889 37.1277 94 36.0791 94 35C94 33.9209 93.7889 32.8723 93.3702 31.8851C92.9685 30.9319 92.3898 30.0774 91.6579 29.3421C90.9226 28.6068 90.0681 28.0315 89.1149 27.6298C88.1277 27.2111 87.0791 27 86 27Z")
        .set("fill", "white");
    document
        .add(inner_circle)
        .add(middle_circle)
        .add(outer_circle)
}

pub fn draw_username(document: Document, username: &str) -> Document {
    let mut username_text = Text::new()
        .set("fill", "white")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 16)
        .set("x", 75)
        .set("y", 21.363_636_364);
    username_text.append(svg::node::Text::new(username));
    document.add(username_text)
}

pub fn draw_level(document: Document, level: u8) -> Document {
    let level_hexagon_gradient = RadialGradient::new()
        .set("id", "level_hexagon_paint")
        .set("cx", 0)
        .set("cy", 0)
        .set("r", 1)
        .set("gradientUnits", "userSpaceOnUse")
        .set(
            "gradientTransform",
            "translate(361.5 50.8333) rotate(-124.824) scale(46.6964)",
        )
        .add(Stop::new().set("stop-color", "#FFD966"))
        .add(
            Stop::new()
                .set("stop-color", "#FF84BF")
                .set("offset", 0.578_125),
        )
        .add(Stop::new().set("stop-color", "#84FFFF").set("offset", 1));
    let level_hexagon = Path::new()
        .set("id", "level_hexagon")
        .set("d", "M339.75 9.18579C342.38 7.66741 345.62 7.66741 348.25 9.18579L359.901 15.9123C362.531 17.4307 364.151 20.2367 364.151 23.2735V36.7265C364.151 39.7633 362.531 42.5693 359.901 44.0877L348.25 50.8142C345.62 52.3326 342.38 52.3326 339.75 50.8142L328.099 44.0877C325.469 42.5693 323.849 39.7633 323.849 36.7265V23.2735C323.849 20.2367 325.469 17.4307 328.099 15.9123L339.75 9.18579Z")
        .set("stroke_width", 9)
        .set("stroke", "url(#level_hexagon_paint)");
    let mut level_text = Text::new()
        .set("id", "xp_level_text")
        .set("fill", "white")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 16)
        .set("letter-spacing", "0em")
        .set("dominant-baseline", "middle")
        .set("text-anchor", "middle")
        .set("x", 344)
        .set("y", 31);
    level_text.append(svg::node::Text::new(format!("{level}")));

    document
        .add(level_hexagon)
        .add(level_hexagon_gradient)
        .add(level_text)
}

pub fn draw_user_group(
    document: Document,
    profile_color: Option<String>,
    is_supporter: bool,
) -> Document {
    let group_color: String;
    if let Some(profile_color) = profile_color {
        if profile_color == "-1" {
            if is_supporter {
                group_color = "#FF66AB".to_string();
            } else {
                group_color = "#0087CA".to_string();
            }
        } else {
            group_color = profile_color;
        }
    } else if is_supporter {
        group_color = "#FF66AB".to_string();
    } else {
        group_color = "#0087CA".to_string();
    }
    let group_rectangle = Rectangle::new()
        .set("x", 65)
        .set("y", 12)
        .set("width", 4)
        .set("height", 40)
        .set("rx", 2)
        .set("fill", group_color);

    document.add(group_rectangle)
}

pub fn draw_join_date(document: Document, join_time: OffsetDateTime) -> Document {
    let mut join_date = Text::new()
        .set("id", "join_date")
        .set("fill", "white")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 10)
        .set("letter-spacing", "0em")
        .set("x", 75)
        .set("y", 55);
    let year_and_date_join = format!(
        "{} {} {}",
        join_time.day(),
        join_time.month(),
        join_time.year()
    );
    let current_time = OffsetDateTime::now_utc();
    let time_since = current_time - join_time;
    let formatted_join_date = format!(
        "Joined {} ({}d ago)",
        year_and_date_join,
        time_since.whole_days()
    );
    join_date.append(svg::node::Text::new(formatted_join_date));
    document.add(join_date)
}

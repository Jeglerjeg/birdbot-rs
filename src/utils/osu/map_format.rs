use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::utils::misc::remove_trailing_zeros;
use crate::utils::osu::calculate::calculate;
use crate::utils::osu::misc::gamemode_from_string;
use crate::utils::osu::misc_format::{format_beatmap_link, format_mode_abbreviation};
use crate::Error;
use lazy_static::lazy_static;
use serenity::all::{Color, CreateEmbed};
use serenity::builder::CreateEmbedAuthor;
use std::collections::HashMap;
use std::env;

lazy_static! {
    static ref MAX_DIFF_LENGTH: usize = env::var("MAX_DIFF_LENGTH")
        .unwrap_or_else(|_| String::from("21"))
        .parse::<usize>()
        .expect("Failed to parse max queued songs.");
}

pub async fn format_beatmapset(mut beatmaps: Vec<Beatmap>) -> Result<String, Error> {
    let mut diff_length = 0;
    let mut calculated_beatmaps = HashMap::new();
    for beatmap in beatmaps.clone() {
        if beatmap.version.len() > diff_length {
            diff_length = beatmap.version.len();
        }
        let difficulty_values = calculate(None, &beatmap, None).await?;
        calculated_beatmaps.insert(beatmap.id, difficulty_values);
    }
    if diff_length > *MAX_DIFF_LENGTH {
        diff_length = *MAX_DIFF_LENGTH;
    } else if diff_length < 10 {
        diff_length = 10;
    }
    let mut formatted_beatmaps = format!(
        "```elm\nM   {:<diff_length$}  Stars  Drain  PP",
        "Difficulty"
    );

    beatmaps.sort_by(|a, b| {
        if let Some(difficulty_values) = calculated_beatmaps.get(&a.id) {
            &difficulty_values.total_stars
        } else {
            &a.difficulty_rating
        }
        .total_cmp(
            if let Some(difficulty_values) = calculated_beatmaps.get(&b.id) {
                &difficulty_values.total_stars
            } else {
                &b.difficulty_rating
            },
        )
    });

    for beatmap in beatmaps {
        let difficulty_values = calculated_beatmaps
            .get(&beatmap.id)
            .ok_or("Couldn't get beatmap difficulty values in format_beatmapset")?;

        let diff_name = if beatmap.version.len() < *MAX_DIFF_LENGTH {
            beatmap.version
        } else {
            let chars = beatmap.version.chars();
            let substring: String = chars.into_iter().take(*MAX_DIFF_LENGTH - 3).collect();
            substring + "..."
        };

        let length = (beatmap.total_length / 60, beatmap.total_length % 60);
        let formatted_length = format!("{}:{:02}", length.0, length.1);
        let formatted_stars = format!(
            "{}★",
            remove_trailing_zeros(difficulty_values.total_stars, 2)?
        );
        formatted_beatmaps.push_str(&format!(
            "\n{:<4}{:<diff_length$}  {:<7}{:<7}{}pp",
            format_mode_abbreviation(
                gamemode_from_string(&beatmap.mode)
                    .ok_or("Failed to format mode abbreviation in format_beatmapset")?
            ),
            diff_name,
            formatted_stars,
            formatted_length,
            remove_trailing_zeros(difficulty_values.pp, 0)?,
        ));
    }

    formatted_beatmaps.push_str("```");

    Ok(formatted_beatmaps)
}

pub async fn format_map_status(
    beatmapset_and_beatmap: (Beatmapset, Vec<Beatmap>),
    color: Color,
) -> Result<CreateEmbed, Error> {
    let beatmapset = beatmapset_and_beatmap.0;
    let header = format!("{} - {}", beatmapset.artist, beatmapset.title);

    let embed = CreateEmbed::new();

    let created_author =
        CreateEmbedAuthor::new(header).url(format_beatmap_link(None, beatmapset.id, None));

    Ok(embed
        .image(beatmapset.cover)
        .color(color)
        .description(format_beatmapset(beatmapset_and_beatmap.1).await?)
        .author(created_author))
}

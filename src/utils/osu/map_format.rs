use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::models::osu_files::OsuFile;
use crate::utils::misc::remove_trailing_zeros;
use crate::utils::osu::calculate::calculate;
use crate::utils::osu::misc::gamemode_from_string;
use crate::utils::osu::misc_format::{format_beatmap_link, format_mode_abbreviation};
use crate::Error;
use serenity::all::{Color, CreateEmbed};
use serenity::builder::CreateEmbedAuthor;
use std::collections::HashMap;
use std::env;
use std::sync::OnceLock;

static MAX_DIFF_LENGTH: OnceLock<usize> = OnceLock::new();

pub fn format_beatmapset(mut beatmaps: Vec<(Beatmap, OsuFile)>) -> Result<String, Error> {
    let mut diff_length = 0;
    let max_diff_length = MAX_DIFF_LENGTH.get_or_init(|| {
        env::var("MAX_DIFF_LENGTH")
            .unwrap_or_else(|_| String::from("19"))
            .parse::<usize>()
            .expect("Failed to parse max diff length.")
    });
    let mut calculated_beatmaps = HashMap::new();
    for (beatmap, osu_file) in &beatmaps {
        if beatmap.version.len() > diff_length {
            diff_length = beatmap.version.len();
        }
        let difficulty_values = calculate(None, beatmap, osu_file, None)?;
        calculated_beatmaps.insert(beatmap.id, difficulty_values);
    }
    if &diff_length > max_diff_length {
        max_diff_length.clone_into(&mut diff_length);
    } else if diff_length < 10 {
        diff_length = 10;
    }
    let mut formatted_beatmaps = format!(
        "```elm\nM   {:<diff_length$}  Stars  Drain  PP",
        "Difficulty"
    );

    beatmaps.sort_by(|a, b| {
        if let Some(difficulty_values) = calculated_beatmaps.get(&a.0.id) {
            &difficulty_values.total_stars
        } else {
            &a.0.difficulty_rating
        }
        .total_cmp(
            if let Some(difficulty_values) = calculated_beatmaps.get(&b.0.id) {
                &difficulty_values.total_stars
            } else {
                &b.0.difficulty_rating
            },
        )
    });

    for (beatmap, _) in beatmaps {
        let difficulty_values = calculated_beatmaps
            .get(&beatmap.id)
            .ok_or("Couldn't get beatmap difficulty values in format_beatmapset")?;

        let diff_name = if &beatmap.version.len() < max_diff_length {
            beatmap.version
        } else {
            let chars = beatmap.version.chars();
            let substring: String = chars.into_iter().take(max_diff_length - 3).collect();
            substring + "..."
        };

        let length = (beatmap.drain / 60, beatmap.drain % 60);
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

pub fn format_map_status(
    beatmapset_and_beatmap: (Beatmapset, Vec<(Beatmap, OsuFile)>),
    color: Color,
) -> Result<CreateEmbed<'static>, Error> {
    let beatmapset = beatmapset_and_beatmap.0;
    let header = format!("{} - {}", beatmapset.artist, beatmapset.title);

    let embed = CreateEmbed::new();

    let created_author =
        CreateEmbedAuthor::new(header).url(format_beatmap_link(None, beatmapset.id, None));

    Ok(embed
        .image(beatmapset.cover)
        .color(color)
        .description(format_beatmapset(beatmapset_and_beatmap.1)?)
        .author(created_author))
}

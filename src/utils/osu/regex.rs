use crate::utils::osu::misc::gamemode_from_string;
use crate::Error;
use lazy_static::lazy_static;
use regex::Regex;
use rosu_v2::prelude::GameMode;

lazy_static! {
    static ref BEATMAP_URL_PATTERN_V1: Regex =
        Regex::new(r"https?://(osu|old)\.ppy\.sh/(?P<type>[bs])/(?P<id>\d+)(?:\?m=(?P<mode>\d))?")
            .unwrap();
}

lazy_static! {
    static ref BEATMAPSET_URL_PATTERN_V2 : Regex = Regex::new(r"https?://osu\.ppy\.sh/beatmapsets/(?P<beatmapset_id>\d+)/?(?:#(?P<mode>\w+)/(?P<beatmap_id>\d+))?").unwrap();
}

lazy_static! {
    static ref BEATMAP_URL_PATTERN_V2: Regex =
        Regex::new(r"https?://osu\.ppy\.sh/beatmaps/(?P<beatmap_id>\d+)(?:\?mode=(?P<mode>\w+))?")
            .unwrap();
}

pub struct BeatmapInfo {
    pub beatmapset_id: Option<i64>,
    pub beatmap_id: Option<i64>,
    pub mode: Option<GameMode>,
}

pub fn get_beatmap_info(url: &str) -> Result<BeatmapInfo, Error> {
    if BEATMAP_URL_PATTERN_V2.is_match(url) {
        let info = BEATMAP_URL_PATTERN_V2
            .captures(url)
            .ok_or("Failed to get BEATMAP_URL_PATTERN_V2 captures in get_beatmap_info function")?;
        Ok(BeatmapInfo {
            beatmapset_id: None,
            beatmap_id: Some(
                info.name("beatmap_id")
                    .ok_or(
                        "Failed to get beatmap_id in BEATMAP_URL_PATTERN_V2 on get_beatmap_info function",
                    )?
                    .as_str()
                    .parse::<i64>()?,
            ),
            mode: gamemode_from_string(
                info.name("mode")
                    .ok_or(
                        "Failed to get mode in BEATMAP_URL_PATTERN_V2 on get_beatmap_info function",
                    )?
                    .as_str(),
            ),
        })
    } else if BEATMAPSET_URL_PATTERN_V2.is_match(url) {
        let info = BEATMAPSET_URL_PATTERN_V2.captures(url).ok_or(
            "Failed to get BEATMAPSET_URL_PATTERN_V2 captures in get_beatmap_info function",
        )?;
        Ok(BeatmapInfo {
            beatmapset_id: Some(
                info.name("beatmapset_id")
                    .ok_or(
                        "Failed to get beatmapset_id in BEATMAPSET_URL_PATTERN_V2 on get_beatmap_info function",
                    )?
                    .as_str()
                    .parse::<i64>()?,
            ),
            beatmap_id: Some(
                info.name("beatmap_id")
                    .ok_or(
                        "Failed to get beatmap_id in BEATMAPSET_URL_PATTERN_V2 on get_beatmap_info function",
                    )?
                    .as_str()
                    .parse::<i64>()?,
            ),
            mode: gamemode_from_string(info.name("mode").ok_or("Failed to get mode in BEATMAPSET_URL_PATTERN_V2 on get_beatmap_info function")?.as_str()),
        })
    } else if BEATMAP_URL_PATTERN_V1.is_match(url) {
        let info = BEATMAP_URL_PATTERN_V1
            .captures(url)
            .ok_or("Failed to get BEATMAP_URL_PATTERN_V1 captures in get_beatmap_info function")?;

        let mode = if let Some(mode) = info.name("mode") {
            Some(
                gamemode_from_string(mode.as_str())
                    .ok_or("Failed to parse mode in BEATMAP_URL_PATTERN_V1")?,
            )
        } else {
            None
        };

        if info
            .name("type")
            .ok_or(
                "Failed to get link type in BEATMAP_URL_PATTERN_V1 in get_beatmap_info function",
            )?
            .as_str()
            == "b"
        {
            Ok(BeatmapInfo {
                beatmapset_id: None,
                beatmap_id: Some(
                    info.name("id")
                        .ok_or(
                            "Failed to get beatmap_id in BEATMAP_URL_PATTERN_V1 (beatmap) on get_beatmap_info function",
                        )?
                        .as_str()
                        .parse::<i64>()?,
                ),
                mode,
            })
        } else {
            Ok(BeatmapInfo {
                beatmapset_id: Some(
                    info.name("id")
                        .ok_or(
                            "Failed to get beatmapset_id in BEATMAP_URL_PATTERN_V1 (beatmapset) on get_beatmap_info function",
                        )?
                        .as_str()
                        .parse::<i64>()?,
                ),
                beatmap_id: None,
                mode,
            })
        }
    } else {
        Ok(BeatmapInfo {
            beatmapset_id: None,
            beatmap_id: None,
            mode: None,
        })
    }
}

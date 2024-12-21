use crate::utils::osu::misc::gamemode_from_string;
use crate::Error;
use regex::Regex;
use rosu_v2::prelude::GameMode;
use std::sync::LazyLock;

static BEATMAP_URL_PATTERN_V1: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https?://(osu|old)\.ppy\.sh/(?P<type>[bs])/(?P<id>\d+)(?:\?m=(?P<mode>\d))?")
        .unwrap()
});

static BEATMAPSET_URL_PATTERN_V2: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https?://osu\.ppy\.sh/beatmapsets/(?P<beatmapset_id>\d+)/?(?:#(?P<mode>\w+)/(?P<beatmap_id>\d+))?").unwrap()
});

static BEATMAP_URL_PATTERN_V2: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https?://osu\.ppy\.sh/beatmaps/(?P<beatmap_id>\d+)(?:\?mode=(?P<mode>\w+))?")
        .unwrap()
});

pub struct BeatmapInfo {
    pub beatmapset_id: Option<i64>,
    pub beatmap_id: Option<i64>,
    pub mode: Option<GameMode>,
}

pub fn get_beatmap_info(url: &str) -> Result<BeatmapInfo, Error> {
    let beatmap_v1_pattern = &*BEATMAP_URL_PATTERN_V1;

    let beatmapset_v2_pattern = &*BEATMAPSET_URL_PATTERN_V2;

    let beatmap_v2_pattern = &*BEATMAP_URL_PATTERN_V2;
    if beatmap_v2_pattern.is_match(url) {
        let info = beatmap_v2_pattern
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
    } else if beatmapset_v2_pattern.is_match(url) {
        let info = beatmapset_v2_pattern.captures(url).ok_or(
            "Failed to get BEATMAPSET_URL_PATTERN_V2 captures in get_beatmap_info function",
        )?;
        if let Some(mode) = info.name("mode") {
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
                mode: gamemode_from_string(mode.as_str()),
            })
        } else {
            Ok(BeatmapInfo {
                beatmapset_id: Some(
                    info.name("beatmapset_id")
                        .ok_or(
                            "Failed to get beatmapset_id in BEATMAPSET_URL_PATTERN_V2 on get_beatmap_info function",
                        )?
                        .as_str()
                        .parse::<i64>()?,
                ),
                beatmap_id: None,
                mode: None,
            })
        }
    } else if beatmap_v1_pattern.is_match(url) {
        let info = beatmap_v1_pattern
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

use crate::utils::osu::misc::gamemode_from_string;
use lazy_static::lazy_static;
use regex::Regex;

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
    pub mode: Option<String>,
}

pub fn get_beatmap_info(url: &str) -> BeatmapInfo {
    if BEATMAP_URL_PATTERN_V2.is_match(url) {
        let info = BEATMAP_URL_PATTERN_V2.captures(url).unwrap();
        BeatmapInfo {
            beatmapset_id: None,
            beatmap_id: info
                .name("beatmap_id")
                .map(|id| id.as_str().parse::<i64>().unwrap()),
            mode: info.name("mode").map(|mode| mode.as_str().to_string()),
        }
    } else if BEATMAPSET_URL_PATTERN_V2.is_match(url) {
        let info = BEATMAPSET_URL_PATTERN_V2.captures(url).unwrap();
        BeatmapInfo {
            beatmapset_id: info
                .name("beatmapset_id")
                .map(|id| id.as_str().parse::<i64>().unwrap()),
            beatmap_id: info
                .name("beatmap_id")
                .map(|id| id.as_str().parse::<i64>().unwrap()),
            mode: info.name("mode").map(|mode| mode.as_str().to_string()),
        }
    } else if BEATMAP_URL_PATTERN_V1.is_match(url) {
        let info = BEATMAP_URL_PATTERN_V1.captures(url).unwrap();
        if info
            .name("type")
            .map(|map_type| map_type.as_str().parse::<String>().unwrap())
            .unwrap()
            == *"b"
        {
            BeatmapInfo {
                beatmapset_id: None,
                beatmap_id: info
                    .name("id")
                    .map(|id| id.as_str().parse::<i64>().unwrap()),
                mode: info
                    .name("mode")
                    .map(|mode| gamemode_from_string(mode.as_str()).unwrap().to_string()),
            }
        } else {
            BeatmapInfo {
                beatmapset_id: info
                    .name("id")
                    .map(|id| id.as_str().parse::<i64>().unwrap()),
                beatmap_id: None,
                mode: info
                    .name("mode")
                    .map(|mode| gamemode_from_string(mode.as_str()).unwrap().to_string()),
            }
        }
    } else {
        BeatmapInfo {
            beatmapset_id: None,
            beatmap_id: None,
            mode: None,
        }
    }
}

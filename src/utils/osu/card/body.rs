use crate::utils::misc::remove_trailing_zeros;
use crate::utils::osu::misc::get_score_rank;
use crate::Error;
use base64::engine::general_purpose;
use base64::Engine;
use num_format::{Locale, ToFormattedString};
use rosu_v2::prelude::{GradeCounts, UserExtended};
use svg::node::element::{Image, Mask, Rectangle, Text};
use svg::{Document, Node};

pub async fn draw_body(mut document: Document, osu_user: &UserExtended) -> Result<Document, Error> {
    document = draw_ranks(document, osu_user).await?;
    document = draw_statistics(document, osu_user)?;
    let grade_counts = if let Some(statistics) = &osu_user.statistics {
        statistics.grade_counts.clone()
    } else {
        GradeCounts {
            ss: 0,
            ssh: 0,
            s: 0,
            sh: 0,
            a: 0,
        }
    };
    document = draw_grades(document, &grade_counts).await?;
    Ok(document)
}

pub async fn draw_ranks(document: Document, osu_user: &UserExtended) -> Result<Document, Error> {
    let global_rank: String;
    let country_rank: String;
    if let Some(statistics) = osu_user.statistics.clone() {
        global_rank = format!(
            "#{}",
            statistics
                .global_rank
                .unwrap_or(0)
                .to_formatted_string(&Locale::en)
        );
        country_rank = format!(
            "#{}",
            statistics
                .country_rank
                .unwrap_or(0)
                .to_formatted_string(&Locale::en)
        );
    } else {
        global_rank = "-".to_string();
        country_rank = "-".to_string();
    }

    let score_rank: String = {
        let user_rank = get_score_rank(osu_user.user_id, osu_user.mode).await?;
        if user_rank == 0 {
            "-".into()
        } else {
            format!("#{}", user_rank.to_formatted_string(&Locale::en))
        }
    };

    let mut global_rank_text = Text::new()
        .set("id", "global_rank_text")
        .set("fill", "white")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 12)
        .set("letter-spacing", "0em")
        .set("x", 25)
        .set("y", 77.8);
    global_rank_text.append(svg::node::Text::new("Global Rank"));
    let mut global_rank_statistics = Text::new()
        .set("id", "global_rank_statistics")
        .set("fill", "white")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 24)
        .set("letter-spacing", "0em")
        .set("x", 22)
        .set("y", 103.1);
    global_rank_statistics.append(svg::node::Text::new(global_rank));

    let mut country_rank_text = Text::new()
        .set("id", "country_rank_text")
        .set("fill", "white")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 12)
        .set("letter-spacing", "0em")
        .set("x", 146)
        .set("y", 77.8);
    country_rank_text.append(svg::node::Text::new("Country Rank"));
    let mut country_rank_statistics = Text::new()
        .set("id", "country_rank_statistics")
        .set("fill", "white")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 24)
        .set("letter-spacing", "0em")
        .set("x", 143)
        .set("y", 103.1);
    country_rank_statistics.append(svg::node::Text::new(country_rank));

    let mut score_rank_text = Text::new()
        .set("id", "score_rank_text")
        .set("fill", "white")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 12)
        .set("letter-spacing", "0em")
        .set("x", 275)
        .set("y", 77.8);
    score_rank_text.append(svg::node::Text::new("Score Rank"));
    let mut score_rank_statistics = Text::new()
        .set("id", "score_rank_statistics")
        .set("fill", "white")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 24)
        .set("letter-spacing", "0em")
        .set("x", 272)
        .set("y", 103.1);
    score_rank_statistics.append(svg::node::Text::new(score_rank));

    Ok(document
        .add(global_rank_text)
        .add(global_rank_statistics)
        .add(country_rank_text)
        .add(country_rank_statistics)
        .add(score_rank_text)
        .add(score_rank_statistics))
}

pub fn draw_statistics(document: Document, osu_user: &UserExtended) -> Result<Document, Error> {
    let pp: String;
    let play_time: String;
    let play_count: String;
    let accuracy: String;
    let ranked_score: String;
    let total_score: String;
    let clears: String;
    if let Some(statistics) = &osu_user.statistics {
        pp = (statistics.pp as u32).to_formatted_string(&Locale::en);
        play_time = format!("{}h", statistics.playtime / 3600);
        play_count = statistics.playcount.to_formatted_string(&Locale::en);
        accuracy = format!("{}%", remove_trailing_zeros(statistics.accuracy.into(), 2)?);
        ranked_score = statistics.ranked_score.to_formatted_string(&Locale::en);
        total_score = statistics.total_score.to_formatted_string(&Locale::en);
        let clear_count = statistics.grade_counts.ssh
            + statistics.grade_counts.ss
            + statistics.grade_counts.sh
            + statistics.grade_counts.s
            + statistics.grade_counts.a;
        clears = clear_count.to_formatted_string(&Locale::en);
    } else {
        pp = "-".into();
        play_time = "-".into();
        play_count = "-".into();
        accuracy = "-".into();
        ranked_score = "-".into();
        total_score = "-".into();
        clears = "-".into();
    }

    let mut medal_count_text = Text::new()
        .set("id", "medal_count_text")
        .set("fill", "white")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 12)
        .set("letter-spacing", "0em")
        .set("x", 25)
        .set("y", 124.8);
    medal_count_text.append(svg::node::Text::new("Medals"));
    let mut medal_count_statistics = Text::new()
        .set("id", "medal_count_text")
        .set("fill", "#DBF0E9")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 14)
        .set("letter-spacing", "0em")
        .set("x", 25)
        .set("y", 140.1);
    medal_count_statistics.append(svg::node::Text::new(format!(
        "{}",
        osu_user.medals.clone().unwrap_or_default().len()
    )));

    let mut pp_text = Text::new()
        .set("id", "pp_text")
        .set("fill", "white")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 12)
        .set("letter-spacing", "0em")
        .set("x", 86)
        .set("y", 124.8);
    pp_text.append(svg::node::Text::new("PP"));
    let mut pp_statistics = Text::new()
        .set("id", "pp_statistics")
        .set("fill", "#DBF0E9")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 14)
        .set("letter-spacing", "0em")
        .set("x", 86)
        .set("y", 140.1);
    pp_statistics.append(svg::node::Text::new(pp));

    let mut play_time_text = Text::new()
        .set("id", "play_time_text")
        .set("fill", "white")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 12)
        .set("letter-spacing", "0em")
        .set("x", 140)
        .set("y", 124.8);
    play_time_text.append(svg::node::Text::new("Play Time"));
    let mut play_time_statistics = Text::new()
        .set("id", "play_time_statistics")
        .set("fill", "#DBF0E9")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 14)
        .set("letter-spacing", "0em")
        .set("x", 140)
        .set("y", 140.1);
    play_time_statistics.append(svg::node::Text::new(play_time));

    let mut play_count_text = Text::new()
        .set("id", "play_count_text")
        .set("fill", "white")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 12)
        .set("letter-spacing", "0em")
        .set("x", 220)
        .set("y", 124.8);
    play_count_text.append(svg::node::Text::new("Play Count"));
    let mut play_count_statistics = Text::new()
        .set("id", "play_count_statistics")
        .set("fill", "#DBF0E9")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 14)
        .set("letter-spacing", "0em")
        .set("x", 220)
        .set("y", 140.1);
    play_count_statistics.append(svg::node::Text::new(play_count));

    let mut accuracy_text = Text::new()
        .set("id", "accuracy_text")
        .set("fill", "white")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 12)
        .set("letter-spacing", "0em")
        .set("x", 302)
        .set("y", 124.8);
    accuracy_text.append(svg::node::Text::new("Accuracy"));
    let mut accuracy_statistics = Text::new()
        .set("id", "accuracy_statistics")
        .set("fill", "#DBF0E9")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 14)
        .set("letter-spacing", "0em")
        .set("x", 302)
        .set("y", 140.1);
    accuracy_statistics.append(svg::node::Text::new(accuracy));

    let mut ranked_score_text = Text::new()
        .set("id", "ranked_score_text")
        .set("fill", "white")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 12)
        .set("letter-spacing", "0em")
        .set("x", 41)
        .set("y", 157.8);
    ranked_score_text.append(svg::node::Text::new("Ranked Score"));
    let mut ranked_score_statistics = Text::new()
        .set("id", "ranked_score_statistics")
        .set("fill", "#DBF0E9")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 14)
        .set("letter-spacing", "0em")
        .set("x", 41)
        .set("y", 173.1);
    ranked_score_statistics.append(svg::node::Text::new(ranked_score));

    let mut total_score_text = Text::new()
        .set("id", "total_score_text")
        .set("fill", "white")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 12)
        .set("letter-spacing", "0em")
        .set("x", 161)
        .set("y", 157.8);
    total_score_text.append(svg::node::Text::new("Total Score"));
    let mut total_score_statistics = Text::new()
        .set("id", "total_score_statistics")
        .set("fill", "#DBF0E9")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 14)
        .set("letter-spacing", "0em")
        .set("x", 161)
        .set("y", 173.1);
    total_score_statistics.append(svg::node::Text::new(total_score));

    let mut clears_text = Text::new()
        .set("id", "clears_text")
        .set("fill", "white")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 12)
        .set("letter-spacing", "0em")
        .set("x", 285)
        .set("y", 157.8);
    clears_text.append(svg::node::Text::new("Clears"));
    let mut clears_statistics = Text::new()
        .set("id", "clears_statistics")
        .set("fill", "#DBF0E9")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 14)
        .set("letter-spacing", "0em")
        .set("x", 285)
        .set("y", 173.1);
    clears_statistics.append(svg::node::Text::new(clears));

    Ok(document
        .add(medal_count_text)
        .add(medal_count_statistics)
        .add(pp_text)
        .add(pp_statistics)
        .add(play_time_text)
        .add(play_time_statistics)
        .add(play_count_text)
        .add(play_count_statistics)
        .add(accuracy_text)
        .add(accuracy_statistics)
        .add(ranked_score_text)
        .add(ranked_score_statistics)
        .add(total_score_text)
        .add(total_score_statistics)
        .add(clears_text)
        .add(clears_statistics))
}

//noinspection ALL
pub async fn draw_grades(document: Document, grades: &GradeCounts) -> Result<Document, Error> {
    let ssh_rectangle = Rectangle::new()
        .set("id", "ssh_rectangle")
        .set("x", 87)
        .set("y", 194)
        .set("width", 32)
        .set("height", 16)
        .set("rx", 8)
        .set("fill", "white");

    let ssh_mask = Mask::new().set("id", "ssh_mask").add(ssh_rectangle);

    let ssh_fs_image = tokio::fs::read("src/utils/osu/card/assets/grades/XH.png").await?;

    let ssh_image_base64 = general_purpose::STANDARD.encode(ssh_fs_image);

    let ssh_image = Image::new()
        .set("x", 87)
        .set("y", 194)
        .set("width", 32)
        .set("height", 16)
        .set(
            "xlink:href",
            format!("data:image/png;charset=utf-8;base64,{ssh_image_base64}"),
        )
        .set("mask", "url(#ssh_mask)");

    let mut ssh_text = Text::new()
        .set("fill", "#DBF0E9")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 12)
        .set("x", 103)
        .set("y", 221.8)
        .set("dominant-baseline", "middle")
        .set("text-anchor", "middle");

    ssh_text.append(svg::node::Text::new(
        grades.ssh.to_formatted_string(&Locale::en),
    ));

    let ss_rectangle = Rectangle::new()
        .set("id", "ss_rectangle")
        .set("x", 129)
        .set("y", 194)
        .set("width", 32)
        .set("height", 16)
        .set("rx", 8)
        .set("fill", "white");

    let ss_mask = Mask::new().set("id", "ss_mask").add(ss_rectangle);

    let ss_fs_image = tokio::fs::read("src/utils/osu/card/assets/grades/X.png").await?;

    let ss_image_base64 = general_purpose::STANDARD.encode(ss_fs_image);

    let ss_image = Image::new()
        .set("x", 129)
        .set("y", 194)
        .set("width", 32)
        .set("height", 16)
        .set(
            "xlink:href",
            format!("data:image/png;charset=utf-8;base64,{ss_image_base64}"),
        )
        .set("mask", "url(#ss_mask)");

    let mut ss_text = Text::new()
        .set("fill", "#DBF0E9")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 12)
        .set("x", 145)
        .set("y", 221.8)
        .set("dominant-baseline", "middle")
        .set("text-anchor", "middle");

    ss_text.append(svg::node::Text::new(
        grades.ss.to_formatted_string(&Locale::en),
    ));

    let sh_rectangle = Rectangle::new()
        .set("id", "sh_rectangle")
        .set("x", 174)
        .set("y", 194)
        .set("width", 32)
        .set("height", 16)
        .set("rx", 8)
        .set("fill", "white");

    let sh_mask = Mask::new().set("id", "sh_mask").add(sh_rectangle);

    let sh_fs_image = tokio::fs::read("src/utils/osu/card/assets/grades/SH.png").await?;

    let sh_image_base64 = general_purpose::STANDARD.encode(sh_fs_image);

    let sh_image = Image::new()
        .set("x", 171)
        .set("y", 194)
        .set("width", 32)
        .set("height", 16)
        .set(
            "xlink:href",
            format!("data:image/png;charset=utf-8;base64,{sh_image_base64}"),
        )
        .set("mask", "url(#sh_mask)");

    let mut sh_text = Text::new()
        .set("fill", "#DBF0E9")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 12)
        .set("x", 187)
        .set("y", 221.8)
        .set("dominant-baseline", "middle")
        .set("text-anchor", "middle");

    sh_text.append(svg::node::Text::new(
        grades.sh.to_formatted_string(&Locale::en),
    ));

    let s_rectangle = Rectangle::new()
        .set("id", "s_rectangle")
        .set("x", 213)
        .set("y", 194)
        .set("width", 32)
        .set("height", 16)
        .set("rx", 8)
        .set("fill", "white");

    let s_mask = Mask::new().set("id", "s_mask").add(s_rectangle);

    let s_fs_image = tokio::fs::read("src/utils/osu/card/assets/grades/S.png").await?;

    let s_image_base64 = general_purpose::STANDARD.encode(s_fs_image);

    let s_image = Image::new()
        .set("x", 213)
        .set("y", 194)
        .set("width", 32)
        .set("height", 16)
        .set(
            "xlink:href",
            format!("data:image/png;charset=utf-8;base64,{s_image_base64}"),
        )
        .set("mask", "url(#s_mask)");

    let mut s_text = Text::new()
        .set("fill", "#DBF0E9")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 12)
        .set("x", 229)
        .set("y", 221.8)
        .set("dominant-baseline", "middle")
        .set("text-anchor", "middle");

    s_text.append(svg::node::Text::new(
        grades.s.to_formatted_string(&Locale::en),
    ));

    let a_rectangle = Rectangle::new()
        .set("id", "a_rectangle")
        .set("x", 255)
        .set("y", 194)
        .set("width", 32)
        .set("height", 16)
        .set("rx", 8)
        .set("fill", "white");

    let a_mask = Mask::new().set("id", "a_mask").add(a_rectangle);

    let a_fs_image = tokio::fs::read("src/utils/osu/card/assets/grades/A.png").await?;

    let a_image_base64 = general_purpose::STANDARD.encode(a_fs_image);

    let a_image = Image::new()
        .set("x", 255)
        .set("y", 194)
        .set("width", 32)
        .set("height", 16)
        .set(
            "xlink:href",
            format!("data:image/png;charset=utf-8;base64,{a_image_base64}"),
        )
        .set("mask", "url(#a_mask)");

    let mut a_text = Text::new()
        .set("fill", "#DBF0E9")
        .set("xml:space", "preserve")
        .set("style", "white-space: pre")
        .set("font-family", "Torus")
        .set("font-size", 12)
        .set("x", 271)
        .set("y", 221.8)
        .set("dominant-baseline", "middle")
        .set("text-anchor", "middle");

    a_text.append(svg::node::Text::new(
        grades.a.to_formatted_string(&Locale::en),
    ));

    Ok(document
        .add(ssh_mask)
        .add(ssh_image)
        .add(ssh_text)
        .add(ss_mask)
        .add(ss_image)
        .add(ss_text)
        .add(sh_mask)
        .add(sh_image)
        .add(sh_text)
        .add(s_mask)
        .add(s_image)
        .add(s_text)
        .add(a_mask)
        .add(a_image)
        .add(a_text))
}

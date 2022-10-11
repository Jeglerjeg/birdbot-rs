// @generated automatically by Diesel CLI.

diesel::table! {
    beatmaps (id) {
        id -> Int8,
        ar -> Float8,
        beatmapset_id -> Int8,
        checksum -> Nullable<Varchar>,
        max_combo -> Int4,
        bpm -> Float8,
        convert -> Bool,
        count_circles -> Int4,
        count_sliders -> Int4,
        count_spinners -> Int4,
        cs -> Float8,
        difficulty_rating -> Float8,
        drain -> Int4,
        mode -> Varchar,
        passcount -> Int4,
        playcount -> Int4,
        status -> Varchar,
        total_length -> Int4,
        user_id -> Int8,
        version -> Varchar,
        time_cached -> Timestamptz,
    }
}

diesel::table! {
    beatmapsets (id) {
        id -> Int8,
        artist -> Varchar,
        bpm -> Float8,
        list_cover -> Text,
        cover -> Text,
        creator -> Varchar,
        play_count -> Int8,
        source -> Varchar,
        status -> Varchar,
        title -> Varchar,
        user_id -> Int8,
        time_cached -> Timestamptz,
    }
}

diesel::table! {
    linked_osu_profiles (id) {
        id -> Int8,
        osu_id -> Int8,
        home_guild -> Int8,
        mode -> Varchar,
    }
}

diesel::table! {
    osu_guild_channels (guild_id) {
        guild_id -> Int8,
        score_channel -> Nullable<Int8>,
        map_channel -> Nullable<Int8>,
    }
}

diesel::table! {
    osu_users (id) {
        id -> Int8,
        username -> Text,
        avatar_url -> Text,
        country_code -> Varchar,
        mode -> Varchar,
        pp -> Float8,
        accuracy -> Float8,
        country_rank -> Int4,
        global_rank -> Int4,
        max_combo -> Int4,
        ranked_score -> Int8,
        ticks -> Int4,
        time_cached -> Timestamptz,
    }
}

diesel::table! {
    prefix (guild_id) {
        guild_id -> Int8,
        guild_prefix -> Bpchar,
    }
}

diesel::table! {
    questions (id) {
        id -> Int4,
        choice1 -> Text,
        choice2 -> Text,
        choice1_answers -> Int4,
        choice2_answers -> Int4,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    beatmaps,
    beatmapsets,
    linked_osu_profiles,
    osu_guild_channels,
    osu_users,
    prefix,
    questions,
);

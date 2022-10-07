// @generated automatically by Diesel CLI.

diesel::table! {
    beatmaps (id) {
        id -> BigInt,
        ar -> Float,
        beatmapset_id -> BigInt,
        checksum -> Text,
        max_combo -> Integer,
        bpm -> Float,
        convert -> Bool,
        count_circles -> Integer,
        count_sliders -> Integer,
        count_spinners -> Integer,
        cs -> Float,
        difficulty_rating -> Float,
        drain -> Integer,
        mode -> Text,
        passcount -> Integer,
        playcount -> Integer,
        status -> Text,
        total_length -> Integer,
        user_id -> BigInt,
        version -> Text,
        time_cached -> Timestamp,
    }
}

diesel::table! {
    beatmapsets (id) {
        id -> BigInt,
        artist -> Text,
        bpm -> Float,
        list_cover -> Text,
        cover -> Text,
        creator -> Text,
        play_count -> BigInt,
        source -> Text,
        status -> Text,
        title -> Text,
        user_id -> BigInt,
        time_cached -> Timestamp,
    }
}

diesel::table! {
    linked_osu_profiles (id) {
        id -> BigInt,
        osu_id -> BigInt,
        home_guild -> BigInt,
        mode -> Text,
    }
}

diesel::table! {
    osu_files (id) {
        id -> BigInt,
        data -> Binary,
    }
}

diesel::table! {
    prefix (guild_id) {
        guild_id -> BigInt,
        guild_prefix -> Text,
    }
}

diesel::table! {
    questions (id) {
        id -> Integer,
        choice1 -> Text,
        choice2 -> Text,
        choice1_answers -> Integer,
        choice2_answers -> Integer,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    beatmaps,
    beatmapsets,
    linked_osu_profiles,
    osu_files,
    prefix,
    questions,
);

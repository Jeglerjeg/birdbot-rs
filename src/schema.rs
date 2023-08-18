// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "tsvector", schema = "pg_catalog"))]
    pub struct Tsvector;
}

diesel::table! {
    beatmaps (id) {
        id -> Int8,
        ar -> Float8,
        beatmapset_id -> Int8,
        #[max_length = 32]
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
        #[max_length = 7]
        mode -> Varchar,
        passcount -> Int4,
        playcount -> Int4,
        #[max_length = 9]
        status -> Varchar,
        total_length -> Int4,
        user_id -> Int8,
        #[max_length = 90]
        version -> Varchar,
        time_cached -> Timestamptz,
    }
}

diesel::table! {
    beatmapsets (id) {
        id -> Int8,
        #[max_length = 80]
        artist -> Varchar,
        bpm -> Float8,
        list_cover -> Text,
        cover -> Text,
        #[max_length = 80]
        creator -> Varchar,
        play_count -> Int8,
        #[max_length = 200]
        source -> Varchar,
        #[max_length = 9]
        status -> Varchar,
        #[max_length = 80]
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
        #[max_length = 7]
        mode -> Varchar,
    }
}

diesel::table! {
    osu_files (id) {
        id -> Int8,
        file -> Bytea,
    }
}

diesel::table! {
    osu_guild_channels (guild_id) {
        guild_id -> Int8,
        score_channel -> Nullable<Array<Nullable<Int8>>>,
        map_channel -> Nullable<Array<Nullable<Int8>>>,
    }
}

diesel::table! {
    osu_notifications (id) {
        id -> Int8,
        last_pp -> Timestamptz,
        last_event -> Timestamptz,
    }
}

diesel::table! {
    osu_users (id) {
        id -> Int8,
        username -> Text,
        avatar_url -> Text,
        #[max_length = 2]
        country_code -> Varchar,
        #[max_length = 7]
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
        #[max_length = 1]
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

diesel::table! {
    summary_enabled_guilds (id) {
        id -> Int8,
        guild_id -> Int8,
        channel_ids -> Array<Nullable<Int8>>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Tsvector;

    summary_messages (id) {
        id -> Int8,
        #[max_length = 4000]
        content -> Bpchar,
        discord_id -> Int8,
        author_id -> Int8,
        channel_id -> Int8,
        is_bot -> Bool,
        ts -> Tsvector,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    beatmaps,
    beatmapsets,
    linked_osu_profiles,
    osu_files,
    osu_guild_channels,
    osu_notifications,
    osu_users,
    prefix,
    questions,
    summary_enabled_guilds,
    summary_messages,
);

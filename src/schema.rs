// @generated automatically by Diesel CLI.

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

diesel::allow_tables_to_appear_in_same_query!(prefix, questions,);

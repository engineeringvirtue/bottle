table! {
    ban (user) {
        report -> Nullable<Int8>,
        user -> Int8,
    }
}

table! {
    bottle (id) {
        id -> Int8,
        reply_to -> Nullable<Int8>,
        user -> Int8,
        message -> Int8,
        guild -> Nullable<Int8>,
        time_pushed -> Timestamp,
        contents -> Text,
        url -> Nullable<Text>,
        image -> Nullable<Text>,
    }
}

table! {
    guild (id) {
        id -> Int8,
        invite -> Nullable<Text>,
        bottle_channel -> Nullable<Int8>,
        admin_channel -> Nullable<Int8>,
    }
}

table! {
    guild_bottle (id) {
        id -> Int8,
        bottle -> Int8,
        guild -> Int8,
        message -> Int8,
        time_recieved -> Timestamp,
    }
}

table! {
    guild_contribution (guild, user) {
        guild -> Int8,
        user -> Int8,
        xp -> Int4,
    }
}

table! {
    report (bottle) {
        bottle -> Int8,
        message -> Int8,
        user -> Int8,
    }
}

table! {
    user (id) {
        id -> Int8,
        session -> Nullable<Uuid>,
        xp -> Int4,
        admin -> Bool,
    }
}

table! {
    user_rank (id) {
        id -> Int8,
        rank -> Int8,
    }
}

table! {
    guild_rank (id) {
        id -> Int8,
        rank -> Int8,
    }
}

joinable!(ban -> report (report));
joinable!(ban -> user (user));
joinable!(bottle -> guild (guild));
joinable!(bottle -> user (user));
joinable!(guild_bottle -> bottle (bottle));
joinable!(guild_bottle -> guild (guild));
joinable!(guild_contribution -> guild (guild));
joinable!(guild_contribution -> user (user));
joinable!(report -> bottle (bottle));
joinable!(report -> user (user));

allow_tables_to_appear_in_same_query!(
    ban,
    bottle,
    guild,
    guild_bottle,
    guild_contribution,
    report,
    user,
);

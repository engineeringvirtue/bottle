table! {
    ban (user) {
        report -> Int8,
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
    report (bottle) {
        bottle -> Int8,
        user -> Int8,
    }
}

table! {
    user (id) {
        id -> Int8,
        session -> Nullable<Uuid>,
        token -> Nullable<Text>,
        xp -> Int4,
        admin -> Bool,
    }
}

joinable!(ban -> report (report));
joinable!(ban -> user (user));
joinable!(bottle -> guild (guild));
joinable!(bottle -> user (user));
joinable!(guild_bottle -> bottle (bottle));
joinable!(guild_bottle -> guild (guild));
joinable!(report -> bottle (bottle));
joinable!(report -> user (user));

allow_tables_to_appear_in_same_query!(
    ban,
    bottle,
    guild,
    guild_bottle,
    report,
    user,
);

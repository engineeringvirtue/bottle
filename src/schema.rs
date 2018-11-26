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
        channel -> Int8,
    }
}

table! {
    guild (id) {
        id -> Int8,
        invite -> Nullable<Text>,
        bottle_channel -> Nullable<Int8>,
        admin_channel -> Nullable<Int8>,
        prefix -> Nullable<Bpchar>,
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
    received_bottle (id) {
        id -> Int8,
        bottle -> Int8,
        message -> Int8,
        time_recieved -> Timestamp,
        channel -> Int8,
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
        tickets -> Int4,
    }
}

joinable!(ban -> report (report));
joinable!(ban -> user (user));
joinable!(bottle -> guild (guild));
joinable!(bottle -> user (user));
joinable!(guild_contribution -> guild (guild));
joinable!(guild_contribution -> user (user));
joinable!(received_bottle -> bottle (bottle));
joinable!(report -> bottle (bottle));
joinable!(report -> user (user));

allow_tables_to_appear_in_same_query!(
    ban,
    bottle,
    guild,
    guild_contribution,
    received_bottle,
    report,
    user,
);

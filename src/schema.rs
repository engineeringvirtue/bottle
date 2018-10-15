table! {
    bottle (bottleid) {
        bottleid -> Int8,
        user -> Int8,
        reply_to -> Nullable<Int8>,
        messageid -> Int8,
        time_pushed -> Timestamp,
        message -> Text,
    }
}

table! {
    bottle_user (bottle, user) {
        bottle -> Int8,
        user -> Int8,
        messageid -> Int8,
        time_recieved -> Timestamp,
    }
}

table! {
    guild (guildid) {
        guildid -> Int8,
        admin_channel -> Int8,
    }
}

table! {
    report (reportid) {
        reportid -> Int8,
        bottle -> Int8,
        guild -> Int8,
        messageid -> Int8,
        user -> Int8,
    }
}

table! {
    user (userid) {
        userid -> Int8,
        subscribed -> Bool,
        token -> Nullable<Text>,
        xp -> Int8,
    }
}

joinable!(bottle -> user (user));
joinable!(bottle_user -> bottle (bottle));
joinable!(bottle_user -> user (user));
joinable!(report -> bottle (bottle));
joinable!(report -> guild (guild));
joinable!(report -> user (user));

allow_tables_to_appear_in_same_query!(
    bottle,
    bottle_user,
    guild,
    report,
    user,
);

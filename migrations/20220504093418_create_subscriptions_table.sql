-- Add migration script here
create table subscriptions
(
    id            uuid        NOT NULL,
    PRIMARY KEY (id),
    email         text        not null unique,
    name          text        not null,
    subscribed_at timestamptz not null
)
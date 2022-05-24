-- Your SQL goes here
create table pipeline (
    id text primary key not null,
    name text not null,
    date_created default current_timestamp not null
);

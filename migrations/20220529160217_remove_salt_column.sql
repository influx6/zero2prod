-- Add migration script here
ALTER table users
    DROP COLUMN salt;

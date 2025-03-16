CREATE TABLE IF NOT EXISTS poller_states (
    chat_id BIGINT NOT NULL,
    repository_full_name TEXT NOT NULL,
    last_poll_time INTEGER NOT NULL,
    PRIMARY KEY (chat_id, repository_full_name)
);
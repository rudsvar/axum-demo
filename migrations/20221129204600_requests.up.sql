CREATE TABLE requests (
    id SERIAL PRIMARY KEY,
    client TEXT,
    server TEXT,
    uri TEXT NOT NULL,
    request_body TEXT,
    response_body TEXT,
    status INT NOT NULL,
    timestamp timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP
);

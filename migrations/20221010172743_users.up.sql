CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(100) NOT NULL UNIQUE,
    password VARCHAR(100) NOT NULL,
    role TEXT NOT NULL
);

INSERT INTO users (username, password, role) VALUES ('user', '$2a$12$xRMhTHN8I5m1AUnbftRJTOqh2LOu4nTMvEF2Awq.uWwKbK96N5ZF6', 'user');
INSERT INTO users (username, password, role) VALUES ('admin', '$2a$12$Rz0QWfY9mH16RookmBBiBu6rlOcwPDHSjJckyApNiFWxEb24TuWXS', 'admin');

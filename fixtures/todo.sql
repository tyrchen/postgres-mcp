CREATE TABLE todo(
  id serial PRIMARY KEY,
  title varchar(255) NOT NULL,
  completed boolean NOT NULL DEFAULT FALSE
);

INSERT INTO todo(title)
  VALUES ('Buy groceries');

INSERT INTO todo(title)
  VALUES ('Finish the MCP server');

INSERT INTO todo(title)
  VALUES ('Learn about MCP');

UPDATE
  todo
SET
  completed = TRUE
WHERE
  id = 1;

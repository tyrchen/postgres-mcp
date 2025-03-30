# PG Agent

This is a simple agent that uses the PostgreSQL MCP server to answer questions about the data.

## Usage

```bash
uv run pg_agent.py
```

Then you could ask questions about the database.

Example:

```bash
➜ uv run pg_agent.py
View trace: https://platform.openai.com/traces/trace_02fec679792c4c6abc4b80b24dc74a2d

Enter a question: connect to todo db. show me the conn id
Connected to the "todo" database. The connection ID is `157df556-f523-4bcd-96f1-2237a89c96bc`.
Enter a question: list all tables
There is one table in the "todo" database:

- **Table Name:** todo
- **Total Rows:** 4
Enter a question: show all unfinished todos
Error invoking MCP tool query: error returned from database: column "finished" does not exist
Enter a question: describe todo table
Here are the unfinished todos:

1. **ID:** 2
   - **Title:** Finish the MCP server
   - **Completed:** false

2. **ID:** 3
   - **Title:** Learn about MCP
   - **Completed:** false

3. **ID:** 4
   - **Title:** see doctor
   - **Completed:** false
Enter a question: describe todo table
The structure of the `todo` table is as follows:

1. **id**
   - **Data Type:** integer
   - **Nullable:** NO
   - **Default:** `nextval('todo_id_seq'::regclass)`

2. **title**
   - **Data Type:** character varying
   - **Length:** 255
   - **Nullable:** NO

3. **completed**
   - **Data Type:** boolean
   - **Nullable:** NO
   - **Default:** `false`
Enter a question: I just finished MCP server
Great! The "Finish the MCP server" task has been marked as completed.
```

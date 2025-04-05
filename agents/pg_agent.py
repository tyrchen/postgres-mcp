import asyncio
import os
import subprocess
from typing import Optional

from agents import Agent, Runner, gen_trace_id, trace
from agents.mcp import MCPServer, MCPServerStdio


class PostgresMCPServer(MCPServerStdio):
    """PostgreSQL MCP Server implementation"""

    def __init__(self):
        postgres_mcp_path = "postgres-mcp"

        super().__init__(
            name="PostgreSQL MCP Server",
            params={
                "command": postgres_mcp_path,
                "args": ["stdio"],
            },
        )


async def run(mcp_server: MCPServer):
    agent = Agent(
        name="PostgreSQL Assistant",
        instructions="Use the tools to interact with PostgreSQL database and perform operations like querying, creating tables, and managing indexes.",
        mcp_servers=[mcp_server],
    )

    context = []
    context.append(
        "SYSTEM: The following questions are about the database server 'postgres://postgres:postgres@localhost:5432'. Please answer the questions in details with all the information the tool provides."
    )

    while True:
        # get user input from CLI
        question = input("Enter a question: ")
        context.append(f"USER: {question}")

        # run the agent
        try:
            result = await Runner.run(starting_agent=agent, input="\n\n".join(context))
            context.append(f"ASSISTANT: {result.final_output}")
        except Exception as e:
            continue

        print(result.final_output)


async def main():
    async with PostgresMCPServer() as server:
        trace_id = gen_trace_id()
        with trace(workflow_name="PostgreSQL MCP Agent", trace_id=trace_id):
            print(f"View trace: https://platform.openai.com/traces/{trace_id}\n")
            await run(server)


if __name__ == "__main__":
    asyncio.run(main())

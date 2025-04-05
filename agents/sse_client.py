import asyncio
import aiohttp
import logging


async def test_sse_endpoint():
    async with aiohttp.ClientSession() as session:
        async with session.get("http://localhost:3000/sse") as response:
            print(f"Status: {response.status}")
            print(f"Headers: {response.headers}")

            async for line in response.content:
                line = line.decode("utf-8").strip()
                if line:
                    print(f"Received: {line}")
                    if "heartbeat" in line:
                        print("SSE test successful!")


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    asyncio.run(test_sse_endpoint())

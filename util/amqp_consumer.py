#!/usr/bin/env python3
import argparse
import asyncio
import json
import logging
import sys

import aio_pika

LOGGING_FORMAT = "[%(levelname)s] : %(message)s"


async def amain() -> int:
    loop = asyncio.get_event_loop()

    parser = argparse.ArgumentParser(prog="amqp_consumer")
    parser.add_argument(
        "-u", "--uri", default="amqp://guest:guest@localhost:5672/", help="amqp_uri"
    )
    parser.add_argument("-e", "--exchange", default="", help="exchange name")
    parser.add_argument("-r", "--routing-key", default="", help="routing key")
    parser.add_argument("-d", "--debug", action="store_true")
    args = parser.parse_args()
    logging.basicConfig(
        stream=sys.stderr,
        format=LOGGING_FORMAT,
        level=("DEBUG" if args.debug else "INFO"),
    )

    conn: aio_pika.Connection = await aio_pika.connect_robust(url=args.uri, loop=loop)
    async with conn:
        channel: aio_pika.Channel = await conn.channel()
        queue: aio_pika.Queue = await channel.declare_queue(
            durable=False, auto_delete=True
        )
        await queue.bind(
            exchange=await channel.get_exchange(name=args.exchange),
            routing_key=args.routing_key,
        )
        async with queue.iterator() as queue_iter:
            async for message in queue_iter:
                async with message.process():
                    print(message.body.decode("ascii"))

    return 0


def main():
    asyncio.run(amain())


if __name__ == "__main__":
    main()

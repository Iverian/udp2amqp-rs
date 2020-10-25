#!/usr/bin/env python3
import argparse
import itertools
import json
import logging
import socket
import sys
import time
from datetime import datetime
from typing import Iterator, Tuple
from urllib.parse import urlparse

LOGGING_FORMAT = "[%(levelname)s] : %(message)s"
WAIT_INTERVAL = 0.01


def parse_udp_url(url: str) -> Tuple[str, int]:
    parsed = urlparse(url, scheme="udp", allow_fragments=False)
    assert parsed.hostname and parsed.port

    return (parsed.hostname, parsed.port)


def main():
    parser = argparse.ArgumentParser(prog="udp_producer")
    parser.add_argument(
        "url", type=str, help="address to bind in format udp://{host}:{port}"
    )
    parser.add_argument(
        "-n",
        "--message-number",
        type=int,
        default=0,
        help="number of messages to produce (0 is unlimited)",
    )
    parser.add_argument("-d", "--debug", action="store_true")
    args = parser.parse_args()
    logging.basicConfig(
        stream=sys.stderr,
        format=LOGGING_FORMAT,
        level=("DEBUG" if args.debug else "INFO"),
    )

    addr = parse_udp_url(args.url)
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    iterator: Iterator[int] = (
        iter(range(args.message_number)) if args.message_number else itertools.count()
    )
    try:
        logging.info("sending data to %s", args.url)
        for i in iterator:
            data = json.dumps(
                {"event_time": datetime.utcnow().isoformat(), "index": i},
                ensure_ascii=True,
            ).encode("ascii")
            logging.debug("sending data: %r", data)
            sock.sendto(data, addr)
            time.sleep(WAIT_INTERVAL)
    finally:
        sock.close()


if __name__ == "__main__":
    main()

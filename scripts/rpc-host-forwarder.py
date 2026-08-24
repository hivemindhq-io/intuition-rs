#!/usr/bin/env python3
"""Host-side JSON-RPC forwarder with rate limiting for Docker crawlers.

Containers call http://host.docker.internal:<port>; this process paces
upstream HTTPS calls so public RPCs are less likely to bandwidth-ban us.
"""
import argparse
import http.server
import ssl
import sys
import threading
import time
import urllib.error
import urllib.request

CTX = ssl._create_unverified_context()


class RateLimiter:
    def __init__(self, min_interval_s: float, max_inflight: int):
        self.min_interval_s = min_interval_s
        self.sem = threading.Semaphore(max_inflight)
        self.lock = threading.Lock()
        self.next_allowed = 0.0

    def acquire(self):
        self.sem.acquire()
        with self.lock:
            now = time.monotonic()
            wait = self.next_allowed - now
            if wait > 0:
                time.sleep(wait)
                now = time.monotonic()
            self.next_allowed = now + self.min_interval_s

    def release(self):
        self.sem.release()


class Handler(http.server.BaseHTTPRequestHandler):
    upstream = None
    limiter = None

    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(length) if length else b""
        req = urllib.request.Request(
            self.upstream,
            data=body,
            headers={
                "Content-Type": "application/json",
                "Accept": "application/json",
                "User-Agent": "curl/8.7.1",
            },
            method="POST",
        )
        self.limiter.acquire()
        try:
            with urllib.request.urlopen(req, timeout=90, context=CTX) as resp:
                data = resp.read()
                self.send_response(resp.status)
                self.send_header(
                    "Content-Type", resp.headers.get("Content-Type", "application/json")
                )
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)
        except urllib.error.HTTPError as e:
            data = e.read()
            self.send_response(e.code)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)
        except Exception as e:
            msg = (
                '{"jsonrpc":"2.0","id":null,"error":{"code":-32000,"message":%r}}'
                % (str(e),)
            ).encode()
            self.send_response(502)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(msg)))
            self.end_headers()
            self.wfile.write(msg)
        finally:
            self.limiter.release()

    def log_message(self, fmt, *args):
        sys.stderr.write("%s - %s\n" % (self.address_string(), fmt % args))


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--listen", default="0.0.0.0")
    p.add_argument("--port", type=int, required=True)
    p.add_argument("--upstream", required=True)
    p.add_argument(
        "--min-interval-ms",
        type=int,
        default=120,
        help="Minimum ms between upstream RPC calls (default 120)",
    )
    p.add_argument(
        "--max-inflight",
        type=int,
        default=1,
        help="Max concurrent upstream calls (default 1)",
    )
    args = p.parse_args()
    Handler.upstream = args.upstream
    Handler.limiter = RateLimiter(args.min_interval_ms / 1000.0, args.max_inflight)
    print(
        f"forwarding :{args.port} -> {args.upstream} "
        f"(min_interval={args.min_interval_ms}ms, max_inflight={args.max_inflight})",
        flush=True,
    )
    http.server.ThreadingHTTPServer((args.listen, args.port), Handler).serve_forever()


if __name__ == "__main__":
    main()

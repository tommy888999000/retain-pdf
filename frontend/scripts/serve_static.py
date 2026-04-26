#!/usr/bin/env python3
from __future__ import annotations

import argparse
import functools
import http.server
import json
import mimetypes
import os
import socketserver
from pathlib import Path


DEFAULT_HOST = os.environ.get("RETAIN_PDF_FRONTEND_BIND_HOST", "0.0.0.0")
DEFAULT_PORT = int(os.environ.get("RETAIN_PDF_FRONTEND_PORT", "40001"))
DEFAULT_ROOT = Path(
    os.environ.get("RETAIN_PDF_FRONTEND_ROOT", "/home/wxyhgk/tmp/Code/frontend")
).resolve()

mimetypes.add_type("application/javascript", ".js")
mimetypes.add_type("text/css", ".css")


class ThreadingHTTPServer(socketserver.ThreadingMixIn, http.server.HTTPServer):
    daemon_threads = True
    allow_reuse_address = True


class FrontendRequestHandler(http.server.SimpleHTTPRequestHandler):
    server_version = "retain-pdf-frontend/1.0"

    def __init__(self, *args, directory: str | None = None, **kwargs):
        super().__init__(*args, directory=directory, **kwargs)

    def do_GET(self) -> None:
        if self.path == "/health":
            payload = {
                "ok": True,
                "service": "retain-pdf-frontend",
                "root": str(Path(self.directory).resolve()),
            }
            encoded = json.dumps(payload).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Content-Length", str(len(encoded)))
            self.end_headers()
            self.wfile.write(encoded)
            return
        super().do_GET()

    def end_headers(self) -> None:
        # Frontend code and runtime config should reflect local deployment immediately.
        no_store_suffixes = (
            ".html",
            ".js",
            ".css",
        )
        if self.path.endswith(no_store_suffixes):
            self.send_header("Cache-Control", "no-store")
        else:
            self.send_header("Cache-Control", "public, max-age=60")
        super().end_headers()

    def log_message(self, format: str, *args) -> None:
        super().log_message(format, *args)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Serve the Retain PDF frontend as static files.")
    parser.add_argument("--host", default=DEFAULT_HOST)
    parser.add_argument("--port", type=int, default=DEFAULT_PORT)
    parser.add_argument("--root", default=str(DEFAULT_ROOT))
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    root = Path(args.root).resolve()
    if not root.exists():
        raise SystemExit(f"frontend root does not exist: {root}")
    handler = functools.partial(FrontendRequestHandler, directory=str(root))
    with ThreadingHTTPServer((args.host, args.port), handler) as httpd:
        print(
            f"retain-pdf-frontend serving {root} on http://{args.host}:{args.port}",
            flush=True,
        )
        httpd.serve_forever()


if __name__ == "__main__":
    main()

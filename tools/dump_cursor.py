#!/usr/bin/env python3
"""Connect to the cursor channel (video port + 1) and print what arrives.

usage: dump_cursor.py [host] [port] [seconds]
"""
import socket, struct, sys, time

host = sys.argv[1] if len(sys.argv) > 1 else "127.0.0.1"
port = int(sys.argv[2]) if len(sys.argv) > 2 else 7879
secs = float(sys.argv[3]) if len(sys.argv) > 3 else 3

s = socket.create_connection((host, port))

def read(n):
    b = b""
    while len(b) < n:
        c = s.recv(n - len(b))
        if not c:
            raise EOFError
        b += c
    return b

t0 = time.time()
positions = 0
shapes = 0
last = None
while time.time() - t0 < secs:
    (n,) = struct.unpack(">I", read(4))
    body = read(n)
    kind = body[0]
    if kind == 1:
        x, y, inside = struct.unpack(">ffB", body[1:10])
        positions += 1
        last = (x, y, inside)
    elif kind == 2:
        w, h, hx, hy = struct.unpack(">ffff", body[1:17])
        shapes += 1
        print(f"shape: {w:.4f}x{h:.4f} of display, hot ({hx:.4f},{hy:.4f}), png {len(body)-17} bytes, magic {body[17:21]!r}")
print(f"{positions} position msgs ({positions/secs:.0f}/s), {shapes} shape msgs, last={last}")

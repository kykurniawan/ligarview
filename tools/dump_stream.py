#!/usr/bin/env python3
"""Connect to the Ligarview host, validate the protocol and dump the H.264 stream.

usage: dump_stream.py [host] [port] [seconds] [out.h264]
"""
import socket, struct, sys, time

host = sys.argv[1] if len(sys.argv) > 1 else "127.0.0.1"
port = int(sys.argv[2]) if len(sys.argv) > 2 else 7878
secs = float(sys.argv[3]) if len(sys.argv) > 3 else 5
out = open(sys.argv[4], "wb") if len(sys.argv) > 4 else None

s = socket.create_connection((host, port))
s.sendall(b"LGRP\x00\x00\x00\x00")  # pairing preamble; loopback needs no code

def read(n):
    b = b""
    while len(b) < n:
        c = s.recv(n - len(b))
        if not c:
            raise EOFError
        b += c
    return b

magic, ver, codec, w, h, fps = struct.unpack(">4sBBHHH", read(12))
assert magic == b"LGRV", magic
print(f"hello: v{ver} codec={codec} {w}x{h} @{fps}")
t0 = time.time()
frames = keys = total = 0
first_key = None
while time.time() - t0 < secs:
    n, flags, pts = struct.unpack(">IBQ", read(13))
    data = read(n)
    if frames == 0:
        first_key = bool(flags & 1)
        print(f"first frame after {time.time()-t0:.2f}s: keyframe={first_key}, {n} bytes")
    frames += 1
    keys += flags & 1
    total += n
    if out:
        out.write(data)
print(f"{frames} frames, {keys} keyframes, {total/1024:.0f} KB, {total*8/secs/1e6:.2f} Mbps")

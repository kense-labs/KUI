"""Remove the optional WebAssembly name section from a release binary."""
from pathlib import Path
import sys

path = Path(sys.argv[1])
data = path.read_bytes()
if data[:8] != b"\0asm\1\0\0\0":
    raise ValueError("not a WebAssembly module")

def leb(position):
    result = 0
    for shift in range(0, 35, 7):
        if position >= len(data):
            raise ValueError("truncated WebAssembly section")
        byte = data[position]
        position += 1
        result |= (byte & 127) << shift
        if not byte & 128:
            return result, position
    raise ValueError("invalid WebAssembly section length")

out = bytearray(data[:8])
position = 8
while position < len(data):
    start = position
    section = data[position]
    length, payload = leb(position + 1)
    end = payload + length
    if end > len(data):
        raise ValueError("truncated WebAssembly payload")
    drop = False
    if section == 0:
        name_length, name_start = leb(payload)
        if name_start + name_length > end:
            raise ValueError("invalid WebAssembly custom section")
        drop = data[name_start:name_start + name_length] == b"name"
    if not drop:
        out.extend(data[start:end])
    position = end
path.write_bytes(out)
print(f"Stripped optional WASM names: {len(data)} → {len(out)} bytes")

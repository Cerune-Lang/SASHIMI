from pathlib import Path
import hashlib, json
root = Path(__file__).resolve().parents[1]
files = json.loads((root / "SOURCE_SNAPSHOT.json").read_text())["sha1_git_blob"]
for path, expected in files.items():
    relative = Path(path).relative_to("src/codegen/x86_64")
    data = (root / "src" / relative).read_bytes()
    actual = hashlib.sha1(f"blob {len(data)}\0".encode() + data).hexdigest()
    assert actual == expected, f"{relative}: {actual} != {expected}"
    print("PASS", relative, actual)

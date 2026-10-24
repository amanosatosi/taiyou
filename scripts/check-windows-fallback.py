"""Actions-only supplemental playback witnesses; no frozen oracle writes."""
import hashlib
import json
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent.parent
out = Path(os.environ["RUNNER_TEMP"]) / "windows-fallback"
out.mkdir(exist_ok=True)
font = Path(os.environ["MUA_OFFICE_FONT"])
fixture = root / "experiments/xy-oracle/fixtures/mua-office-myanmar.ass"
binary = root / "target/debug/taiyou.exe"
reports = []

def render(name, source, mode=None, extra_fonts=()):
    png, raw = out / (name + ".png"), out / (name + ".rgba")
    command = [str(binary), "render", "--input", str(source), "--font", str(font),
               "--time-us", "1000000", "--width", "1920", "--height", "1080",
               "--profile", "core-v1", "--output", str(png), "--raw", str(raw)]
    if mode: command += ["--font-source", mode]
    for other in extra_fonts: command += ["--font", str(other)]
    run = subprocess.run(command, capture_output=True, text=True, encoding="utf-8")
    (out / (name + ".log")).write_text(run.stderr, encoding="utf-8")
    run.check_returncode()
    data = raw.read_bytes()
    assert len(data) == 1920 * 1080 * 4
    reports.append({"name": name, "sha256": hashlib.sha256(data).hexdigest(), "log": run.stderr})
    raw.unlink()  # keep small PNG/log evidence; comparison workflow retains raw frames
    return data, run.stderr

explicit, old_log = render("explicit", fixture, "explicit")
system, new_log = render("system", fixture, "system")
default, _ = render("default", fixture)
repeat, _ = render("repeat", fixture, "system")
assert "MissingGlyph" in old_log, "explicit mode must retain the absent-bracket diagnostic"
assert "MissingGlyph" not in new_log, "system mode must resolve the whole missing clusters"
assert explicit != system
assert system == default == repeat
# Covered Myanmar must remain byte-identical when host discovery is enabled.
myanmar = out / "myanmar-only.ass"
myanmar.write_text(fixture.read_text(encoding="utf-8").replace("〈စမ်းမယ်〉", "စမ်းမယ်"), encoding="utf-8")
left, _ = render("myanmar-explicit", myanmar, "explicit")
right, _ = render("myanmar-system", myanmar, "system")
assert left == right
(out / "report.json").write_text(json.dumps(reports, indent=2) + "\n", encoding="utf-8")
print("WINDOWS_FALLBACK_WITNESSES_PASS=" + json.dumps([r["name"] for r in reports]))

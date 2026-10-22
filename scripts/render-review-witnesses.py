"""CI-only candidate renders of small, Taiyou-owned visual reductions. No oracle."""
import argparse
from pathlib import Path
import shutil
import sys
import tempfile

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "validation"))
from compare import execute
import images

# name, folder, milliseconds, explanation
CASES = [
    ("kt", "karaoke", [250, 750, 1250], r"Top: \kt50 overrides the second sweep start; bottom: sequential \kf control."),
    ("run-splits", "karaoke", [0, 500, 1000], r"Top: italic run split inside one \k word; bottom: unsplit control."),
    ("collision", "positioning", [500], "Two simultaneous implicitly positioned events, no borders or animation."),
    ("layoutres", "scaling", [500], "PlayRes 320x160, LayoutRes 640x160, output 640x320, one word."),
    ("scaled-border", "scaling", [500], "PlayRes 320x160, output 640x320; SBAS yes/no paired at runtime."),
    ("alpha", "alpha", [2040, 2560, 4960], "Left: animated 0x1FF alpha target; right: 0xFF control. No geometric clip."),
    ("faux-italic", "fonts", [0], "Top: synthetic italics; bottom: upright control. Separate DejaVu TrueType and FansubBlock CFF variants."),
    ("drawing", "drawings", [0], "Single unbordered closed rectangle, no font geometry or transforms."),
    ("trimming", "parsing", [0], "Example vs ***Example with trailing style-name spaces; one explicit Example event."),
]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--review", type=Path, required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    args = parser.parse_args()
    font = args.corpus / "regression/v4++/DejaVuSans.ttf"
    binary = ROOT / "target/debug/taiyou-compare"
    notes = ["Taiyou-owned reduced witnesses: candidate-only, NO authenticated xy output,",
             "NO expected images and NO pass/mismatch assertions. Excluded from corpus counts.",
             "Sources are copied beside their images. Timings are event-local milliseconds.", ""]
    failures = []
    for name, folder, times, explanation in CASES:
        directory = args.review / folder
        directory.mkdir(exist_ok=True)
        original = ROOT / "tests/taiyou/visual-review" / (name + ".ass")
        variants = [(name, original.read_text(encoding="utf-8"))]
        if name == "scaled-border":
            variants = [(name + "-yes", variants[0][1]),
                        (name + "-no", variants[0][1].replace("ScaledBorderAndShadow: yes", "ScaledBorderAndShadow: no"))]
        elif name == "faux-italic":
            variants.append((name + "-cff", variants[0][1].replace("DejaVu Sans", "FansubBlock-CFF").replace("ABCD", "A")))
        notes.extend([folder + "/witness-" + name + "-*: " + explanation])
        for variant, source in variants:
            render_font = (args.corpus / "regression/faux-italics/FansubBlock-CFF.otf"
                           if variant == "faux-italic-cff" else font)
            script = directory / ("witness-" + variant + ".ass")
            script.write_text(source, encoding="utf-8")
            for time in times:
                output = directory / (f"witness-{variant}-{time:04}-taiyou.png")
                with tempfile.TemporaryDirectory(prefix="taiyou-witness-") as temporary:
                    temporary = Path(temporary)
                    png, raw = temporary / "actual.png", temporary / "actual.rgba"
                    code, diagnostic = execute([binary, "render", "--input", script.resolve(),
                        "--time-us", str(time * 1000), "--width", "640", "--height", "320",
                        "--profile", "core-v1", "--font", render_font, "--output", png, "--raw", raw], temporary)
                    notes.append(f"  {output.name}: strict exit {code}")
                    if code == 0:
                        images.raw_rgba(raw, 640, 320)
                        if images.read_png(png)[:2] != (640, 320):
                            raise ValueError("witness PNG dimensions")
                        shutil.copyfile(png, output)
                    else:
                        failures.append(output.name + ": " + diagnostic)
    notes.extend(["", "Witness execution failures: " + str(len(failures)), *failures])
    (args.review / "WITNESSES.txt").write_text("\n".join(notes) + "\n", encoding="utf-8")
    print("\n".join(notes))
    return int(bool(failures))


if __name__ == "__main__":
    sys.exit(main())

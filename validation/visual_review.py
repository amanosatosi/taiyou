"""Display-only, shallow PNG export. Never decides comparison outcomes."""
import argparse
from collections import Counter
import json
from pathlib import Path
import shutil

import images
import oversampling


# Group by the behavior exercised, not the upstream directory spelling.
# Values: folder, readable filename prefix, short family explanation.
FAMILIES = {
    "blurs/be": ("blur", "be", r"Edge blur (\be) strengths and repeated filtering."),
    "blurs/blur+t": ("blur", "blur-t", r"Gaussian blur animated by \t."),
    "blurs-anamorphic/blur": ("blur", "anamorphic", "Anamorphic blur with upstream 8x1 sampling."),
    "karaoke/216-vertical": ("karaoke", "vertical", "Karaoke with historical vertical @font behavior."),
    "karaoke/357-k-and-kf-desynced": ("karaoke", "k-kf-desynced", r"Interaction of \k and \kf timing."),
    "karaoke/karaoke-and-runsplits": ("karaoke", "run-splits", "Karaoke word/effect boundaries across style and run splits."),
    "v4++/kt": ("karaoke", "kt", r"\kt placement, values, repeated tags and interaction with \k/\kf."),
    "implicit-pos/shift-dir": ("positioning", "shift-dir", "Implicit alignment and collision movement, including invisible text."),
    "implicit-pos/bordercollision": ("positioning", "bordercollision", "Border geometry in automatic collision placement."),
    "ssa/effect-delay": ("positioning", "ssa-effect-delay", "Legacy SSA timed event effects and movement."),
    "scaling_playres/playresx_sbas-no": ("scaling", "playres-sbas-no", "LayoutRes/PlayRes with unscaled borders/shadows."),
    "scaling_playres/playresx_sbas-yes": ("scaling", "playres-sbas-yes", "LayoutRes/PlayRes with scaled borders/shadows, spacing and drawings."),
    "faux-italics/cff": ("fonts", "faux-italic-cff", "Synthetic italic outlines in CFF fonts."),
    "faux-italics/truetype": ("fonts", "faux-italic-truetype", "Synthetic italic outlines in TrueType fonts."),
    "embedded-font/efont": ("fonts", "embedded-font", "ASS embedded font decoding and face selection."),
    "draw-images/drawImg": ("drawings", "drawimg", "ASS vector drawing commands and filled paths."),
    "zero-area/zero-area": ("positioning", "zero-area", "Collision placement and event order with a zero-advance combining mark."),
    "token-parsing/trimming": ("parsing", "trimming", "Style-name token trimming/lookup, including *** and trailing whitespace."),
    "vsf-quirks-var/alpha-clipping": ("alpha", "alpha-clipping", "Out-of-range alpha targets during interpolation (not geometric clipping)."),
    "ssa/alpha": ("alpha", "ssa-alpha", "Legacy SSA style alpha mapping."),
}
for _name in ("ffmpeg", "ffmpeg_legacy2", "ffmpeg_legacy2+sbas", "ffmpeg_legacy3", "ffmpeg_legacy4", "not-ffmpeg"):
    FAMILIES["scaled-border-default/" + _name] = (
        "scaling", "scaled-border-" + _name.replace("_", "-").replace("+", "-"),
        "ScaledBorderAndShadow defaults and legacy script metadata (" + _name + ").")
for _name in ("big5-hkscs", "legacy-arabic-simplified", "legacy-arabic-traditional", "shiftjis", "symbol-first-char-0x0000", "symbol-first-char-0xF000"):
    FAMILIES["font_nonunicode/" + _name] = ("fonts", _name.lower(), "Legacy font/charmap selection: " + _name + ".")

PROVENANCE = """Reference PNGs are byte copies from the pinned libass-tests corpus.
Taiyou PNGs are produced by this PR candidate. No authenticated xy-VSFilter
captures are available. A libass pixel mismatch does not establish xy incompatibility.
Diff PNGs use the existing absolute-difference display: premultiplied encoded
RGBA16 deltas / 257, opaque alpha, alpha-only differences included in red.
Unit-scale Taiyou PNGs retain the renderer output; oversampled PNGs are the
existing display-only RGBA8 preview of normalized RGBA16 comparison pixels.
*-diagnostic-unsupported-* triplets are tolerant output, NEVER passes.
witness-* images are Taiyou-owned reductions, candidate-only, with NO oracle/diff.
"""


def family(case):
    return FAMILIES[case["source"].removeprefix("regression/").removesuffix(".ass")]


def stem(case):
    # Preserve upstream zero padding, including kt's five-digit millisecond names.
    time = Path(case["frames"][0]["expected"]).stem.rsplit("-", 1)[1]
    return family(case)[1] + "-" + time


def result_counts(records):
    counts = Counter(r.get("observed_result", "INFRA_ERROR") for r in records)
    return ", ".join(f"{counts[k]} {label}" for k, label in (
        ("PASS_EXACT", "exact"), ("PIXEL_MISMATCH", "mismatch"),
        ("BLOCKED_BY_UNSUPPORTED_FEATURE", "unsupported"), ("EXECUTION_FAILED", "execution failure"),
        ("INFRA_ERROR", "infrastructure error"), ("CRASH", "crash")))


class Review:
    def __init__(self, root):
        self.root = Path(root)
        self.root.mkdir(parents=True, exist_ok=False)
        self.frames = {}
        self.errors = []

    def save(self, case, reference, expected, actual, png, diagnostic=False):
        """Export existing render bytes; errors are separate from strict results."""
        try:
            folder, _, _ = family(case)
            directory = self.root / folder
            directory.mkdir(exist_ok=True)
            name = stem(case) + ("-diagnostic-unsupported" if diagnostic else "")
            if case["id"] in self.frames:
                raise ValueError("duplicate visual review case")
            paths = [directory / (name + suffix) for suffix in
                     ("-1-reference.png", "-2-taiyou.png", "-3-diff.png")]
            if any(path.exists() for path in paths):
                raise ValueError("visual review filename collision")
            shutil.copyfile(reference, paths[0])
            if oversampling.factors(case["scale"]) == (1, 1):
                shutil.copyfile(png, paths[1])
            else:
                oversampling.preview(paths[1], actual)
            images.compare(expected, actual, paths[2])
            self.frames[case["id"]] = {"diagnostic": diagnostic,
                "images": [str(path.relative_to(self.root)).replace("\\", "/") for path in paths]}
        except (OSError, ValueError, KeyError) as error:
            self.errors.append(case["id"] + ": " + str(error))

    def finish(self, report, selected):
        by_id = {r["case"]: r for r in report["records"]}
        lines = ["Visual compatibility review", "candidate: " + report["candidate_sha"],
                 "base: " + report["accepted_baseline"], "libass-tests: " + report["corpus_revision"],
                 "suite: " + report["suite"] + " (no carried results)", "", PROVENANCE]
        manifest = []
        folders = sorted({family(c)[0] for c in selected if c["kind"] == "regression"})
        for folder in folders:
            cases = [c for c in selected if c["kind"] == "regression" and family(c)[0] == folder]
            records = [by_id[c["id"]] for c in cases]
            frames = [self.frames[c["id"]] for c in cases if c["id"] in self.frames]
            diagnostic = sum(f["diagnostic"] for f in frames)
            lines.extend([folder, f"  {len(frames)} visual frames with triplets ({len(frames) - diagnostic} strict, {diagnostic} diagnostic/unsupported)",
                f"  {len(cases)} selected corpus frames",
                "  results: " + result_counts(records),
                "  tests: " + ", ".join(sorted({family(c)[1] for c in cases})), ""])
            for name in sorted({family(c)[1] for c in cases}):
                lines.append("    " + name + ": " + result_counts(
                    [by_id[c["id"]] for c in cases if family(c)[1] == name]))
            lines.append("")
            directory = self.root / folder
            directory.mkdir(exist_ok=True)
            explanations = sorted({family(c)[1] + "-*: " + family(c)[2] for c in cases})
            (directory / "README.txt").write_text("\n\n".join(explanations) + "\n\n" + PROVENANCE, encoding="utf-8")
            for case, record in zip(cases, records):
                saved = self.frames.get(case["id"])
                if record.get("exit_code") == 0 and record.get("metrics") and not saved:
                    self.errors.append(case["id"] + ": successful strict render missing review triplet")
                manifest.append({"case": case["id"], "folder": folder, "status": record["status"],
                    "observed_result": record.get("observed_result"), "exit_code": record.get("exit_code"),
                    "tolerant_exit_code": record.get("tolerant_exit_code"),
                    "diagnostics": record.get("diagnostics"), "metrics": record.get("metrics"),
                    "reference_sha256": record.get("expected", {}).get("sha256"),
                    "raw_sha256": record.get("raw_sha256"), "png_sha256": record.get("png_sha256"),
                    "review": saved})
        lines.extend(["Comparison results (authoritative; suite identified above):", json.dumps(report["summary"].get("visual", {}), sort_keys=True),
                      "Parser-only probes (no images): " + json.dumps(report["summary"].get("safety", {}), sort_keys=True),
                      "Upstream-disabled sources: " + json.dumps(report["summary"].get("disabled", [])), "",
                      "Per-case results and missing-image reasons: RESULTS.txt / manifest.json.",
                      "Reduced witnesses: WITNESSES.txt (separate from corpus counts).",
                      "Review export errors: " + str(len(self.errors)), *self.errors])
        (self.root / "SUMMARY.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
        (self.root / "RESULTS.txt").write_text("\n".join(
            f'{r["folder"]}/{r["case"]}: {r["observed_result"]}; '
            + ("diagnostic/unsupported triplet" if r["review"] and r["review"]["diagnostic"] else "strict triplet" if r["review"] else "NO IMAGES: " + str(r["diagnostics"]))
            for r in manifest) + "\n", encoding="utf-8")
        (self.root / "manifest.json").write_text(json.dumps({"candidate_sha": report["candidate_sha"],
            "frames": manifest, "errors": self.errors}, indent=2) + "\n", encoding="utf-8")
        (self.root / "comparison-report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")


def verify(root):
    root = Path(root)
    manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
    if manifest["errors"]:
        raise ValueError("review export failed: " + str(manifest["errors"]))
    paths = []
    for record in manifest["frames"]:
        saved = record["review"]
        if record["exit_code"] == 0 and record["metrics"] and not saved:
            raise ValueError("strict frame missing triplet")
        if not saved:
            continue
        if saved["diagnostic"] and (record["exit_code"] != 20 or record["tolerant_exit_code"] != 0):
            raise ValueError("diagnostic output lacks strict unsupported evidence")
        for name in saved["images"]:
            if len(Path(name).parts) != 2 or not (root / name).is_file():
                raise ValueError("missing/non-shallow image: " + name)
            paths.append(name)
        if saved["images"] != sorted(saved["images"]):
            raise ValueError("triplet sort order")
    if len(paths) != len(set(paths)):
        raise ValueError("duplicate review image")
    print("REVIEW_VERIFIED: " + str(len(paths) // 3) + " corpus triplets; " + manifest["candidate_sha"])
    print((root / "SUMMARY.txt").read_text(encoding="utf-8"))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--verify", type=Path, required=True)
    verify(parser.parse_args().verify)

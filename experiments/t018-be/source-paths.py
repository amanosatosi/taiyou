"""CI diagnostic: authenticate saved GDI observations; compare current native paths.

No font/reference assets are modified. This is not an authenticated xy capture.
"""
import hashlib
import io
import json
import os
from pathlib import Path
import urllib.error
import urllib.request
import zipfile

ROOT = Path(os.environ['TAIYOU_BE_AUDIT_OUTPUT'])
API = 'https://api.github.com/repos/amanosatosi/taiyou/actions/artifacts/12279509017'
HEAD = 'ca6ea9286c46f5559c70ddeba43becdde02fa530'
DIGEST = '02b2e27553beca2cb9d8e8f1963b25f3d6b234e04bfddb3e65d6bb55ea5fb5ed'

def request(url):
    return urllib.request.Request(url, headers={'Authorization': 'Bearer ' + os.environ['GH_TOKEN'], 'Accept': 'application/vnd.github+json'})

metadata = json.load(urllib.request.urlopen(request(API), timeout=60))
assert not metadata['expired'] and metadata['workflow_run']['head_sha'] == HEAD
assert metadata['size_in_bytes'] == 5635 and metadata['digest'] == 'sha256:' + DIGEST

class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None

try:
    urllib.request.build_opener(NoRedirect).open(request(API + '/zip'), timeout=60)
    raise RuntimeError('artifact endpoint did not redirect')
except urllib.error.HTTPError as response:
    if response.code != 302:
        raise
    location = response.headers['Location']
    assert location.startswith('https://')
# Deliberately omit Authorization on the external artifact-storage request.
with urllib.request.urlopen(location, timeout=60) as response:
    data = response.read(5636)
assert len(data) == 5635 and hashlib.sha256(data).hexdigest() == DIGEST
with zipfile.ZipFile(io.BytesIO(data)) as archive:
    gdi = json.loads(archive.read('t018-gdi-cell-probe.json'))
assert gdi['candidate'] == HEAD
font = next(f for f in gdi['fonts'] if f['observations'][0]['family'] == 'Aileron')
assert font['font_sha256'] == '2762f4fc2ebad8323264aea52ffa2260b86c9677493d3ce2dc4f34e5851d2aa2'
native = [json.loads(row) for row in (ROOT / 'font-outlines-q6.jsonl').read_text().splitlines()]
report = {'gdi_run': 39454311798, 'gdi_artifact': 12279509017, 'gdi_sha256': DIGEST,
          'gdi_candidate': HEAD, 'candidate': os.environ['GITHUB_SHA'],
          'environment': {k: gdi[k] for k in ['image', 'os', 'architecture']},
          'font_sha256': font['font_sha256'], 'glyphs': []}
paths = []
for group, text, label in [(0, '.', 'period'), (4, '+', 'plus'), (8, '¶', 'paragraph')]:
    observation = next(o for o in font['observations'] if o['height'] == 6400 and o['text'] == text)
    expected = [[x, y, kind & ~1] for x, y, kind in observation['path']]
    actual = [[x, y + observation['metrics']['ascent'], kind] for x, y, kind in native[group]['points']]
    if actual[-1][2] == 2 and actual[-1][:2] == actual[0][:2]:
        actual.pop()  # FreeType emits an explicit closing line; GDI flags closure.
    assert len(actual) == len(expected)
    assert [p[2] for p in actual] == [p[2] for p in expected]
    deltas = [a - b for p, q in zip(actual, expected) for a, b in zip(p[:2], q[:2])]
    report['glyphs'].append({'text': text, 'points': len(actual),
                            'changed_coordinates': sum(d != 0 for d in deltas),
                            'max_native_q6_delta': max(map(abs, deltas)), 'deltas': deltas})
    # Exact source/reference points enter BOTH raster backends. No attempt to
    # compensate for a GDI outline difference by changing a filter coefficient.
    for name, points in [('gdi', expected), ('freetype', actual)]:
        paths.append(label + '-' + name + '\t' + ';'.join(f'{k},{x},{y}' for x, y, k in points))
(ROOT / 'supplied-paths.tsv').write_text('\n'.join(paths) + '\n')
(ROOT / 'outline-differences.json').write_text(json.dumps(report, indent=2) + '\n')
print('OUTLINE_DIFFERENCES=' + json.dumps(report))

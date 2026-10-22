"""Runner-only acquisition of the existing frozen vertical witness font."""
import hashlib
import json
import os
from pathlib import Path
import urllib.request
root = Path(__file__).resolve().parent.parent
inventory = json.loads((root / 'validation/inventory.json').read_text())
record = next(a for a in inventory['assets'] if a['path'] == 'regression/.fonts/IPAexGothic.ttf')
url = 'https://raw.githubusercontent.com/libass/libass-tests/a5073319af8117271d8ac13ef5c088197ff2d59b/' + record['path']
with urllib.request.urlopen(url, timeout=60) as response:
    data = response.read(record['bytes'] + 1)
if len(data) != record['bytes'] or hashlib.sha256(data).hexdigest() != record['sha256']:
    raise RuntimeError('frozen vertical font identity differs')
path = Path(os.environ['RUNNER_TEMP']) / 'taiyou-shaping-fonts/IPAexGothic.ttf'
path.write_bytes(data)
print('VERTICAL_FONT=' + json.dumps(record))

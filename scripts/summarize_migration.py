"""Assemble measured results and validation evidence without copying input data."""
import datetime
import json
import subprocess
from pathlib import Path

def read(path):return json.loads(Path(path).read_text())
def benchmark(name):return read(f'benchmarks/{name}.json')
electron=benchmark('electron-conversion');rust=benchmark('rust-conversion')
assert electron['dataset']['sha256']==rust['dataset']['sha256'],'Different input datasets'
assert all(s['exit_code']==0 for report in [electron,rust] for s in report['samples'])
parity=read('output/full-parity.json')
assert parity['semantic_pass'] and parity['raster_pass'],'Output parity check failed'
baseline=benchmark('electron-initial')
def metrics(implementation,conversion,application_bytes):
    return {'conversion_ms':conversion['median_elapsed_ms'],'conversion_rss_bytes':conversion['median_peak_rss_bytes'],
        'application_bytes':application_bytes,
        'build_clean_ms':benchmark(implementation+'-build-clean')['median_elapsed_ms'],
        'build_warm_ms':benchmark(implementation+'-build-warm')['median_elapsed_ms']}
def docx_bytes(path):return sum(p.stat().st_size for p in Path(path).glob('*.docx'))
e=metrics('electron',electron,baseline['artifacts']['application_bytes'])
r=metrics('rust',rust,benchmark('rust-package')['artifact_bytes'])
e['docx_bytes']=docx_bytes('output/electron-reference');r['docx_bytes']=docx_bytes('output/rust-reference')
report={'schema_version':1,'timestamp':datetime.datetime.now().astimezone().isoformat(),
    'electron_source_commit':electron['source_commit'],'rust_source_commit':rust['source_commit'],
    'dataset':electron['dataset'],'environment':baseline['environment'],
    'conversion_runtime':{'electron':'Electron 37.10.3 / embedded Node v22.21.1, ELECTRON_RUN_AS_NODE=1','rust':'Native release executable, CLI mode'},
    'electron':e,'rust':r,
    'changes_percent':{key:(r[key]/e[key]-1)*100 for key in e},
    'validation':{'semantic_pass':parity['semantic_pass'],'raster_pass':parity['raster_pass'],'docx_count':parity['docx_count'],
        'raster_images_checked':parity['raster']['checked'],'raster_mean_absolute_channel_error':parity['raster']['mean_absolute_channel_error'],
        'raster_worst_absolute_channel_error':max(item['mean_absolute_channel_error'] for item in parity['raster']['worst']),
        'raster_tolerance_per_image_mae':parity['raster']['max_allowed_per_image_mae'],'markers':85603,'mismatches':10383},
    'ci':read('output/benchmarks/final-ci.json'),
    'notes':['Three conversion runs per implementation, same prepared CSV files and concurrency=2.',
        'Conversion RSS measures the engine process in CLI mode; GUI windows are excluded for both implementations.',
        'Clean Rust release build compiles third-party Rust dependencies; Electron uses prebuilt Electron and sharp distributions. Dependency installation is excluded.',
        'Warm build means a repeated command without source changes.',
        'Raster pixels are not identical: different renderers and font rasterizers. Every image must remain within the configured per-image tolerance; worst frames were visually inspected.',
        'Rust macOS packaging creates app+ZIP; original Electron packaging created app+ZIP+DMG.']}
Path('benchmarks/comparison.json').write_text(json.dumps(report,ensure_ascii=False,indent=2))
print(json.dumps({'electron':e,'rust':r,'changes_percent':report['changes_percent'],'validation':report['validation']},ensure_ascii=False,indent=2))

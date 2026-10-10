"""Assemble measured results and validation evidence without copying input data."""
import argparse
import datetime
import json
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--rust-output', type=Path, default=Path('output/rust-reference'))
parser.add_argument('--optimization-parity', type=Path)
args = parser.parse_args()

def read(path):return json.loads(Path(path).read_text())
def benchmark(name):return read(f'benchmarks/{name}.json')
electron=benchmark('electron-conversion');rust=benchmark('rust-conversion')
assert electron['dataset']['sha256']==rust['dataset']['sha256'],'Different input datasets'
assert all(s['exit_code']==0 for report in [electron,rust] for s in report['samples'])
parity=read('output/full-parity.json')
assert parity['semantic_pass'] and parity['raster_pass'],'Output parity check failed'
ci = read('output/benchmarks/final-ci.json')
assert ci['conclusion'] == 'success', 'Cross-platform CI must succeed before publishing the comparison'
assert ci['headSha'] == rust['source_commit'], 'CI did not verify the benchmarked Rust source'
baseline=benchmark('electron-initial')
def metrics(implementation,conversion,application_bytes):
    return {'conversion_ms':conversion['median_elapsed_ms'],'conversion_rss_bytes':conversion['median_peak_rss_bytes'],
        'application_bytes':application_bytes,
        'build_clean_ms':benchmark(implementation+'-build-clean')['median_elapsed_ms'],
        'build_warm_ms':benchmark(implementation+'-build-warm')['median_elapsed_ms']}
def docx_bytes(path):return sum(p.stat().st_size for p in Path(path).glob('*.docx'))
e=metrics('electron',electron,baseline['artifacts']['application_bytes'])
r=metrics('rust',rust,benchmark('rust-package')['artifact_bytes'])
e['docx_bytes']=docx_bytes('output/electron-reference');r['docx_bytes']=docx_bytes(args.rust_output)
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
    'ci':ci,
    'notes':['Three conversion runs per implementation, same prepared CSV files and concurrency=2.',
        'Conversion RSS measures the engine process in CLI mode; GUI windows are excluded for both implementations.',
        'Clean Rust release build compiles third-party Rust dependencies; Electron uses prebuilt Electron and sharp distributions. Dependency installation is excluded.',
        'Warm build means a repeated command without source changes.',
        'Raster pixels are not identical: different renderers and font rasterizers. Every image must remain within the configured per-image tolerance; worst frames were visually inspected.',
        'Rust macOS packaging creates app+ZIP; original Electron packaging created app+ZIP+DMG.']}
if args.optimization_parity:
    optimization = read(args.optimization_parity)
    assert optimization['semantic_pass'] and optimization['raster_pass']
    assert optimization['raster']['max_allowed_per_image_mae'] == 0
    assert optimization['raster']['checked'] == optimization['docx_count']
    assert r['conversion_ms'] < e['conversion_ms'], 'Optimization must beat Electron on the same dataset'
    image_bytes = read('output/optimization-image-bytes.json')
    assert image_bytes['png_bytes_identical'] and image_bytes['checked'] == optimization['docx_count']
    before = read('benchmarks/history/pre-optimization-rust-conversion.json')
    report['optimization'] = {
        'previous_conversion_ms': before['median_elapsed_ms'],
        'speedup_over_electron': e['conversion_ms'] / r['conversion_ms'],
        'conversion_change_percent': (r['conversion_ms'] / before['median_elapsed_ms'] - 1) * 100,
        'semantic_pass': optimization['semantic_pass'],
        'pixel_identical_to_previous_rust': optimization['raster']['mean_absolute_channel_error'] == 0,
        'images_checked': optimization['raster']['checked'],
        'png_bytes_identical_to_previous_rust': image_bytes['png_bytes_identical'],
        'docx_bytes_identical_to_previous_rust': image_bytes.get('docx_bytes_identical'),
        'cause': 'File-backed fonts were reopened and mapped per glyph. Selected font faces now share resident binary data within each batch.',
    }
    report['notes'].append('Clean build timings are the original migration measurements; conversion, packaging and warm Rust build were remeasured after font caching.')
Path('benchmarks/comparison.json').write_text(json.dumps(report,ensure_ascii=False,indent=2))
print(json.dumps({'electron':e,'rust':r,'changes_percent':report['changes_percent'],'validation':report['validation']},ensure_ascii=False,indent=2))

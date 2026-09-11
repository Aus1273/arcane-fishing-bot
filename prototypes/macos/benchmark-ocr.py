#!/usr/bin/env python3
"""Reproducible local PNG evaluation; never captures the display or sends input."""
import argparse
import json
import platform
from pathlib import Path
import statistics
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('screenshots', type=Path)
parser.add_argument('--output', type=Path, default=Path(__file__).parent / 'benchmarks' / 'ocr-results.json')
args = parser.parse_args()
prototype = Path(__file__).resolve().parent
binary_dir = subprocess.check_output(['swift', 'build', '--package-path', str(prototype), '-c', 'release', '--show-bin-path'], text=True).strip()
binary = str(Path(binary_dir) / 'native-services')
expected = {'1..png': 956, '2..png': 953, '3.png': 954, '4..png': 954}
results = []
for filename, capacity in expected.items():
    for engine, preparation in [('vision', 'raw'), ('vision', 'threshold'), ('tesseract', 'threshold')]:
        command = [binary, 'ocr', str(args.screenshots / filename), '--engine', engine, '--prepare', preparation, '--iterations', '5', '--warmup', '1']
        result = json.loads(subprocess.check_output(command, text=True))
        result['expected'] = {'usable': capacity, 'capacity': capacity}
        result['correct_samples'] = sum(sample.get('usable') == capacity and sample.get('capacity') == capacity for sample in result['samples'])
        result['median_ms'] = statistics.median(sample['elapsed_ms'] for sample in result['samples'])
        results.append(result)
        print(f'{filename} {engine:9} {preparation:9} {result["correct_samples"]}/5 median={result["median_ms"]:.2f}ms text={result["samples"][0]["text"]!r}', flush=True)
summary = {}
for engine, preparation in [('vision', 'raw'), ('vision', 'threshold'), ('tesseract', 'threshold')]:
    matching = [r for r in results if r['engine'] == engine and r['preparation'] == preparation]
    times = [sample['elapsed_ms'] for result in matching for sample in result['samples']]
    summary[f'{engine}-{preparation}'] = {'correct': sum(r['correct_samples'] for r in matching), 'samples': len(times), 'median_ms': statistics.median(times), 'min_ms': min(times), 'max_ms': max(times)}
report = {'schema_version': 1, 'machine': platform.machine(), 'macos': platform.mac_ver()[0], 'hardware': subprocess.check_output(['sysctl', '-n', 'machdep.cpu.brand_string'], text=True).strip(), 'method': 'Five measured reads plus one warmup per source/engine/preparation. Source decoding, cropping and preprocessing are outside timing. The first warmup remains reported, not included in median. Each group starts a new service process.', 'summary': summary, 'results': results}
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(summary, indent=2))
print(f'Saved {args.output}')

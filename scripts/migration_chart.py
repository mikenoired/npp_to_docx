"""Create a measured comparison as SVG, then rasterize that SVG to JPEG."""
import argparse
import html
import json
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("--report",type=Path,default=Path("benchmarks/comparison.json"))
parser.add_argument("--output",type=Path,default=Path("benchmarks/migration"))
args = parser.parse_args()
data = json.loads(args.report.read_text())
svg = ['<svg xmlns="http://www.w3.org/2000/svg" width="1800" height="1200" viewBox="0 0 1800 1200">',
       '<rect width="1800" height="1200" fill="#ffffff"/>',
       '<style>text {font-family:Arial,sans-serif;fill:#18354c} .label {font-size:24px} .small {font-size:21px;fill:#526a7d}</style>']
def text(x,y,value,size=26,color=None):
    svg.append(f'<text x="{x}" y="{y}" font-size="{size}"'+(f' fill="{color}"' if color else '')+'>'+html.escape(str(value))+'</text>')
def line(y):svg.append(f'<path d="M80 {y}H1720" stroke="#d4dee5"/>')
text(80,90,'npp_to_docx · Electron → Rust + GPUI',48)
text(80,140,'1410 SVG · 85 603 маркера · Apple M1 · macOS · параллелизм 2',25)
text(80,180,'Медиана трёх прогонов; CSV подготовлены. Меньше — лучше.',22)
line(215)
metrics=[('Обработка всего набора','conversion_ms',1000,'с'),('Пиковая память обработки','conversion_rss_bytes',1_000_000,'МБ'),('Размер приложения','application_bytes',1_000_000,'МБ')]
for panel,(title,key,divisor,unit) in enumerate(metrics):
    x=80+panel*560
    text(x,285,title,27)
    electron=data['electron'][key]/divisor;rust=data['rust'][key]/divisor
    maximum=max(electron,rust)
    for i,(name,value,color) in enumerate([('Electron',electron,'#bd7440'),('Rust + GPUI',rust,'#276d94')]):
        y=350+i*130
        text(x,y,name,24)
        svg.append(f'<rect x="{x}" y="{y+15}" width="{430*value/maximum:.2f}" height="34" rx="4" fill="{color}"/>')
        text(x,y+85,f'{value:.2f} {unit}',30)
    change=(1-rust/electron)*100
    text(x,640,('Снижение '+f'{change:.1f}%' if change>=0 else 'Рост '+f'{-change:.1f}%'),29)
line(690)
text(80,745,'Сборка release без установки зависимостей',30)
text(80,800,'Измерение',24);text(880,800,'Electron',24);text(1230,800,'Rust + GPUI',24)
for y,label,key in [(855,'Чистая сборка (исходный замер)','build_clean_ms'),(915,'Повторная сборка без изменений','build_warm_ms')]:
    text(80,y,label,24)
    text(880,y,f"{data['electron'][key]/1000:.2f} с",26)
    text(1230,y,f"{data['rust'][key]/1000:.2f} с",26)
line(955)
text(80,1010,'Проверка результата: 1410 DOCX, таблицы, описания KKS и индекс поиска совпадают.',24)
text(80,1055,'Сглаживание изображений у resvg и librsvg отличается; растровая проверка сохранена отдельно.',21)
text(80,1110,'JSON, команды, хеш набора и все прогоны: benchmarks/ · Отчёт: comparison.json',21)
svg.append('</svg>')
args.output.parent.mkdir(parents=True,exist_ok=True)
svg_path=args.output.with_suffix('.svg');svg_path.write_text('\n'.join(svg))
png_path=Path('output/migration-chart.png');png_path.parent.mkdir(exist_ok=True)
subprocess.run(['cargo','run','--release','--locked','-p','npp-core','--example','render_svg','--',str(svg_path),str(png_path)],check=True)
from PIL import Image
Image.open(png_path).convert('RGB').save(args.output.with_suffix('.jpeg'),quality=95,subsampling=0)
print(svg_path,args.output.with_suffix('.jpeg'))

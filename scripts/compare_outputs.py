"""Compare DOCX contents, formatting, reports and search records against Electron.

Raster metrics are reported separately because resvg and librsvg use different
font rasterizers. A passing semantic check alone does not imply pixel equality.
"""
import argparse
import io
import json
import statistics
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET

NS = {
    "w": "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
    "wp": "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing",
}


def document(path):
    with zipfile.ZipFile(path) as archive:
        assert archive.testzip() is None, f"Invalid ZIP: {path}"
        root = ET.fromstring(archive.read("word/document.xml"))
        paragraphs = ["".join(p.itertext()) for p in root.findall(".//w:p", NS)]
        widths = [node.attrib for node in root.findall(".//w:tblGrid/w:gridCol", NS)]
        red = ["".join(r.itertext()) for r in root.findall(".//w:r", NS)
               if any(c.get("{" + NS["w"] + "}val") == "CC0000" for c in r.findall("w:rPr/w:color", NS))]
        image = next(archive.read(name) for name in archive.namelist() if name.startswith("word/media/") and name.endswith(".png"))
        return {
            "paragraphs": paragraphs,
            "table_widths": widths,
            "red_numbers": red,
            "page_size": root.find(".//w:pgSz", NS).attrib,
            "page_margins": root.find(".//w:pgMar", NS).attrib,
            "image_extents": [node.attrib for node in root.findall(".//wp:extent", NS)],
            "header_rows": len(root.findall(".//w:tblHeader", NS)),
            "bold_runs": ["".join(r.itertext()) for r in root.findall(".//w:r", NS) if r.find("w:rPr/w:b", NS) is not None],
        }, image


def records(directory):
    data = json.loads((directory / "search-index.json").read_text())
    return sorted(data["records"], key=lambda r: (r["frameName"], r["markerIndex"]))


def reports(directory):
    return sorted(json.loads((directory / "passport-report.json").read_text()), key=lambda r: r["frame"])


def compare(reference, candidate, raster=False):
    expected = {p.name for p in reference.glob("*.docx")}
    actual = {p.name for p in candidate.glob("*.docx")}
    differences = []
    if expected != actual:
        differences.append({"files_missing": sorted(expected - actual), "files_extra": sorted(actual - expected)})
    if records(reference) != records(candidate):
        left, right = records(reference), records(candidate)
        differences.append({"search_records": {"expected": len(left), "actual": len(right),
            "examples": [(a,b) for a,b in zip(left,right) if a != b][:5]}})
    if reports(reference) != reports(candidate):
        left, right = reports(reference), reports(candidate)
        differences.append({"passport_reports": [(a,b) for a,b in zip(left,right) if a != b][:10]})
    metrics = []
    if raster:
        from PIL import Image, ImageChops, ImageStat
    for name in sorted(expected & actual):
        left, lp = document(reference / name)
        right, rp = document(candidate / name)
        diff = {key: {"expected": left[key], "actual": right[key]} for key in left if left[key] != right[key]}
        if diff:
            differences.append({"file": name, "differences": diff})
        if raster:
            a = Image.open(io.BytesIO(lp)).convert("RGBA")
            b = Image.open(io.BytesIO(rp)).convert("RGBA")
            if a.size != b.size:
                differences.append({"file": name, "image_size": [a.size,b.size]})
                continue
            # Compare as displayed on white paper; ignore PNG encoding differences.
            def white(image):
                background = Image.new("RGBA", image.size, "white")
                return Image.alpha_composite(background, image).convert("RGB")
            diff = ImageChops.difference(white(a), white(b))
            stat = ImageStat.Stat(diff)
            metrics.append({"file": name, "mean_absolute_channel_error": statistics.mean(stat.mean),
                            "rms_channel_error": statistics.mean(stat.rms)})
    return {"semantic_pass": not differences, "docx_count": len(expected), "differences": differences,
            "raster": {"checked": len(metrics), "pixel_equality_required": False,
                "mean_absolute_channel_error": statistics.mean(m["mean_absolute_channel_error"] for m in metrics) if metrics else None,
                "worst": sorted(metrics,key=lambda m:m["mean_absolute_channel_error"],reverse=True)[:15]}}


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("reference", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--raster", action="store_true")
    parser.add_argument("--report", type=Path, default=Path("output/parity.json"))
    args = parser.parse_args()
    result = compare(args.reference,args.candidate,args.raster)
    args.report.parent.mkdir(parents=True,exist_ok=True)
    args.report.write_text(json.dumps(result,ensure_ascii=False,indent=2))
    print(json.dumps({key: value for key,value in result.items() if key != "differences"},indent=2))
    print(f"Differences: {len(result['differences'])}. Report: {args.report}")
    raise SystemExit(0 if result["semantic_pass"] else 1)

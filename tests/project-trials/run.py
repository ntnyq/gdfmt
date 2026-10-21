#!/usr/bin/env python3
"""Pinned source-only project trials. Downloads require --fetch; never edits projects."""
import argparse
import ast
import concurrent.futures
import difflib
import hashlib
import importlib.metadata
import json
from pathlib import Path
import re
import subprocess
import tempfile
import urllib.parse
import urllib.request

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent


def digest(data):
    return hashlib.sha256(data).hexdigest()


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf8")


def verify_environment():
    for package, version in re.findall(r"^([\w-]+)==([^\s\\]+)", (HERE / "requirements.txt").read_text(), re.M):
        installed = importlib.metadata.version(package)
        if installed != version:
            raise RuntimeError(f"{package}: expected {version}, installed {installed}")


def prepare(project, cache, fetch):
    def read(entry):
        relative = Path(entry["path"])
        if relative.is_absolute() or ".." in relative.parts:
            raise RuntimeError(f"invalid source path: {relative}")
        path = cache / "sources" / project["id"] / relative
        if not path.exists():
            if not fetch:
                raise RuntimeError(f"missing {path}; use --fetch to download pinned sources")
            repo = project["repository"].removeprefix("https://github.com/")
            url = f"https://raw.githubusercontent.com/{repo}/{project['commit']}/{urllib.parse.quote(entry['path'])}"
            with urllib.request.urlopen(url, timeout=30) as response:
                data = response.read()
            if digest(data) != entry["sha256"]:
                raise RuntimeError(f"download hash mismatch: {project['id']}/{relative}")
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        data = path.read_bytes()
        if digest(data) != entry["sha256"]:
            raise RuntimeError(f"cached source hash mismatch: {path}")
        return entry["path"], data.decode("utf8")

    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        files = dict(pool.map(read, project["files"]))
    # Unexpected files cannot silently expand the trial population.
    found = {str(p.relative_to(cache / "sources" / project["id"])) for p in (cache / "sources" / project["id"]).rglob("*.gd")}
    if found != {name for name in files if name.endswith(".gd")}:
        raise RuntimeError(f"script inventory mismatch: {project['id']}")
    return files


def normalized_tokens(source):
    # A review aid, never an equivalence oracle. Invariants are checked by Rust
    # against the original CST. Lark's TYPE_HINT can include dotted/indexed names.
    from gdtoolkit.parser import parser
    result = []
    for token in parser._parser.lex(source):  # Pinned gdtoolkit internal API.
        if token.type in {"_NL", "_INDENT", "_DEDENT"}:
            continue
        if "STRING" in token.type:
            try:
                result.append(repr(ast.literal_eval(str(token))))
            except (ValueError, SyntaxError):
                result.append(str(token))
        else:
            result.extend(re.findall(r"\w+|[^\w\s]", str(token)))
    return result


def classify(left, right):
    from gdtoolkit.parser import parser
    if left == right:
        return ["identical"]
    a, b = normalized_tokens(left), normalized_tokens(right)
    categories = {"layout"}
    changes = []
    for tag, i, j, k, end in difflib.SequenceMatcher(a=a, b=b, autojunk=False).get_opcodes():
        if tag != "equal":
            changes.append([a[i:j], b[k:end]])
    if changes:
        if all(t in {"(", ")", ",", ";"} for pair in changes for side in pair for t in side):
            categories.add("parentheses-commas-semicolons")
        elif all(t in {"(", ")", ",", ";", ":"} for pair in changes for side in pair for t in side):
            categories.add("inner-class-extends")
            categories.add("parentheses-commas-semicolons")
        else:
            raise RuntimeError(f"unexplained token differences: {changes}")
    raw_strings = lambda s: [str(t) for t in parser._parser.lex(s) if "STRING" in t.type]
    if raw_strings(left) != raw_strings(right):
        categories.add("quotes")
    comments = lambda s: [str(t) for t in parser.parse_comments(s).children]
    if comments(left) != comments(right):
        raise RuntimeError("unexplained comment text/order difference")
    return sorted(categories)


def check_rejection(binary, source, expected):
    with tempfile.TemporaryDirectory(prefix="gdfmt-project-rejection-") as directory:
        path = Path(directory)
        (path / "project.godot").write_text("config_version=5\n", encoding="utf8")
        (path / "case.gd").write_text(source, encoding="utf8")
        run = subprocess.run([str(binary), "--headless", "--path", directory, "--check-only", "--script", "case.gd"], capture_output=True, text=True, timeout=15)
        log = run.stdout + run.stderr
        if run.returncode not in {0, 1} or expected not in log or "Failed to load script" not in log:
            raise RuntimeError(f"source rejection no longer matches pinned engine: {log}")
        return {"exitCode": run.returncode, "stdout": run.stdout, "stderr": run.stderr}


def main():
    args = argparse.ArgumentParser(description=__doc__)
    args.add_argument("--fetch", action="store_true")
    args.add_argument("--cache", type=Path, default=ROOT / "target/project-trials")
    args.add_argument("--godot", type=Path, required=True)
    args.add_argument("--record-review", action="store_true", help="replace reviewed output fingerprints after inspecting generated diffs")
    options = args.parse_args()
    cache = options.cache.resolve()
    binary = options.godot.resolve()
    manifest = json.loads((HERE / "manifest.json").read_text())
    verify_environment()
    from gdtoolkit.formatter import format_code
    engines = json.loads((ROOT / "tests/compatibility/engine.json").read_text())["engines"]
    pin = next(engine for engine in engines if engine["languageVersion"] == "4.7")
    version = subprocess.check_output([str(binary), "--version"], text=True, timeout=15).strip()
    if version != pin["engineVersion"]:
        raise RuntimeError(f"Godot version mismatch: {version}")
    inputs, rejections = [], {}
    for project in manifest["projects"]:
        files = prepare(project, cache, options.fetch)
        for name, source in sorted(files.items()):
            if not name.endswith(".gd"):
                continue
            rejection = project["sourceRejections"].get(name)
            inputs.append({"project": project["id"], "path": name, "source": source, "expectedDiagnostics": rejection["diagnostics"] if rejection else []})
            if rejection:
                rejections[f"{project['id']}/{name}"] = check_rejection(binary, source, rejection["engineError"])
    write_json(cache / "inputs.json", inputs)
    subprocess.run(["cargo", "run", "--locked", "-p", "xtask", "--", "project-trials", "--input", str(cache / "inputs.json"), "--report", str(cache / "layouts.json")], cwd=ROOT, check=True)
    cases = json.loads((cache / "layouts.json").read_text())
    reviewed, comparisons = {}, []
    for case in cases:
        original = case["input"]
        key = f"{original['project']}/{original['path']}"
        other = format_code(original["source"], 100)
        output = case["layouts"][0]["output"]
        if case["sourceRejected"]:
            categories = ["source-rejected-gdtoolkit-accepted"]
        else:
            categories = classify(output, other)
            diff_path = cache / "diffs" / original["project"] / (original["path"] + ".diff")
            diff_path.parent.mkdir(parents=True, exist_ok=True)
            diff_path.write_text("".join(difflib.unified_diff(other.splitlines(keepends=True), output.splitlines(keepends=True), fromfile="gdtoolkit-4.5.0", tofile="gdfmt")), encoding="utf8")
        reviewed[key] = {"gdfmtSha256": digest(output.encode()) if output is not None else None, "gdtoolkitSha256": digest(other.encode()), "categories": categories}
        comparisons.append({"project": original["project"], "path": original["path"], "gdtoolkitOutput": other, "categories": categories})
    review = {"schemaVersion": 1, "manifestSha256": digest((HERE / "manifest.json").read_bytes()), "cases": reviewed}
    write_json(cache / "review-candidate.json", review)
    write_json(cache / "comparisons.json", comparisons)
    accepted = [case for case in cases if not case["sourceRejected"]]
    summary = {"schemaVersion": 1, "languageVersion": "4.7", "gdtoolkitVersion": manifest["gdtoolkit"], "engineVersion": version, "manifestSha256": review["manifestSha256"], "scripts": len(cases), "acceptedScripts": len(accepted), "invariantLayouts": 4 * len(accepted), "sourceRejections": rejections, "identicalToGdtoolkit": sum(item["categories"] == ["identical"] for item in reviewed.values()), "differentFromGdtoolkit": sum(item["categories"] != ["identical"] for key, item in reviewed.items() if key not in rejections), "unexplainedDifferences": 0}
    write_json(cache / "summary.json", summary)
    review_path = HERE / "reviewed.json"
    if options.record_review:
        write_json(review_path, review)
    elif not review_path.exists() or json.loads(review_path.read_text()) != review:
        raise RuntimeError(f"reviewed fingerprints changed; inspect {cache / 'review-candidate.json'} and diffs before --record-review")
    print(json.dumps({k: v for k, v in summary.items() if k != "sourceRejections"}, indent=2))


if __name__ == "__main__":
    main()

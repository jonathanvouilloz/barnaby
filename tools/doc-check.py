#!/usr/bin/env python3
# harness-commit: 1d45ba7
"""doc-check: checks a project's docs against its harness.toml contract.

Usage: doc-check.py [--fast | --check | --write] [--root DIR]
Exit codes: 0 clean, 1 findings, 2 contract missing or invalid.
"""
import argparse, json, re, subprocess, sys, tomllib
from pathlib import Path

SECTIONS = ["À quoi ça sert", "Cas d'usage", "Règles", "Écrans et points d'entrée", "Tests", "Limites connues"]
STATUS = re.compile(r"^> Statut\s*:\s*(spec|en cours|livré|retiré)\b")
LINK = re.compile(r"\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
CODE = re.compile(r"`([^`\s]+)`")
WINDOW = 12


class Invalid(Exception):
    pass


def load_contract(root):
    f = root / "harness.toml"
    if not f.is_file():
        raise Invalid("harness.toml absent — créer le contrat à la racine du projet")
    try:
        c = tomllib.loads(f.read_text())
    except tomllib.TOMLDecodeError as e:
        raise Invalid(f"harness.toml illisible ({e}) — corriger la syntaxe")
    if c.get("scale", "M") not in ("S", "M", "L"):
        raise Invalid("scale doit valoir S, M ou L — corriger harness.toml")
    docs = {"prd": "docs/project/prd.md", "handoff": "docs/project/handoff.md", "architecture": "docs/project/architecture.md", "features": "docs/features", "index": "docs/project/index.md",
            "changelog": "CHANGELOG.md", "decisions": "docs/project/decisions.md", "archive": "docs/archive",
            "code": [], "feature_template": None, **c.get("docs", {})}
    dc = {"max_feature_tokens": 3000, "allow_duplicates": [], **c.get("doc_check", {})}
    facts = c.get("facts", {})
    ok = (isinstance(dc["max_feature_tokens"], int) and isinstance(dc["allow_duplicates"], list)
          and all(isinstance(v, str) for k, v in docs.items() if k not in ("code",) and v is not None)
          and all(isinstance(v, dict) and isinstance(v.get("cmd"), str) and isinstance(v.get("doc", ""), str) for v in facts.values()))
    if not ok:
        raise Invalid("types invalides dans harness.toml — vérifier [docs], [doc_check] et [facts.*] (cmd et doc en texte)")
    return c.get("scale", "M"), docs, dc, facts


def strip_generated(lines):
    """(number, text) of lines outside code fences and generated sections."""
    fenced = generated = False
    for n, t in enumerate(lines, 1):
        if t.lstrip().startswith("```"):
            fenced = not fenced
        elif "<!-- BEGIN generated:" in t:
            generated = True
        elif "<!-- END generated:" in t:
            generated = False
        elif not fenced and not generated:
            yield n, t


def render(items):
    cols = ["name"] + [k for k in ("group", "description") if any(k in i for i in items)]
    extra = list(dict.fromkeys(k for i in items for k in i.get("columns", {})))
    row = lambda vals: "| " + " | ".join(str(v).replace("|", "\\|") for v in vals) + " |"
    head = [row(cols + extra), row(["---"] * (len(cols) + len(extra)))]
    return "\n".join(head + [row([i.get(k, "") for k in cols] + [i.get("columns", {}).get(k, "") for k in extra]) for i in items])


def run_fact(root, name, spec):
    p = subprocess.run(spec["cmd"], shell=True, cwd=root, capture_output=True, text=True, timeout=300)
    if p.returncode:
        raise ValueError(f"la commande échoue ({p.stderr.strip()[:80]})")
    items = json.loads(p.stdout)
    names = [i.get("name") for i in items] if isinstance(items, list) and all(isinstance(i, dict) for i in items) else None
    if not names or not all(isinstance(n, str) for n in names) or len(set(names)) != len(names):
        raise ValueError("sortie invalide : tableau d'éléments avec un `name` texte unique attendu")
    return items


def section_bounds(text, name):
    """(start, end) of the inside of the generated section, or an error string."""
    b, e = f"<!-- BEGIN generated:{name} -->", f"<!-- END generated:{name} -->"
    i = text.find(b)
    if i < 0:
        return "marqueur BEGIN absent"
    j = text.find(e, i)
    return (i + len(b), j) if j >= 0 else "marqueur END manquant"


def main():
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group()
    g.add_argument("--fast", action="store_true")
    g.add_argument("--check", action="store_true")
    g.add_argument("--write", action="store_true")
    ap.add_argument("--root", default=".")
    a = ap.parse_args()
    root = Path(a.root).resolve()
    try:
        scale, docs, dc, facts = load_contract(root)
    except Invalid as e:
        print(f"doc-check: {e}")
        return 2

    errs = []
    add = lambda p, n, problem, fix: errs.append(f"{p}:{n} — {problem} — {fix}")
    rel = lambda p: p.relative_to(root).as_posix()
    read = lambda p: p.read_text().splitlines()

    archive = root / docs["archive"]
    plans = root / "docs/plans"
    md = [*sorted(root.glob("*.md")), *sorted((root / "docs").rglob("*.md"))]
    md = [p for p in md if not p.name.startswith("_") and archive not in p.parents]
    live = [p for p in md if plans not in p.parents]

    # Required documents by scale
    need = ["AGENTS.md", docs["handoff"], docs["changelog"]]
    if scale != "S":
        need += [docs[k] for k in ("prd", "architecture", "decisions", "index", "features")]
    for path in need:
        if not (root / path).exists():
            add(path, 0, "document exigé absent", f"créer {path} (taille {scale})")

    # Feature sheets, index, template
    feats = root / docs["features"]
    index = root / docs["index"]
    template = SECTIONS
    if docs["feature_template"] and (root / docs["feature_template"]).is_file():
        template = re.findall(r"^## (.+?)\s*$", (root / docs["feature_template"]).read_text(), re.M)
    sheets = sorted(p for p in feats.glob("*.md") if not p.name.startswith("_")) if feats.is_dir() else []
    linked = {}  # resolved path -> (line)
    if index.is_file():
        for n, t in enumerate(read(index), 1):
            for m in LINK.finditer(t):
                target = (index.parent / m.group(1).split("#")[0]).resolve()
                if target.parent == feats.resolve():
                    linked[target] = (n, m.group(1))
    for s in sheets:
        if s.resolve() not in linked and index.is_file():
            add(rel(s), 0, "fiche absente de l'index", f"ajouter une ligne vers cette fiche dans {docs['index']}")
    for target, (n, raw) in linked.items():
        if not target.is_file():
            add(rel(index), n, f"ligne d'index sans fiche : {raw}", "créer la fiche ou retirer la ligne")
    max_chars = dc["max_feature_tokens"] * 4  # tokens = characters / 4
    for s in sheets:
        text = s.read_text()
        heads = {h.strip() for h in re.findall(r"^## (.+)$", text, re.M)}
        for h in template:
            if h not in heads:
                add(rel(s), 0, f"titre du modèle manquant : ## {h}", "ajouter la section, même courte")
        if not any(STATUS.match(l) for l in text.splitlines()):
            add(rel(s), 0, "ligne de statut manquante", "ajouter « > Statut : spec | en cours | livré | retiré · … »")
        if len(text) > max_chars:
            add(rel(s), 0, f"fiche trop longue ({len(text) // 4} jetons, max {dc['max_feature_tokens']})", "découper la fiche")

    # Decisions index (directory variant)
    dec = root / docs["decisions"]
    if dec.is_dir():
        didx = dec / "index.md"
        names = {p.name for p in dec.glob("*.md") if p.name != "index.md" and not p.name.startswith("_")}
        got = set()
        if didx.is_file():
            for n, t in enumerate(read(didx), 1):
                for m in LINK.finditer(t):
                    got.add(m.group(1).split("#")[0])
                    if not (dec / m.group(1).split("#")[0]).is_file():
                        add(rel(didx), n, f"ligne d'index sans fichier : {m.group(1)}", "créer la décision ou retirer la ligne")
        else:
            add(rel(didx), 0, "document exigé absent", "créer l'index des décisions")
        for name in sorted(names - got) if didx.is_file() else []:
            add(rel(didx), 0, f"décision absente de l'index : {name}", "ajouter une ligne dans l'index")

    # Broken links and cited code paths
    for p in md:
        for n, t in strip_generated(read(p)):
            for m in LINK.finditer(t):
                raw = m.group(1)
                if re.match(r"[a-z][a-z0-9+.-]*:", raw) or raw.startswith("#"):
                    continue
                target = (root / raw[1:] if raw.startswith("/") else p.parent / raw).resolve()
                if not target.exists() and not (p == index and target.parent == feats.resolve()):
                    add(rel(p), n, f"lien cassé vers {raw.split('#')[0]}", "corriger le chemin ou retirer le lien")
            if p in live:
                for tok in CODE.findall(t):
                    tok = re.sub(r":\d+$", "", tok.rstrip(".,;"))
                    if "/" in tok and not tok.startswith("/") and not re.search(r"[*{}<>$]", tok) \
                            and (root / tok.split("/")[0]).exists() and not (root / tok).exists():
                        add(rel(p), n, f"chemin cité introuvable : {tok}", "corriger le chemin ou retirer la citation")

    # Duplicates: 12-word windows across different live documents
    allow = [" ".join(re.sub(r"[^\w\s]", " ", s.lower()).split()) for s in dc["allow_duplicates"]]
    seen, reported = {}, set()
    for p in live:
        ws = [(w, n) for n, t in strip_generated(read(p)) if not t.lstrip().startswith(("|", "#", "> Statut"))  # tables, headings and status lines are structure
              for w in re.sub(r"[^\w\s]", " ", t.lower()).split()]
        words = [w for w, _ in ws]
        skip = set()  # shortcut: windows overlapping an allowed phrase are skipped, not just those starting in it
        for s in allow:
            sw = s.split()
            for i in range(len(words) - len(sw) + 1):
                if sw and words[i:i + len(sw)] == sw:
                    skip.update(range(i - WINDOW + 1, i + len(sw)))
        for i in range(len(words) - WINDOW + 1):
            key = " ".join(words[i:i + WINDOW])
            first = seen.setdefault(key, (p, ws[i][1]))
            if first[0] != p and i not in skip and (first[0], p) not in reported:
                reported.add((first[0], p))
                add(rel(p), ws[i][1], f"phrase de 12 mots en double avec {rel(first[0])}:{first[1]}",
                    "garder le fait à un seul endroit et renvoyer vers lui")

    # Facts (generated sections)
    if a.check or a.write:
        for name, spec in facts.items():
            try:
                items = run_fact(root, name, spec)
            except (ValueError, json.JSONDecodeError, subprocess.TimeoutExpired) as e:
                add(f"facts.{name}", 0, f"fact illisible : {e}", f"corriger la commande `{spec['cmd']}`")
                continue
            if not spec.get("doc"):  # fact used by the map only
                continue
            doc = root / spec["doc"]
            if not doc.is_file():
                add(spec["doc"], 0, f"document du fact « {name} » absent", "créer le document avec ses marqueurs")
                continue
            text = doc.read_text()
            b = section_bounds(text, name)
            if isinstance(b, str):
                add(spec["doc"], 0, f"{b} : {name}", f"ajouter <!-- BEGIN generated:{name} --> et <!-- END generated:{name} -->")
                continue
            want = "\n" + render(items) + "\n"
            if a.write:
                if text[b[0]:b[1]] != want:
                    doc.write_text(text[:b[0]] + want + text[b[1]:])
                continue
            for i in items:
                if i["name"] not in text:
                    add(spec["doc"], 0, f"nom absent du document : {i['name']}", f"lancer doc-check --write")
            if text[b[0]:b[1]].strip() != want.strip():
                add(spec["doc"], 0, f"section générée périmée : {name}", "lancer doc-check --write")

    # Map (sibling script, built by another task): L only
    pm = Path(__file__).with_name("project-map.py")
    if scale == "L" and pm.is_file() and (a.check or a.write):
        cmd = [sys.executable, str(pm), "--root", str(root)] + ([] if a.write else ["--check"])
        if subprocess.run(cmd, capture_output=True).returncode:
            add("docs/project/carte.html", 0, "carte absente ou périmée", "lancer doc-check --write")

    if a.write:  # report what remains after the rewrite
        return 0 if not errs else (print("\n".join(errs)) or 1)
    print("\n".join(errs))
    print(f"doc-check: {len(errs)} écart(s)")
    return 1 if errs else 0


if __name__ == "__main__":
    sys.exit(main())

"""Build and validate the static docs site. Development-only; Python 3.11+."""

import argparse
from html import escape
from html.parser import HTMLParser
import json
from pathlib import Path
import shutil
from urllib.parse import urlparse
from xml.etree import ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
SITE_URL = "https://m-umer-farooq-dev.github.io/actually-done/"


class Links(HTMLParser):
    def __init__(self):
        super().__init__()
        self.local = []
        self.titles = 0
        self.h1 = 0
        self.description = False

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        self.titles += tag == "title"
        self.h1 += tag == "h1"
        self.description |= tag == "meta" and attrs.get("name") == "description"
        for key in ["href", "src"]:
            value = attrs.get(key, "")
            if value and not urlparse(value).scheme and not value.startswith(("#", "//")):
                self.local.append(value.split("#")[0])


def build(output, base_url=SITE_URL, verification=""):
    if not base_url.startswith("https://") or not base_url.endswith("/"):
        raise ValueError("Site URL must be HTTPS and end in /")
    output.mkdir(parents=True, exist_ok=True)
    shutil.copytree(ROOT / "site/assets", output / "assets", dirs_exist_ok=True)
    pages = json.loads((ROOT / "site/pages.json").read_text(encoding="utf-8"))
    urls = []
    for page in pages:
        slug = page["slug"]
        title, description = escape(page["title"]), escape(page["description"])
        canonical = base_url + ("" if slug == "index" else slug + ".html")
        content = (ROOT / "site/content" / (slug + ".html")).read_text(encoding="utf-8")
        schema = json.dumps({"@context": "https://schema.org", "@type": "SoftwareSourceCode", "name": "actually-done", "description": "Local completion evidence receipts for Claude Code and Codex", "codeRepository": "https://github.com/M-Umer-Farooq-Dev/actually-done", "programmingLanguage": "Rust", "license": "https://github.com/M-Umer-Farooq-Dev/actually-done/blob/main/LICENSE", "url": base_url})
        verification_tag = f'<meta name="google-site-verification" content="{escape(verification, quote=True)}">' if verification else ""
        html = f'''<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title><meta name="description" content="{description}"><link rel="canonical" href="{escape(canonical)}">
<meta property="og:title" content="{title}"><meta property="og:description" content="{description}"><meta property="og:type" content="website"><meta property="og:url" content="{escape(canonical)}">
<meta name="twitter:card" content="summary">{verification_tag}<link rel="stylesheet" href="assets/style.css">
<script type="application/ld+json">{schema}</script></head><body><a class="skip" href="#main">Skip to content</a>
<header><a class="brand" href="index.html">actually-done</a><nav aria-label="Main"><a href="install.html">Install</a><a href="agent-skill.html">Agent skill</a><a href="verdicts.html">FAQ</a><a href="https://github.com/M-Umer-Farooq-Dev/actually-done">GitHub ↗</a></nav></header>
<main id="main">{content}</main><footer><span>MIT · Local processing · Evidence with limits</span><a href="https://github.com/M-Umer-Farooq-Dev/actually-done/blob/main/SECURITY.md">Security reporting</a><a href="sitemap.xml">Sitemap</a></footer></body></html>'''
        (output / (slug + ".html")).write_text(html, encoding="utf-8", newline="\n")
        urls.append(canonical)
    sitemap = ET.Element("urlset", xmlns="http://www.sitemaps.org/schemas/sitemap/0.9")
    for url in urls:
        ET.SubElement(ET.SubElement(sitemap, "url"), "loc").text = url
    ET.ElementTree(sitemap).write(output / "sitemap.xml", encoding="utf-8", xml_declaration=True)
    (output / "robots.txt").write_text(f"User-agent: *\nAllow: /\nSitemap: {base_url}sitemap.xml\n", encoding="utf-8")
    (output / ".nojekyll").write_text("", encoding="utf-8")
    for path in output.glob("*.html"):
        links = Links()
        links.feed(path.read_text(encoding="utf-8"))
        if links.h1 != 1 or links.titles != 1 or not links.description:
            raise ValueError(f"Missing/duplicate title, H1, or description: {path.name}")
        for link in links.local:
            if not (path.parent / link).is_file():
                raise ValueError(f"Broken local link: {path.name} -> {link}")
    print(f"Built and validated {len(pages)} pages in {output}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=ROOT / "_site")
    parser.add_argument("--site-url", default=SITE_URL)
    parser.add_argument("--google-verification", default="")
    args = parser.parse_args()
    build(args.out, args.site_url, args.google_verification)

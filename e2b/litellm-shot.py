#!/usr/bin/env python3
"""litellm-shot — screenshot a LiteLLM UI page with a FRUGAL chromium so it never
trips the sandbox memory threshold (default chromium + full-page render OOM-kills
a 4 GB sandbox). Logs in via the real form and saves a viewport PNG (not full_page).

Usage:  litellm-shot <base_url> [page_query] [out_png]
  e.g.  litellm-shot http://127.0.0.1:4000 "?page=api-keys" /home/user/keys.png
"""
import os, sys
from playwright.sync_api import sync_playwright

base = sys.argv[1].rstrip("/") if len(sys.argv) > 1 else "http://127.0.0.1:4000"
page_q = sys.argv[2] if len(sys.argv) > 2 else "?page=api-keys"
out = sys.argv[3] if len(sys.argv) > 3 else "/home/user/keys.png"
master = os.environ.get("LITELLM_MASTER_KEY", "sk-1234")

# Frugal flags keep chromium's RSS low — this is the whole point of the helper.
ARGS = ["--no-sandbox", "--disable-dev-shm-usage", "--single-process",
        "--disable-gpu", "--no-zygote", "--disable-extensions"]

with sync_playwright() as p:
    b = p.chromium.launch(args=ARGS)
    ctx = b.new_context(viewport={"width": 1440, "height": 900})
    pg = ctx.new_page()
    pg.goto(f"{base}/ui/", wait_until="domcontentloaded")
    pg.wait_for_timeout(10000)  # routes lazy-compile on first hit
    try:
        if pg.query_selector('input[name="username"], input#username, input[type="text"]'):
            pg.fill('input[name="username"], input#username, input[type="text"]', "admin")
            pg.fill('input[name="password"], input#password, input[type="password"]', master)
            pg.click('button[type="submit"], button:has-text("Login"), button:has-text("Sign")')
            pg.wait_for_timeout(8000)
    except Exception as e:
        print(f"[litellm-shot] login step skipped: {e}", file=sys.stderr)
    pg.goto(f"{base}/ui/{page_q}", wait_until="domcontentloaded")
    pg.wait_for_timeout(10000)
    pg.screenshot(path=out)  # viewport only — NOT full_page (memory)
    b.close()
print(out)

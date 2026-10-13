#!/usr/bin/env python3
"""生成 Tauri updater 需要的 latest.json。

本地构建不会自动生成更新清单（那是 GitHub Action 的活），所以手工产出一份。
产物上传到 Release 后，应用通过 tauri.conf.json 里的
plugins.updater.endpoints 拿到它，比对版本号决定是否提示更新。

用法：
  python scripts/make-update-json.py \
    --version 0.4.1 \
    --sig "E:/tauri_build/.../ExpenseLedger_0.4.1_x64-setup.exe.sig" \
    --url  "https://github.com/.../download/ExpenseLedger_0.4.1_x64-setup.exe" \
    --notes release_notes.md \
    --out  latest.json
"""
import argparse
import json
import datetime
import pathlib

parser = argparse.ArgumentParser()
parser.add_argument("--version", required=True)
parser.add_argument("--sig", required=True, help=".sig 签名文件路径")
parser.add_argument("--url", required=True, help="安装包公开直链")
parser.add_argument("--notes", default="", help="更新说明文本文件，可选")
parser.add_argument("--out", required=True)
args = parser.parse_args()

signature = pathlib.Path(args.sig).read_text(encoding="utf-8").strip()
if not signature:
    raise SystemExit("签名文件为空，先确认构建时已设置 TAURI_SIGNING_PRIVATE_KEY")

notes = ""
if args.notes:
    p = pathlib.Path(args.notes)
    if p.exists():
        notes = p.read_text(encoding="utf-8").strip()

pub_date = datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0).isoformat()

manifest = {
    "version": args.version,
    "notes": notes,
    "pub_date": pub_date,
    "platforms": {
        "windows-x86_64": {
            "signature": signature,
            "url": args.url,
        }
    },
}

pathlib.Path(args.out).write_text(
    json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8"
)
print(f"已生成 {args.out}")
print(f"  version : {args.version}")
print(f"  pub_date: {pub_date}")
print(f"  url     : {args.url}")
print(f"  sig     : {signature[:48]}...")

#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
报销台账 Logo / 应用图标生成脚本（方案 A「账本凭证」）

设计语义：深空蓝圆角底板（沉稳可信）+ 白色凭证单据（台账/报销单）
         + 青蓝抬头栏（主色强调）+ 绿色核对勾（已核销/放心）
采用 4x 超采样绘制后降采样到目标尺寸，保证边缘平滑。
输出：
  icon-source.png   1024x1024  应用图标源（供 tauri icon 生成全套尺寸）
  logo-mark.svg     纯矢量标志（可无损缩放 / 印刷）
  logo-full.svg     矢量标志 + 中英文组合
  logo-wordmark.png 1200x300 横向组合版（文档、官网、README 用）
"""
import os
from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))

# ---- 品牌色（与 tailwind.config.js v4 token 保持一致）----
NAVY = (0x12, 0x3A, 0x5E, 255)      # deep #123A5E
WHITE = (0xFF, 0xFF, 0xFF, 255)
ACCENT = (0x4E, 0x9B, 0xC0, 255)    # accent #4E9BC0
GRAY = (0xB9, 0xC6, 0xD6, 255)      # line 旧色，作单据正文线
GRAY_L = (0xDC, 0xE4, 0xED, 255)    # line #DCE4ED
GREEN = (0x2E, 0x8B, 0x70, 255)     # income #2E8B70
INK = (0x1C, 0x2B, 0x3D, 255)       # ink #1C2B3D
MUTED = (0x5B, 0x72, 0x88, 255)     # ink-400

SS = 4                 # 超采样倍数
OUT = 1024             # 最终图标边长
C = OUT * SS           # 画布边长（4096）

# 归一化比例：以宽容 carbon artboard 为 140 单位（与确认稿一致）
tile_inset = int(24 / 1024 * C)          # 底板留白
tile = C - tile_inset * 2
tile_radius = int(34 / 140 * tile)       # 圆角 ≈ 34/140
rx, ry = tile_inset, tile_inset          # 底板左上
cx = cy = C // 2

rec_w = int(70 / 140 * tile)
rec_h = int(92 / 140 * tile)
rec_x = cx - rec_w // 2
rec_y = cy - rec_h // 2
rec_radius = int(8 / 70 * rec_w)

img = Image.new("RGBA", (C, C), (0, 0, 0, 0))
d = ImageDraw.Draw(img)

# 底板
d.rounded_rectangle(
    [rx, ry, rx + tile, ry + tile], radius=tile_radius, fill=NAVY
)

# 白色凭证单据
d.rounded_rectangle(
    [rec_x, rec_y, rec_x + rec_w, rec_y + rec_h], radius=rec_radius, fill=WHITE
)


def line(frac_x, frac_y, frac_w, frac_h, color):
    """按凭证单据的比例绘制一行账目线"""
    x = rec_x + frac_x * rec_w
    y = rec_y + frac_y * rec_h
    w = frac_w * rec_w
    h = frac_h * rec_h
    d.rounded_rectangle([x, y, x + w, y + h], radius=max(h // 2, 1), fill=color)


line(0.171, 0.1522, 0.657, 0.0761, ACCENT)  # 抬头栏（主色强调）
line(0.171, 0.3261, 0.486, 0.0543, GRAY)
line(0.171, 0.4565, 0.571, 0.0543, GRAY)
line(0.171, 0.5870, 0.371, 0.0543, GRAY_L)

# 核对勾（绿色圆章 + 白色勾）
cir_r = int(15 / 140 * tile)
cir_x = rec_x + int(0.757 * rec_w)
cir_y = rec_y + int(0.848 * rec_h)
d.ellipse(
    [cir_x - cir_r, cir_y - cir_r, cir_x + cir_r, cir_y + cir_r], fill=GREEN
)

s = cir_r / 15.0  # 由 15 单位半径映射到实际像素
d.line(
    [
        (cir_x - 6 * s, cir_y),
        (cir_x - 1 * s, cir_y + 5 * s),
        (cir_x + 6 * s, cir_y - 5 * s),
    ],
    fill=WHITE,
    width=max(int(2.5 * s), 1),
    joint="curve",
)

icon = img.resize((OUT, OUT), Image.LANCZOS)
icon_path = os.path.join(HERE, "icon-source.png")
icon.save(icon_path)
print("icon-source.png ->", icon_path, icon.size)


# ---- 横向组合版（文档 / 官网用）----
def load_font(size, bold=False):
    cands = [
        "C:/Windows/Fonts/msyhbd.ttc" if bold else "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/simhei.ttf",
    ]
    for p in cands:
        if os.path.exists(p):
            try:
                return ImageFont.truetype(p, size)
            except Exception:
                pass
    return ImageFont.load_default()


W, H = 1200, 300
wm = Image.new("RGBA", (W, H), (0, 0, 0, 0))
wd = ImageDraw.Draw(wm)
mark = Image.open(icon_path).resize((232, 232), Image.LANCZOS)
wm.paste(mark, (34, (H - 232) // 2), mark)

f1 = load_font(96, bold=True)
f2 = load_font(34)
wd.text((316, 92), "报销台账", font=f1, fill=INK)
wd.text((320, 196), "EXPENSE LEDGER", font=f2, fill=MUTED)

wm_path = os.path.join(HERE, "logo-wordmark.png")
wm.save(wm_path)
print("logo-wordmark.png ->", wm_path, wm.size)

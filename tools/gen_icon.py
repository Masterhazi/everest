"""Launcher icon: a snow peak at night, the friend's yellow coat flying from a pole on the summit.
Pixel art on a 72px grid (the visible part of a 108dp adaptive icon), scaled with nearest-neighbour."""
import os, numpy as np
from PIL import Image, ImageDraw
rng=np.random.default_rng(13)
V=72

def sky(n):
    a=np.zeros((n,n,4),np.uint8)
    top=np.array([8,14,32]); bot=np.array([52,78,118])
    for y in range(n):
        t=(y/n)**1.3; a[y,:,:3]=(top+(bot-top)*t).astype(np.uint8)
    a[...,3]=255
    im=Image.fromarray(a,'RGBA'); d=ImageDraw.Draw(im)
    for _ in range(n//4):
        x,y=int(rng.integers(0,n)),int(rng.integers(0,int(n*0.45))); v=int(rng.integers(150,230))
        d.point((x,y),fill=(v,v,min(255,v+25),255))
    return im

def art():
    im=Image.new('RGBA',(V,V),(0,0,0,0)); d=ImageDraw.Draw(im)
    # far ridge
    d.polygon([(0,58),(12,44),(20,50),(28,40),(36,72),(0,72)],fill=(40,58,90,255))
    d.polygon([(44,72),(58,46),(66,52),(72,44),(72,72)],fill=(40,58,90,255))
    # the peak: lit snow face on the left, shadowed face on the right
    summit=(36,20)
    d.polygon([(4,72),(18,50),(26,44),summit,(36,72)],fill=(232,240,250,255))
    d.polygon([summit,(46,38),(54,50),(68,72),(36,72)],fill=(120,145,185,255))
    # rock bands and snow texture
    for (x0,y0,x1,y1) in [(20,56,30,52),(14,64,24,60),(40,48,48,54),(46,60,56,64)]:
        d.line([(x0,y0),(x1,y1)],fill=(70,82,110,255),width=1)
    for _ in range(40):
        x,y=int(rng.integers(8,36)),int(rng.integers(30,72))
        if im.getpixel((x,y))[:3]==(232,240,250): d.point((x,y),fill=(205,218,236,255))
    # pole and the friend's yellow coat, hung by its hood, blown sideways by the wind
    d.line([(36,21),(36,4)],fill=(60,45,35,255),width=1)
    coat=(226,178,56,255); dark=(168,120,30,255); light=(246,210,110,255)
    d.rectangle([37,4,40,6],fill=coat)                              # hood on the pole
    d.polygon([(37,7),(48,8),(47,19),(38,19)],fill=coat)             # body, flaring in the wind
    d.line([(47,9),(55,13)],fill=coat,width=3)                       # sleeve streaming out
    d.line([(46,14),(53,19)],fill=coat,width=2)                      # other sleeve
    d.line([(38,7),(46,8)],fill=light)                               # light on the shoulders
    d.line([(42,8),(42,19)],fill=dark)                               # zip
    d.point((39,14),fill=dark); d.point((45,14),fill=dark)           # pockets
    d.line([(38,19),(47,19)],fill=dark)                              # hem
    # wind streaks
    for (x,y,l) in [(57,10,5),(50,23,6),(46,4,5)]:
        d.line([(x,y),(x+l,y)],fill=(220,230,245,160))
    return im

a=art(); bg=sky(V)
legacy=bg.copy(); legacy.alpha_composite(a)
os.makedirs('res/mipmap-anydpi-v26',exist_ok=True)
for name,scale in [('mdpi',1),('hdpi',1.5),('xhdpi',2),('xxhdpi',3),('xxxhdpi',4)]:
    dpath=f'res/mipmap-{name}'; os.makedirs(dpath,exist_ok=True)
    n=int(48*scale)                         # legacy icon is 48dp
    legacy.resize((n,n),Image.NEAREST).save(f'{dpath}/ic_launcher.png')
    full=int(108*scale)                     # adaptive layers are 108dp; art sits in the middle 72dp
    fg=Image.new('RGBA',(108,108),(0,0,0,0)); fg.paste(a,(18,18),a)
    fg.resize((full,full),Image.NEAREST).save(f'{dpath}/ic_launcher_foreground.png')
    sky(108).resize((full,full),Image.NEAREST).save(f'{dpath}/ic_launcher_background.png')
open('res/mipmap-anydpi-v26/ic_launcher.xml','w').write('''<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@mipmap/ic_launcher_background"/>
    <foreground android:drawable="@mipmap/ic_launcher_foreground"/>
</adaptive-icon>
''')
legacy.resize((512,512),Image.NEAREST).save('assets/icon_512.png')
print('ok')

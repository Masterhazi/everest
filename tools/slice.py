"""Cut the concept sheet into placeholder game assets (transparent PNGs)."""
import numpy as np, colorsys, json
from PIL import Image
from scipy import ndimage
SRC='/mnt/user-data/uploads/1000362069.png'
OUT='assets/sprites/'
sheet=np.asarray(Image.open(SRC).convert('RGB')).astype(int)

def cutout(box, tol=34):
    x0,y0,x1,y1=box
    reg=sheet[y0:y1,x0:x1]
    # background = dark navy; remove only regions connected to the crop border
    bgref=np.median(np.concatenate([reg[0],reg[-1],reg[:,0],reg[:,-1]]),axis=0)
    near=np.abs(reg-bgref).sum(2)<tol
    lab,_=ndimage.label(near)
    border=set(np.unique(np.concatenate([lab[0],lab[-1],lab[:,0],lab[:,-1]])))-{0}
    bg=np.isin(lab,list(border))
    fg=ndimage.binary_fill_holes(ndimage.binary_closing(~bg,iterations=2))
    a=np.where(fg,255,0).astype(np.uint8)
    rgba=np.dstack([reg.astype(np.uint8),a])
    im=Image.fromarray(rgba,'RGBA')
    bb=im.getbbox()
    return im.crop(bb) if bb else im

def to_yellow(im):
    """Recolour red/orange fabric to faded mustard (friend's coat)."""
    a=np.asarray(im).astype(float)/255
    out=a.copy()
    for y in range(a.shape[0]):
        for x in range(a.shape[1]):
            r,g,b,al=a[y,x]
            if al==0: continue
            h,s,v=colorsys.rgb_to_hsv(r,g,b)
            if (h<0.09 or h>0.93) and s>0.35 and v>0.18:
                nr,ng,nb=colorsys.hsv_to_rgb(0.125,min(1,s*0.9+0.05),min(1,v*1.4+0.04))
                out[y,x,:3]=(nr,ng,nb)
    return Image.fromarray((out*255).astype(np.uint8),'RGBA')

# ---- protagonist: 9 rows, bottom-centre aligned into uniform cells
rows={
 'idle':[(105,32,146,104),(173,32,214,104),(238,30,280,104),(309,30,353,103),(383,33,424,104)],
 'walk':[(103,110,148,179),(171,110,215,179),(235,109,281,179),(302,110,348,179),(373,110,421,179)],
 'climb':[(107,182,161,261),(176,183,226,261),(241,186,295,261),(308,186,365,261),(380,187,440,261)],
 'axe':[(111,264,157,343),(172,268,224,342),(235,268,288,343),(300,268,353,342),(366,269,425,343)],
 'rope':[(98,347,149,418),(159,348,215,416),(223,348,280,418),(292,350,356,418),(375,349,429,417)],
 'dig':[(108,421,174,484),(190,421,245,484),(265,425,336,484),(348,440,436,484)],
 'exhausted':[(106,495,154,561),(175,495,222,561),(243,502,288,562),(301,508,358,561),(373,514,435,561)],
 'fall':[(101,565,175,634),(174,573,241,633),(244,585,325,633),(335,595,423,633)],
 'injured':[(103,645,171,694),(181,646,257,693),(265,651,355,693),(358,660,437,694)],
}
CW,CH=96,88
atlas=Image.new('RGBA',(CW*5,CH*len(rows)),(0,0,0,0))
meta={}
for r,(name,boxes) in enumerate(rows.items()):
    meta[name]={'row':r,'frames':len(boxes)}
    for c,b in enumerate(boxes):
        f=cutout(b)
        atlas.alpha_composite(f,(c*CW+(CW-f.width)//2, r*CH+CH-f.height-2))
atlas.save(OUT+'hero.png')
json.dump({'cell':[CW,CH],'rows':meta},open(OUT+'hero.json','w'),indent=1)

# ---- single sprites
single={
 'ice_axe':(481,30,550,128),'rope':(655,25,755,127),'crampons':(781,25,932,128),
 'shovel':(953,25,1067,130),'headlamp':(1076,48,1197,101),
 'backpack':(471,174,582,296),'backpack_coat_red':(605,171,727,296),
 'coat_red':(765,179,909,295),'cross':(931,178,1034,298),
 'tent':(467,561,599,662),'flags':(702,560,815,660),'anchor':(824,566,915,661),
 'camp_gear':(599,551,700,660),
}
for k,b in single.items(): cutout(b).save(OUT+k+'.png')
to_yellow(cutout(single['coat_red'])).save(OUT+'coat_friend.png')
to_yellow(cutout(single['backpack_coat_red'])).save(OUT+'backpack_coat.png')

# ---- UI (keep their framed backgrounds)
icons={'ui_axe':(15,747,102,835),'ui_rope':(120,746,208,835),'ui_dig':(227,747,314,835),
 'ui_anchor':(334,747,422,835),'ui_crampons':(441,747,534,835),'ui_headlamp':(551,747,644,835),
 'ui_rest':(662,747,754,835),'ui_shelter':(777,747,870,835)}
for k,(x0,y0,x1,y1) in icons.items():
    Image.fromarray(sheet[y0:y1,x0:x1].astype(np.uint8)).convert('RGBA').save(OUT+k+'.png')
cutout((946,747,1039,841),tol=20).save(OUT+'joy_base.png')
# knob: bright disc from drag state
cutout((1120,775,1165,815),tol=60).save(OUT+'joy_knob.png')

# ---- terrain tiles
tiles={'t_snow':(472,382,557,476),'t_ice':(572,382,659,476),'t_rock':(673,382,759,476),
 't_ledge':(773,382,865,476),'t_crevasse':(882,382,975,476),'t_cornice':(991,382,1084,476),
 't_avalanche':(1099,382,1195,476)}
for k,(x0,y0,x1,y1) in tiles.items():
    Image.fromarray(sheet[y0:y1,x0:x1].astype(np.uint8)).convert('RGBA').save(OUT+k+'.png')
print('ok')

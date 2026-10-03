"""Assets for the continuous-mountain version: repeating textures, headlamp masks, seracs."""
import numpy as np
from PIL import Image, ImageDraw
S='assets/sprites/'
rng=np.random.default_rng(3)

def rep(src,size,name,b=1.0):
    im=Image.open(S+src).convert('RGB').resize(size,Image.BOX)
    Image.fromarray(np.clip(np.asarray(im).astype(float)*b,0,255).astype(np.uint8)).save(S+name)
rep('t_rock.png',(64,64),'tex_body.png',0.8)
rep('t_rock.png',(40,40),'tex_rock.png',1.3)
rep('t_ice.png',(44,44),'tex_ice.png')
rep('t_snow.png',(40,40),'tex_snow.png')

def mask(name,r,soft):
    N=400; yy,xx=np.mgrid[0:N,0:N]; d=np.sqrt((xx-N/2)**2+(yy-N/2)**2)*4
    a=np.round(np.clip((d-r)/soft,0,1)*6)/6   # banded falloff reads as pixel art
    Image.fromarray(np.dstack([np.full((N,N),5),np.full((N,N),8),np.full((N,N),18),a*255]).astype(np.uint8),'RGBA').save(S+name)
mask('lamp_on.png',90,110)
mask('lamp_off.png',22,40)

# serac: a leaning, jagged ice tower, wider at the base, lit from the left
ice=np.asarray(Image.open(S+'t_ice.png').convert('RGB').resize((40,40),Image.BOX)).astype(float)
for k in range(3):
    W,H=34,96
    m=Image.new('L',(W,H),0); d=ImageDraw.Draw(m)
    top=[(int(rng.integers(4,12)),int(rng.integers(0,10))),(int(rng.integers(12,20)),int(rng.integers(0,6))),(int(rng.integers(20,28)),int(rng.integers(4,14)))]
    poly=[(0,H-1)]+[(2,int(H*0.55))]+top+[(W-4,int(H*0.4)),(W-1,H-1)]
    d.polygon(poly,fill=255)
    a=np.asarray(m)>0
    img=np.zeros((H,W,4))
    for y in range(H):
        for x in range(W):
            if not a[y,x]: continue
            shade=1.15-0.5*(x/W)+0.1*np.sin(y*0.35+k)
            img[y,x,:3]=np.clip(ice[y%40,x%40]*shade,0,255); img[y,x,3]=255
    # a few dark fracture lines
    im=Image.fromarray(img.astype(np.uint8),'RGBA'); dd=ImageDraw.Draw(im)
    for _ in range(3):
        y0=int(rng.integers(15,H-10)); dd.line([(int(rng.integers(3,10)),y0),(int(rng.integers(18,30)),y0+int(rng.integers(-6,6)))],fill=(70,100,140,255),width=1)
    im.save(S+f'serac{k}.png')
print('ok')

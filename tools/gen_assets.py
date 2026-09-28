"""Generate placeholder backgrounds, tiles and audio for the vertical slice."""
import numpy as np, random
from PIL import Image, ImageDraw, ImageFilter
from scipy import ndimage
from scipy.io import wavfile
from scipy.signal import butter, lfilter
S='assets/sprites/'; A='assets/audio/'
rng=np.random.default_rng(7); random.seed(7)

def lerp(a,b,t): return tuple(int(a[i]+(b[i]-a[i])*t) for i in range(3))

def ridge(w, base, amp, rough, seed):
    r=np.random.default_rng(seed); ys=[]; y=base
    for x in range(w):
        y+=r.normal(0,rough); y+= (base-y)*0.02
        ys.append(y)
    ys=np.array(ys)
    # add a few big peaks
    for _ in range(3):
        c=r.integers(0,w); h=r.uniform(0.5,1)*amp; wd=r.uniform(12,30)
        ys+=h*np.maximum(0,1-np.abs(np.arange(w)-c)/wd)
    return ys

def background(top,mid,hor,far,near,snow,name,stars=True):
    W,H=120,300
    im=Image.new('RGB',(W,H)); d=ImageDraw.Draw(im)
    for y in range(H):
        t=y/H
        c=lerp(top,mid,t/0.6) if t<0.6 else lerp(mid,hor,(t-0.6)/0.4)
        d.line([(0,y),(W,y)],fill=c)
    if stars:
        for _ in range(40):
            x,y=random.randrange(W),random.randrange(int(H*0.5))
            v=random.randint(120,210); d.point((x,y),fill=(v,v,min(255,v+30)))
    for (col,base,amp,rough,seed,cap) in [(far,H*0.46,70,0.9,1,0.35),(near,H*0.6,55,1.2,2,0.25)]:
        ys=ridge(W,base,amp,rough,seed)
        for x in range(W):
            top_y=int(H-ys[x]) if False else int(ys[x]- (ys.max()-ys[x])*0 )
        pk=ridge(W,0,amp,rough,seed)
        for x in range(W):
            yy=int(base-pk[x])
            d.line([(x,yy),(x,H)],fill=col)
            capn=int(pk[x]*cap)
            if capn>2: d.line([(x,yy),(x,yy+capn//3)],fill=lerp(col,snow,0.55))
    im=im.resize((W*3,H*3),Image.NEAREST); im.save(S+name)

background((8,14,30),(22,40,70),(70,95,130),(44,62,92),(28,40,62),(200,215,235),'bg_day.png')
background((30,24,52),(120,70,90),(240,150,90),(90,70,100),(50,40,66),(250,200,170),'bg_sunset.png',stars=False)

def load(n): return Image.open(S+n).convert('RGBA')
def tile(src,size,name,bright=1.0,alpha=255):
    im=load(src).resize(size,Image.BOX)
    a=np.asarray(im).astype(float); a[...,:3]*=bright; a[...,3]=alpha
    Image.fromarray(np.clip(a,0,255).astype(np.uint8)).save(S+name)
tile('t_ledge.png',(40,36),'ledge.png')
tile('t_ice.png',(44,44),'wall_ice.png')
tile('t_rock.png',(40,40),'wall_rock.png',1.25)

tile('t_snow.png',(40,40),'snowfield.png')

# boulder: rock ellipse with snow cap
rock=load('t_rock.png').resize((72,58)); m=Image.new('L',(72,58),0)
ImageDraw.Draw(m).polygon([(4,57),(10,24),(24,8),(44,4),(60,16),(70,40),(68,57)],fill=255)
rock.putalpha(m); cap=load('t_snow.png').resize((72,58)); cm=Image.new('L',(72,58),0)
ImageDraw.Draw(cm).polygon([(12,22),(24,6),(44,2),(60,14),(56,20),(40,14),(26,18)],fill=255)
rock.paste(cap,(0,0),cm); rock.save(S+'boulder.png')

# snow mound / avalanche mass from the avalanche tile (cut the white heap off its dark bg)
av=np.asarray(load('t_avalanche.png')).astype(int)
lum=av[...,:3].mean(2); mask=ndimage.binary_opening(lum>120,iterations=1)
mask=ndimage.binary_fill_holes(mask)
av[...,3]=np.where(mask,255,0)
heap=Image.fromarray(av.astype(np.uint8)); heap=heap.crop(heap.getbbox())
heap.resize((70,44),Image.BOX).save(S+'mound.png')
heap.resize((260,210),Image.NEAREST).save(S+'avalanche.png')

# vignette
W,H=360,640; yy,xx=np.mgrid[0:H,0:W]; r=np.sqrt(((xx-W/2)/(W*0.62))**2+((yy-H/2)/(H*0.62))**2)
al=np.clip((r-0.55)/0.5,0,1)**1.6*255
Image.fromarray(np.dstack([np.zeros((H,W,3)),al]).astype(np.uint8),'RGBA').save(S+'vignette.png')

# crack in snow
c=Image.new('RGBA',(140,14),(0,0,0,0)); d=ImageDraw.Draw(c); x,y=0,7; pts=[]
while x<140: pts.append((x,y)); x+=random.randint(4,9); y=max(2,min(11,y+random.randint(-3,3)))
d.line(pts,fill=(30,40,60,255),width=2); c.save(S+'crack.png')

# ---------------- audio
SR=22050
def bp(x,lo,hi,o=2):
    b,a=butter(o,[lo/(SR/2),hi/(SR/2)],btype='band'); return lfilter(b,a,x)
def lp(x,f,o=2):
    b,a=butter(o,f/(SR/2)); return lfilter(b,a,x)
def save(name,x,g=0.9):
    x=x/ (np.abs(x).max()+1e-9)*g; wavfile.write(A+name,SR,(x*32767).astype(np.int16))
def loopfade(x,n=2000):
    # crossfade tail into head for a seamless loop
    head=x[:n].copy(); x=x[n:]; x[-n:]=x[-n:]*np.linspace(1,0,n)+head*np.linspace(0,1,n); return x
t=lambda d: np.arange(int(SR*d))/SR
# wind: band noise with slow swells
d=12; n=rng.normal(0,1,int(SR*d)+2000); tt=np.arange(len(n))/SR
env=0.55+0.3*np.sin(2*np.pi*tt/6.0)+0.15*np.sin(2*np.pi*tt/2.3+1)
w=bp(n,180,900)*env+0.5*bp(n,60,200)
save('wind.wav',loopfade(w),0.8)
# gust
g=bp(rng.normal(0,1,int(SR*2.2)),250,1600)*np.sin(np.pi*t(2.2)/2.2)**2; save('gust.wav',g)
# breathing loop: inhale/exhale 3s
tb=t(3.0); env=np.where(tb<1.2,np.sin(np.pi*tb/1.2)**2*0.7,0)+np.where((tb>1.4)&(tb<2.8),np.sin(np.pi*(tb-1.4)/1.4)**2,0)
br=bp(rng.normal(0,1,len(tb)),400,2600)*env; save('breath.wav',br,0.7)
# heartbeat loop 1.1s
th=t(1.1); hb=np.zeros_like(th)
for s0 in (0.0,0.28):
    m=(th>=s0)&(th<s0+0.16); hb[m]+=np.sin(2*np.pi*48*(th[m]-s0))*np.exp(-(th[m]-s0)*22)
save('heart.wav',hb)
# whumpf: deep thud
tw=t(1.0); wh=np.sin(2*np.pi*(55-20*tw)*tw)*np.exp(-tw*5)+0.4*lp(rng.normal(0,1,len(tw)),150)*np.exp(-tw*6)
save('whumpf.wav',wh)
# rumble: growing low roar
tr=t(6.0); ru=lp(rng.normal(0,1,len(tr)),220,3)*np.clip(tr/3.5,0,1)**1.5*np.clip((6-tr)/0.8,0,1)
save('rumble.wav',ru)
# crunch (dig/step)
tc=t(0.22); cr=bp(rng.normal(0,1,len(tc)),900,5000)*np.exp(-tc*18)*(rng.random(len(tc))>0.3)
save('crunch.wav',cr,0.6)
# axe chink
tk=t(0.45); ch=(np.sin(2*np.pi*2150*tk)+0.5*np.sin(2*np.pi*3370*tk))*np.exp(-tk*14)+bp(rng.normal(0,1,len(tk)),2000,6000)*np.exp(-tk*60)
save('chink.wav',ch,0.5)
# memory pad: soft drone chord (placeholder, NOT the song)
tm=t(16.0); pad=sum(np.sin(2*np.pi*f*tm+ph)*(1+0.3*np.sin(2*np.pi*tm/(5+i))) for i,(f,ph) in enumerate([(110,0),(164.8,1),(220,2),(277.2,.5)]))
pad=lp(pad,900)*np.clip(tm/3,0,1)*np.clip((16-tm)/3,0,1); save('memory.wav',pad,0.5)
# pickup chime for the cross
tp=t(1.2); pk=np.sin(2*np.pi*880*tp)*np.exp(-tp*4)+0.5*np.sin(2*np.pi*1320*tp)*np.exp(-tp*5); save('chime.wav',pk,0.35)
print('assets ok')

# ---------------- mountain face: two rock buttresses with a gully of open sky between
def face():
    W,H=120,334
    rockt=np.asarray(load('t_rock.png').resize((40,40),Image.BOX).convert('RGB')).astype(float)
    snowt=np.asarray(load('t_snow.png').resize((40,40),Image.BOX).convert('RGB')).astype(float)
    r=np.random.default_rng(11)
    def walk(base,amp):
        v=[];y=base
        for i in range(H):
            y+=r.normal(0,0.9); y+=(base-y)*0.04; v.append(y)
        v=np.array(v); v+=amp*np.sin(np.arange(H)/37.0+r.uniform(0,6))
        return v
    le=walk(24,7); re=walk(96,6)
    out=np.zeros((H,W,4))
    for y in range(H):
        for x in range(W):
            inl=x<le[y]; inr=x>re[y]
            if not(inl or inr): continue
            edge=min(abs(x-le[y]),abs(x-re[y]))
            c=rockt[y%40,x%40]*0.62
            # snow clings to the upper side of the rock and near the edges
            if (r.random()<0.10 and edge>3) or (edge<1.5 and r.random()<0.6):
                c=snowt[y%40,x%40]*0.85
            out[y,x,:3]=c; out[y,x,3]=255
    im=Image.fromarray(out.astype(np.uint8),'RGBA').transpose(Image.FLIP_TOP_BOTTOM)
    im.resize((W*3,H*3),Image.NEAREST).save(S+'face.png')
face()

# ---------------- avalanche: a churning heap of snow, soft pixel edges
def avalanche():
    W,H=90,72
    r=np.random.default_rng(5); m=np.zeros((H,W))
    yy,xx=np.mgrid[0:H,0:W]
    for _ in range(26):
        cx=r.uniform(8,W-8); cy=r.uniform(H*0.35,H-6); rad=r.uniform(7,17)
        cy=min(cy,H-rad*0.6)
        m=np.maximum(m,1-np.sqrt((xx-cx)**2+(yy-cy)**2)/rad)
    mask=m>0.02
    snowt=np.asarray(load('t_snow.png').resize((45,45),Image.BOX).convert('RGB')).astype(float)
    col=np.zeros((H,W,3))
    for y in range(H):
        for x in range(W):
            shade=0.78+0.35*m[y,x]-0.25*(y/H)
            col[y,x]=np.clip(snowt[y%45,x%45]*shade,0,255)
    a=np.where(mask,255,0)
    im=Image.fromarray(np.dstack([col,a]).astype(np.uint8),'RGBA')
    im.resize((W*3,H*3),Image.NEAREST).save(S+'avalanche.png')
avalanche()
print('face + avalanche ok')

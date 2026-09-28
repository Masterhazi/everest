import sys, numpy as np
from PIL import Image
from scipy import ndimage
im=np.asarray(Image.open('/mnt/user-data/uploads/1000362069.png').convert('RGB')).astype(int)
def boxes(x0,y0,x1,y1,thr=40,dil=3,minarea=150):
    reg=im[y0:y1,x0:x1]
    bg=np.array([6,14,24])
    m=(np.abs(reg-bg).sum(2)>thr)
    md=ndimage.binary_dilation(m,iterations=dil)
    lab,n=ndimage.label(md)
    out=[]
    for sl in ndimage.find_objects(lab):
        h=sl[0].stop-sl[0].start; w=sl[1].stop-sl[1].start
        if h*w>=minarea: out.append((sl[1].start+x0,sl[0].start+y0,sl[1].stop+x0,sl[0].stop+y0))
    return sorted(out,key=lambda b:(b[1]//20,b[0]))
if __name__=='__main__':
    a=list(map(int,sys.argv[1:5]))
    for b in boxes(*a): print(b, b[2]-b[0], b[3]-b[1])

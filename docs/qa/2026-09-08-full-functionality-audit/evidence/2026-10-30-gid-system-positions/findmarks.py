# Find exact keyed occurrences of small STRATEGY bitmaps in a 640x480 capture.
import sys, os, json
import numpy as np
from PIL import Image
BMP='data/base/ui/strategy-dll/BMP/'
IDS=[i for i in list(range(10130,10144))+list(range(10146,10171))+[10180,10181] if os.path.exists(BMP+'%d.bmp'%i)]
def keyed(i):
    im=Image.open(BMP+'%d.bmp'%i).convert('RGB'); a=np.asarray(im).astype(np.int16)
    key=a[-1,0]
    blue=(a[...,0]<32)&(a[...,1]<32)&(a[...,2]>192)
    mask=~((a==key).all(-1)|blue)
    return a,mask
TPL={i:keyed(i) for i in IDS}
def find(path, box=(110,60,610,425)):
    img=np.asarray(Image.open(path).convert('RGB')).astype(np.int16)
    hits=[]
    for i,(t,m) in TPL.items():
        h,w=m.shape
        if m.sum()<6: continue
        ys,xs=np.nonzero(m); vals=t[ys,xs]
        y0,x0=ys[0],xs[0]; v0=vals[0]
        x1,y1,x2,y2=box
        sub=img[y1:y2-h, x1:x2-w]
        cand=np.argwhere((img[y1+y0:y2-h+y0, x1+x0:x2-w+x0]==v0).all(-1))
        for cy,cx in cand:
            Y=y1+cy; X=x1+cx
            if (img[Y+ys, X+xs]==vals).all(): hits.append((i,int(X),int(Y)))
    return hits
if __name__=='__main__':
    out={}
    for p in sys.argv[1:]:
        out[p]=find(p)
    json.dump(out,sys.stdout)

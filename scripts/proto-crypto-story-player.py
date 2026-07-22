# Story trace-player prototype: real AES {panels,steps} -> a STORY-driven portrait
# reel (flow strip + one live state grid with per-op highlighting + S-box lookup
# callout + round-key flash + step info). Existing manic vocab only, no crypto builtins.
import json, sys
trace_path, out_path = sys.argv[1], sys.argv[2]
d = json.load(open(trace_path))
# encryption round steps only (skip the 11 key-schedule expand beats; summarise once)
steps = [s for s in d['steps'] if s.get('target')=='state' and 'after' in s and s['op']!='expand']

ACCENT = {'load':'cyan','xor':'lime','sub':'cyan','permute':'gold','mix':'magenta','emit':'lime'}
CHIP   = {'sub':0,'permute':1,'mix':2,'xor':3}   # which flow chip lights up
FKEY   = {'sub':'sub','permute':'shift','mix':'mix','xor':'xor'}

L=[]; w=L.append
w('title("AES-128 — How It Works");')
w('canvas("portrait");')
w('template("mono");')
w('')
w('text(hdr, (cx, 120), "AES-128 · encrypt");  size(hdr, 42); bold(hdr); color(hdr, cyan);')
w('')
w('// --- FULL FLOW strip (you-are-here) ---')
chips=[("SubBytes",0),("ShiftRows",1),("MixColumns",2),("AddRoundKey",3)]
xs=[cx for cx in [270,490,700,900]]
for lab,i in chips:
    w(f'text(chip{i}, ({xs[i]}, 210), "{lab}");  size(chip{i}, 19); color(chip{i}, dim);')
w('text(loop, (cx, 250), "the round repeats x10");  size(loop, 18); color(loop, dim);')
w('text(rnd, (cx, 300), "");  size(rnd, 24); color(rnd, cyan); bold(rnd);')
w('')
w('// --- the live 4x4 state (cells relabelled + highlighted in place) ---')
first = steps[0]['after']
def rowstr(a,i): return " ".join(a[i])
w(f'matrix(st, "{"; ".join(rowstr(first,i) for i in range(4))}", (cx, cy - 40), 150, 132);')
w('')
w('// --- formulas (one per op, shown when active) ---')
w('equation(f_sub,   (cx, cy - 470), `b\' = S(b)`, 40);            hidden(f_sub);')
w('equation(f_shift, (cx, cy - 470), `\\text{row } r \\lll r`, 40);  hidden(f_shift);')
w('equation(f_mix,   (cx, cy - 470), `c\' = M\\,c`, 40);            hidden(f_mix);')
w('equation(f_xor,   (cx, cy - 470), `\\text{state} \\oplus K_n`, 40); hidden(f_xor);')
w('')
w('// --- secondary beat: S-box lookup callout (SubBytes) / round key (AddRoundKey) ---')
w('text(sbox, (cx, cy + 430), "");  size(sbox, 26); color(sbox, cyan);  hidden(sbox);')
w('matrix(rk, "00 00 00 00; 00 00 00 00; 00 00 00 00; 00 00 00 00", (cx, cy + 470), 120, 96);')
w('color(rk, lime); hidden(rk);')
w('text(rklab, (cx, cy + 300), "round key");  size(rklab, 20); color(rklab, dim); hidden(rklab);')
w('')
w('text(cap, (cx, h - 210), "");   size(cap, 28); wrap(cap, w*0.82); bold(cap);')
w('text(exp, (cx, h - 150), "");   size(exp, 22); color(exp, dim); wrap(exp, w*0.84);')
w('text(mark, (w - 150, h - 44), "Made with Manic");  size(mark, 22); color(mark, dim);')
w('')
w('// ---------- timeline ----------')
w('show(hdr, 0.4); show(mark, 0.4);')
for lab,i in chips: w(f'show(chip{i}, 0.3);')
w('show(loop, 0.3); show(st, 0.5);')
w(f'say(cap, "{steps[0]["title"]}", 0.3);')
w(f'say(exp, "{d["steps"][0].get("explain","")[:90]}", 0.3);')
w('wait(0.8);')
w('')

prev_f=None
def cells_say(after, dur):
    out=[]
    for i in range(4):
        for j in range(4):
            out.append(f'say(st.r{i}c{j}, "{after[i][j]}", {dur})')
    return out

for k in range(1, len(steps)):
    s=steps[k]; op=s['op']; acc=ACCENT.get(op,'cyan')
    ph=s.get('phase',''); title=s.get('title','').replace('"',"'"); explain=s.get('explain','').replace('"',"'")[:92]
    w(f'step("{ph} · {op}") {{')
    # flow strip: light the active chip, dim the rest; round counter
    if op in CHIP:
        for _,i in chips:
            col = acc if i==CHIP[op] else 'dim'
            w(f'  recolor(chip{i}, {col}, 0.15);')
    if ph.startswith('Round') and ph.split()[-1].isdigit() and ph.split()[-1]!='0':
        w(f'  say(rnd, "Round {ph.split()[-1]} / 10", 0.15);')
    elif 'Round 0' in ph or op=='load':
        w(f'  say(rnd, "initial", 0.15);')
    # formula
    fk = FKEY.get(op)
    cur_f = f'f_{fk}' if fk else None
    parts=[]
    if prev_f and prev_f!=cur_f: parts.append(f'fade({prev_f}, 0.15)')
    if cur_f and cur_f!=prev_f: parts.append(f'show({cur_f}, 0.2)')
    # relabel the state cells + colour the grid to the op accent
    parts.append(f'recolor(st, {acc}, 0.2)')
    w('  par { ' + '; '.join(parts + cells_say(s['after'], 0.25)) + '; }')
    # secondary beat
    if op=='sub' and s.get('detail'):
        looks = "  ".join(f'{x["from"]}→{x["to"]}' for x in s['detail'][:3])
        w(f'  par {{ say(sbox, "S-box:  {looks}", 0.2); show(sbox, 0.2); fade(rk, 0.2); fade(rklab, 0.2); }}')
    elif op=='xor' and s.get('operands') and s['operands'][0].get('values'):
        vals=s['operands'][0]['values']
        w(f'  say(rk.r0c0, "{vals[0][0]}", 0.1); say(rk.r0c1, "{vals[0][1]}", 0.1); say(rk.r0c2, "{vals[0][2]}", 0.1); say(rk.r0c3, "{vals[0][3]}", 0.1);')
        w(f'  say(rk.r1c0, "{vals[1][0]}", 0.1); say(rk.r1c1, "{vals[1][1]}", 0.1); say(rk.r1c2, "{vals[1][2]}", 0.1); say(rk.r1c3, "{vals[1][3]}", 0.1);')
        w(f'  say(rk.r2c0, "{vals[2][0]}", 0.1); say(rk.r2c1, "{vals[2][1]}", 0.1); say(rk.r2c2, "{vals[2][2]}", 0.1); say(rk.r2c3, "{vals[2][3]}", 0.1);')
        w(f'  say(rk.r3c0, "{vals[3][0]}", 0.1); say(rk.r3c1, "{vals[3][1]}", 0.1); say(rk.r3c2, "{vals[3][2]}", 0.1); say(rk.r3c3, "{vals[3][3]}", 0.1);')
        w('  par { show(rk, 0.2); show(rklab, 0.2); fade(sbox, 0.2); }')
    else:
        w('  par { fade(sbox, 0.2); fade(rk, 0.2); fade(rklab, 0.2); }')
    # step info + settle grid back to fg
    w(f'  say(cap, "{title}", 0.2);')
    w(f'  say(exp, "{explain}", 0.2);')
    w('}')
    w('  recolor(st, fg, 0.25);')
    w('wait(0.35);')
    if cur_f: prev_f=cur_f

res=d.get('output',{}).get('ciphertext','')
w('')
w('par { recolor(st, lime, 0.4); fade(sbox, 0.2); fade(rk, 0.2); fade(rklab, 0.2); }')
w('pulse(st, 0.8);')
w(f'text(done, (cx, h - 250), "ciphertext  {res}");  size(done, 22); color(done, lime); hidden(done);')
w('show(done, 0.5);')
w('wait(1.6);')
open(out_path,'w').write("\n".join(L)+"\n")
print(f"wrote {out_path}  ({len(steps)} steps, {len(L)} lines)")

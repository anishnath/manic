# Prototype trace-player: real crypto-viz {panels,steps} JSON -> a complete .manic
# file, using ONLY existing manic vocabulary (matrix/equation/text/say/fade/show).
import json, sys

trace_path, out_path, direction = sys.argv[1], sys.argv[2], sys.argv[3]
d = json.load(open(trace_path))
steps = [s for s in d['steps'] if s.get('target') == 'state' and 'after' in s]

def mat(after): return "; ".join(" ".join(r) for r in after)

# formula per op-type (generic; the caption carries round specifics)
FORMULA = {
    'sub':     r"b' = S(b)",
    'permute': r'\text{row } r \lll r',
    'mix':     r"c' = M\,c \quad (\mathrm{GF}(2^8))",
    'xor':     r'\text{state} \oplus K',
}
OPNAME = {'load':'load','xor':'AddRoundKey','sub':'SubBytes',
          'permute':'ShiftRows','mix':'MixColumns','emit':'result'}

L = []
w = L.append
title = d['algorithm']
w(f'title("AES-128 — {direction}");')
w('canvas("portrait");')
w('template("mono");')
w('')
w(f'text(hdr, (cx, 140), "AES-128 · {direction}");')
w('size(hdr, 44); bold(hdr); color(hdr, cyan);')
w('text(rnd, (cx, 204), "");  size(rnd, 26); color(rnd, dim);')
w('')
# one matrix per state snapshot, stacked at centre, hidden (revealed in sequence)
for i, s in enumerate(steps):
    w(f'matrix(s{i}, "{mat(s["after"])}", (cx, cy), 150, 130);')
    if i: w(f'hidden(s{i});')
w(f'color(s{len(steps)-1}, lime);')
w('')
# one formula equation per op-type, hidden
for op, tex in FORMULA.items():
    w(f'equation(f_{op}, (cx, cy - 430), `{tex}`, 40);  hidden(f_{op});')
w('')
w('text(cap, (cx, cy + 470), "");  size(cap, 28); wrap(cap, w*0.82);')
w('text(mark, (w - 150, h - 44), "Made with Manic");  size(mark, 22); color(mark, dim);')
w('')
w('// --- timeline: replay the real trace, snapshot by snapshot ---')
w('show(hdr, 0.4); show(mark, 0.4); show(s0, 0.5);')
w(f'say(rnd, "{steps[0]["phase"]}", 0.2);')
w(f'say(cap, "{steps[0]["title"]}", 0.3);')
w('wait(0.7);')
w('')
prev_f = None
for i in range(1, len(steps)):
    s = steps[i]
    op = s['op']
    w(f'step("{s["phase"]} · {OPNAME.get(op,op)}") {{')
    w(f'  say(rnd, "{s["phase"]}", 0.2);')
    w(f'  say(cap, "{s["title"]}", 0.25);')
    parts = [f'fade(s{i-1}, 0.3)', f'show(s{i}, 0.3)']
    cur_f = f'f_{op}' if op in FORMULA else None
    if prev_f and prev_f != cur_f: parts.append(f'fade({prev_f}, 0.2)')
    if cur_f and cur_f != prev_f: parts.append(f'show({cur_f}, 0.3)')
    w('  par { ' + '; '.join(parts) + '; }')
    w('}')
    w('wait(0.45);')
    if cur_f: prev_f = cur_f
out = d.get('output', {})
res = out.get('ciphertext') or out.get('plaintext') or ''
w('')
w(f'pulse(s{len(steps)-1}, 0.8);')
w(f'text(done, (cx, cy + 560), "{("ciphertext" if direction=="encrypt" else "plaintext")}  {res}");')
w('size(done, 24); color(done, lime); hidden(done);')
w('show(done, 0.5);')
w('wait(1.4);')

open(out_path, 'w').write("\n".join(L) + "\n")
print(f"wrote {out_path}  ({len(steps)} state steps, {len(L)} lines)")

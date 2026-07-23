# Your Flowchart Should *Run*

### Every diagram tool draws dead boxes-and-arrows you have to drag into place by hand. manic draws a flowchart that lays *itself* out, stays readable at any size, and actually **runs** — a token walking the process and taking the branch.

---

Here's the thing nobody says out loud about flowcharts: you spend more time *fighting the layout* than thinking about the logic.

You open Mermaid or draw.io or Lucidchart. You drop a "start" box. Then a decision. Then two branches, which now overlap, so you drag them apart. Then the loop-back arrow cuts straight across three other boxes, so you re-route it by hand. Forty minutes later you have a diagram — and it's *dead*. A static picture. It shows the boxes, but it can't show the one thing that actually matters: **how execution flows through it.** Which branch gets taken. Where the loop goes. What a real run looks like.

I wanted a flowchart that did three things I could never get at once:

1. **I never position anything by hand.**
2. **It stays readable no matter how big it gets.**
3. **It actually runs** — I can watch a token walk the process and take a branch.

That turned out to be **manic** — a tiny language where you *describe* a diagram in text and it animates. Here's how it does all three.

**▶ Watch it in action: [youtu.be/5iBfClaMyY8](https://youtu.be/5iBfClaMyY8)** — a CI/CD pipeline that builds itself box-by-box, then runs, with two tokens racing the branches.

---

## The whole idea in 30 seconds

A flowchart in manic is just **nodes and connections**. You never type a coordinate.

```manic
flowchart(fc);
node(start, fc, "terminator", "start");
node(rd,    fc, "io",         "read n");
node(check, fc, "decision",   "i <= n?");
node(body,  fc, "process",    "f = f*i");
node(fin,   fc, "terminator", "end");

connect(e1, start, rd);
connect(e2, rd, check);
connect(e3, check, body);   // yes: keep looping
connect(e4, check, fin);    // no: done
connect(e5, body, check);   // loop back
```

Declare the boxes, connect them, done. The engine ranks the nodes by the connections between them and lays the whole thing out — no `x`, no `y`, no dragging. Add a node and everything reflows; the arrows follow.

That's the entire model. Now the good parts.

---

## Beat 1 — Shapes are just words

There's no "diamond tool" or "parallelogram tool." Every box is the same call — `node(id, parent, "kind", "label")` — and the **kind** is the shape:

- `terminator` → a start/end pill
- `process` → a step rectangle
- `decision` → a diamond
- `io` → a parallelogram
- `connector` → a small circle (an off-page hand-off)

```manic
node(check, fc, "decision", "n % i == 0?");
```

One form for every node. The shape *is* the box, with the label centered inside. Nothing to learn beyond the five shape names you already know from every flowchart you've ever seen.

---

## Beat 2 — It lays itself out (and re-lays as you type)

This is the part that deletes the busywork. You don't place boxes; you state relationships, and the layout is a *consequence* of them. A decision with two `connect`s out of it becomes a fork automatically. Add a step in the middle and the rest slides down to make room. Delete one and it closes up.

Because there are no coordinates, the same flowchart just *works* — at any canvas size, in any theme. You describe the logic once; the picture is always correct.

---

## Beat 3 — It stays readable (the part I care about most)

Here's my actual rule: **a flowchart is only useful if you can read the boxes.** A diagram that "fits" by shrinking every label to four unreadable pixels has failed. Fitting is worthless if you can't read it.

So manic's flowcharts are built readability-first:

- **A long flow wraps into side-by-side columns.** Lay a 24-step pipeline top-down and it'd be a thin, unreadable ribbon down the middle of a wide screen. Instead manic wraps it into as many columns as fit the frame — you read *down* one column, then *across* to the top of the next, like a multi-column diagram on paper. Every node stays full-size.
- **Long feedback loops route around the perimeter.** A "roll back → start" arrow doesn't slash diagonally across the whole chart; it drops to the bottom margin and runs a clean rail around the outside, the way you'd draw it by hand.
- **When it's genuinely too big to read, the editor tells you.** Past a node limit it warns you — not to shrink it, but to **split it into linked sub-flows** (end one chart with a `connector` that hands off to the next). Because the honest answer to "this process is huge" is *break it up*, not *make the text tiny*.

Here's a real one — a CI/CD pipeline, 24 nodes, 7 decisions, several loops — declared with **zero coordinates** and no manual layout:

```manic
flowchart(cd);
node(start, cd, "terminator", "start");
node(build, cd, "process", "build");
node(bok,   cd, "decision", "build ok?");
node(unit,  cd, "process", "unit tests");
node(smok,  cd, "decision", "smoke ok?");
node(roll,  cd, "process", "rollback");
// … more …

connect(e5,  bok, unit);      // pass
connect(e6,  bok, fix);       // fail
connect(e23, roll, start);    // loop all the way back
```

It auto-wraps into columns, the `roll → start` loop rides the perimeter, and it's readable at a glance. I never touched a coordinate.

---

## Beat 4 — And then it *runs*

This is the difference between a manic flowchart and a picture of a flowchart.

You reveal the chart, then **`route` a token** from the start terminator. It walks the process; at each decision, the branch *you* authored lights up while the others stay cold; a loop is just an edge back to an earlier node, so the token goes around and comes back.

```manic
request(tok, start, "commit");

route(tok, e1, 0.3, smooth);   // start → build
route(tok, e4, 0.3, smooth);   // build ok?
route(tok, e6, 0.3, smooth);   // → fix (it failed)
route(tok, e7, 0.4, smooth);   // fix → build  (loop back!)
route(tok, e4, 0.3, smooth);   // build ok? — this time, yes
// … on to green …
```

Suddenly the diagram isn't documentation — it's the algorithm *executing*. And because `route` is just "move this token along that edge," you can do things a static tool can't dream of. Want two pull requests racing the same pipeline to different outcomes? Run them **in parallel**:

```manic
step("round-1") {
  par {
    seq { show(a.parts, 0.2); route(a, e1, 0.2, smooth); /* … ships     */ }
    seq { show(b.parts, 0.2); route(b, e1, 0.2, smooth); /* … rolls back */ }
  }
}
```

Two dots ride the pipeline together, split at the smoke test — one goes green to "shipped," the other fails and loops around the perimeter back to start. Then a one-liner clears the lit paths and you run a different pair. The chart doesn't just show the branches; it *takes* them.

---

## Beat 5 — Watch it build itself

For teaching, my favorite trick: let the chart **draw itself box-by-box** before it runs. Each edge draws in, its node appears, in flow order — you watch the whole pipeline take shape, then the token walks it.

```manic
untraced(cd.connections);           // edges start undrawn
step("build-chart") {
  seq {
    show(start, 0.3);
    draw(e1, 0.14); show(checkout, 0.18);
    draw(e2, 0.14); show(install, 0.18);
    draw(e3, 0.14); show(build, 0.18);
    // … each box appears as we build toward it …
  }
}
```

There is no better way to *learn* how a flowchart is put together than to watch one assemble itself, then run.

---

## Why it feels different

- **No coordinates, ever.** You state the logic; the layout is computed. Add, remove, reorder — it re-lays itself and the arrows follow.
- **Readable by construction.** It wraps long flows into columns, rails long loops around the perimeter, and *tells you to split* when a process is truly too big — instead of quietly shrinking it into soup.
- **It runs.** `route` a token and watch execution take a branch; race tokens in parallel; loop back and come around. A picture can't do that.
- **It's honest.** The token follows the graph you actually drew. It can't fake a path that isn't there.
- **It's just text.** Deterministic, diff-able, reviewable. Your flowchart lives in version control next to the code it describes, and the same file renders the same video every time.

The shift is the same one that makes all of manic click: you stop *drawing* the thing and start *describing* it — and the moment you do, the diagram can suddenly move, wrap, split, and run, because it was never a picture in the first place. It was a description all along.

---

## Start here

Nothing to install. Open the playground, declare a few boxes, watch it lay out and run:

**▶ [Watch the flowchart video](https://youtu.be/5iBfClaMyY8)** — build-then-run, in under a minute
**▶ [8gwifi.org/manic](https://8gwifi.org/manic)** — the browser playground
**▶ [8gwifi.org/manic/docs](https://8gwifi.org/manic/docs)** — the full guide + a gallery, each example with the exact script that made it

Copy a flowchart. Add a decision. Route a token through it. That's the loop — and it's the reason I stopped dragging boxes and started describing the logic I actually wanted to see run.

*Made with manic.*

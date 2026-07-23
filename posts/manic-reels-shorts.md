# A Perfect Reel in a Dozen Lines

### Making an educational Short is supposed to be *editing*: cutting clips, keyframing text, nudging things out of the caption-safe zone, then re-exporting for every platform. manic skips all of it — you **describe** the Short, and it renders a branded, format-safe, timed reel from a few lines of text.

---

Here's the part of "content creation" nobody puts in the tutorial: the *idea* takes ten minutes and the *production* takes three hours.

You have a neat concept — say, "Pascal's triangle secretly hides a fractal." Great Short. Now open CapCut or After Effects. Lay out the visual. Keyframe each number in. Add a title. Realize the title is under the platform's UI, nudge it. Add a caption track, time it to the beat. Add your handle, your logo, a "follow" end card. Export at 1080×1920. Then your friend says "put it on the feed too" — so you re-frame everything for 4:5 and 1:1 and re-export. The idea was ten minutes. You just lost your evening.

manic's bet is that a Short is *structured* enough to **describe** instead of edit. You say what happens; the engine handles layout, timing, safe areas, branding, and every aspect ratio. Here's how.

**▶ Try it free: [8gwifi.org/manic](https://8gwifi.org/manic) · Docs + gallery: [8gwifi.org/manic/docs](https://8gwifi.org/manic/docs)**

---

## The whole idea in 30 seconds

A Short in manic is a **canvas**, a **creator profile**, some **content**, and a **script** of beats. No timeline, no keyframes.

```manic
canvas("9:16");
template("shorts");

creator(me, "@you name=Your_Channel tagline=Math_made_visual accent=cyan secondary=magenta cta=Follow_for_more safe=reels");

text(hook, (cx, h*0.14), "watch this triangle hide a fractal");
untraced(hook); cursor(hook);

socials(me);
endcard(me, "title=Follow cta=your.link");

type(hook, 1.4);          // the hook types itself on
// ... your visual builds here ...
show(me.endcard, 0.6);    // the branded end card
```

That one `creator(...)` line is doing a *lot*: it sets your handle, channel name, tagline, accent colors, the social lockup, the end-card CTA, **and** the reels-safe margins so nothing lands under the platform's buttons. You never think about any of it again.

Now the good parts.

---

## Beat 1 — One script, every format

This is the one that saves the evening. You write the Short **once**. The same file renders portrait, feed, square, and landscape:

```
manic reel.manic --canvas portrait   # 9:16 Short / Reel
manic reel.manic --canvas 4:5        # feed
manic reel.manic --canvas square     # 1:1
manic reel.manic --canvas 16:9       # YouTube
```

Because you never wrote coordinates — you wrote *relationships* (`(cx, h*0.14)`, `safe=reels`) — the layout **reflows** for each shape, and the *timing, identity, and story stay identical*. One idea, every screen, no re-editing.

---

## Beat 2 — The content is *true*, not faked

Here's what separates a manic Short from a motion-graphics template: the visual is **computed**, not hand-placed. When a Short says "each number is the sum of the two above it," that's not a designer typing numbers into boxes — manic *calculates* them:

```manic
// every cell of Pascal's triangle is a real binomial coefficient
let val = prod(i in 1..k+1 : (n-k+i)/i);   // C(n,k), computed
counter(cell, (px, py), val, 0);            // shown on screen
```

Color the odd ones and Sierpiński's triangle appears — because it's genuinely `C(n,k) mod 2`, not a drawing of a fractal. The same engine plots real functions, runs real physics, and generates real noise (`fbm(x,y)` sculpts actual fractal terrain). Your audience isn't watching an animation *of* an idea; they're watching the idea itself. That's the credibility a template can't fake.

---

## Beat 3 — Captions that pop, without a caption track

Two of the most-requested Short effects, each one verb:

```manic
type(hook, 1.4);              // typewriter — types on letter by letter (add cursor(hook))

caption(line, "each cell = the two above it", (cx, h*0.8), 33, cyan);
hidden(line.words);
wordpop(line, 0.13);          // TikTok-style word-by-word pop-in
```

No keyframing each word. `type` reveals a line like a typewriter; `wordpop` pops words in one at a time (or `karaoke` highlights them in sequence). You point at the phrase and pick the effect.

---

## Beat 4 — The quiz format, batteries included

"Pause and predict" is the highest-retention Short format there is, and it's a first-class thing in manic. Question, options, a think-timer, the reveal, an explanation — all authored, all timed:

```manic
quiz(q, "Rule 90 from a single dot — what shape appears?", "studio safe=reels accent=cyan");
option(q, "A checkerboard");
option(q, "Sierpinski's triangle", correct);
option(q, "Random static");
explain(q, "XOR of two parents = Pascal mod 2 — the odd cells are Sierpinski.", "Why");

timing(q, "ask=1.2 think=5 reveal=0.8 hold=2.4");   // the beat
timerstyle(q, "look=ring label=PREDICT color=cyan"); // the timer's look — independent

run(q);   // plays ask → think (timer drains) → reveal (correct card + explanation)
```

`timing` controls the *pacing*; `timerstyle` controls the *look* — change one without touching the other. Drop your visual in as the quiz's media (it can even build while the timer runs, so viewers watch and guess), and `run(q)` choreographs the whole beat.

---

## Beat 5 — A storyline, not a clip

The Shorts that work aren't visuals — they're **arcs**. manic makes the arc the structure of the file:

> **Hook** (a question, typed on) → **Build** (the visual assembles) → **Reveal** (the twist) → **The point** (why it matters) → **Follow** (the end card).

Every math Short we've shipped follows it — Pascal's fractal, lattice-path counting, a Rule-90 quiz, a "flat map becomes a 3D world" terrain reveal, "how noise builds worlds" from 1D to fractal. Same five beats, wildly different topics, each one a handful of lines that reads like a script because it *is* one.

---

## Why this is different

Template tools give you *motion*. manic gives you a **language for the idea**:

- **You describe, you don't edit.** No timeline, no keyframes, no dragging.
- **It's format-native.** One script → every aspect ratio, safe areas handled.
- **It's true.** The math/physics/noise on screen is computed, not decorated.
- **Your brand is one line.** Handle, socials, accent, end card, safe zones — set once.
- **It's a script you can diff.** Change a number, re-run, done — no re-export marathon.

The idea took ten minutes. Now the production does too.

**▶ Make your first Short free: [8gwifi.org/manic](https://8gwifi.org/manic)**

---

*Made with manic — the animation language that turns plain text into precise, correct animations. No coding, no timeline scrubbing: you describe what you want and manic animates it — math, physics, algorithms, geometry — with the visual actually TRUE, not hand-drawn.*

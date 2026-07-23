# manic — Business & Application Ideas (parking lot)

> **Engine first.** This is a downstream strategy doc, deliberately kept **out of
> CAPABILITIES.md**. None of it matters until the engine is genuinely right —
> broad, correct, reliable, and reliably generatable. Once the engine is solid,
> everything below gets *much* smoother, because every application here reduces to
> "a different input adapter + audience skin over the same engine." Revisit when the
> engine bar is met; don't let it pull focus before then.

*Captured from a strategy conversation — these are assessments/opinions, not
commitments, and there are no financials behind them (users, CAC, render cost,
pricing, conversion are all unknown).*

---

## The core insight: engine ≠ application

The **engine (manic)** is one business — infrastructure sold to the few thousand
people who make animations. **Applications built on it** are a different, usually
bigger business — sold to the *end users of a problem*. (Cf. the database vs. the
SaaS on it; GPT the model vs. Cursor/Perplexity the apps that became the real
businesses.) You own the engine, so you don't have to choose — but the value
capture and the large market are in the **application layer**.

## The sharper insight: manic's real leverage is as an AI *target language*

The unsolved problem in edtech-AI: **AI can explain, but it can't produce a
*correct* visual.** Sora/Runway hallucinate a wrong diagram; text-only tutors
(Khanmigo) never *show*. manic is the missing piece:

> **An LLM can't reliably make a correct video, but it *can* make correct structured
> DSL** — because manic's output space is bounded (typed kits) and *verifiable*
> (catalog + `check()` + autofix + SYSTEM_PROMPT).

Pipeline: **content → LLM writes manic → `check`/autofix repairs → manic renders it
truthfully → embed.** That's a moat no one else has:
- vs **AI video** → manic is *true*, not hallucinated.
- vs **Manim** → no human coder in the loop; auto-generated.

**Reframe:** "creators author manic by hand" is a tool business. "**LLMs author
manic; humans just consume the animation**" is a platform business — same engine.
(Note: much of the engine-side work already done — SYSTEM_PROMPT hardening, the
catalog, `check`, the autofix loop, the WASM services — *is* the "LLM writes manic
reliably" substrate. The app path is closer than it looks.)

---

## Application menu (grouped by who / what pain)

**Learner / consumer — biggest TAM, hardest distribution**
- **"Animate this" browser extension** — highlight any STEM paragraph/equation on
  any page → inline, correct animated explainer. The universal one.
- **AI tutor that *shows*** — every chat answer ships with a truthful animation.
- **Homework/notes companion (mobile)** — snap a page / paste notes → animated
  explainers.
- **Animated flashcards** — Anki, but each card is a correct micro-animation.

**Creator / prosumer — already ~80% built (fastest to revenue)**
- **AI Reel/Short Studio** — describe a Short → LLM writes the manic → branded,
  format-safe reel. (Creator kit + gallery + autofix + render API already exist.)
- **STEM channel factory** — a topic list → a month of finished Shorts, every
  aspect ratio.

**Educator / institution — highest ACV**
- **Teacher tool** — type a concept → classroom animation / worksheet figure.
- **Textbook-publisher pipeline (B2B2C)** — "animate your catalog"; license per title.
- **LMS/course integration** — Coursera/edX/Khan auto-animate course content.
- **Assessment generator** — animated quiz questions (quiz kit already does
  pause-and-predict).

**Developer / API — clean recurring line**
- **Animation-as-an-API** — programmatic, truthful video from a description (a
  *domain-aware* Remotion; the onecompiler render API is the seed).
- **Docs/diagram automation** — code/architecture/algorithm → animated diagram
  (systems/flowchart/C4 kits) for eng docs, PR explainers, READMEs.

---

## Recommended sequencing (once the engine bar is met)

1. **AI Reel Studio (creator) first.** Pieces already exist; it's the shortest path
   to revenue; **every output is an ad** (watermark + "made with manic" + the exact
   script in the gallery); and it proves "LLM→manic→correct" at *low stakes* (a wrong
   reel is cheap — a wrong tutor answer isn't). Funds everything.
2. **Then the consumer "animate any concept" app** (extension/tutor) — the 100×
   market, unlocked once the reel studio has proven the LLM hit-rate.
3. **Opportunistically, B2B publisher/LMS deals** — one license ≈ 10,000 consumer
   subs; they come to you after viral reels + a working app.
4. **Always-on API** for devs who find you.

**Unifying point:** every app = the *same* engine + the *same* LLM-writes-manic
pipeline, differing only by **input adapter** (highlighted page / chat / topic list
/ PDF / API call) and **audience skin**. Build the correctness-and-render core once;
spin up verticals cheaply.

---

## Honest constraints & risks

- **Coverage = STEM/quantitative, not "any" textbook.** manic's kits are
  math/physics/CS/optics/geometry/diagrams — a calculus page, not a history page. So
  "animate any textbook" is really "animate any **STEM** concept." Still enormous.
- **AI-generation reliability is the new core risk** (the engine risk is mostly
  retired). The bounded DSL + check/autofix make it far more tractable than open
  video, but topic coverage/quality must be proven cold, not cherry-picked.
- **Latency & cost** per animation (LLM + render) → aggressive caching (same concept
  → cached render).
- **Distribution muscle** shifts to consumer/edtech GTM. Great engine ≠ found engine;
  organic reel distribution converting cheaply is the make-or-break variable.
- **"For non-programmers" must be *true*.** If the DSL/app isn't usable by non-coders,
  TAM collapses to Manim's audience (technical, price-resistant, and Manim is free).

## The one thing to validate before betting on the app

**LLM hit-rate on cold input.** Take 50 *random* STEM paragraphs → run
text→manic→`check`→render → score how many come out **correct and clear**. That
single number is the business case: ~70%+ → there's a company; ~30% → more engine
work first. (A harness for this is worth building before any app UI.)

## Profitability read (condensed)

Cost structure is *favorable* for a lean SaaS/API outcome: Rust + WASM, generative,
**no assets** — render is the only real marginal cost, and plans/limits already price
above it. The moat vs AI is durable ("it's TRUE"). Whether it's *profitable* is now a
**go-to-market question, not an engineering one**: (a) does organic distribution
convert cheaply, (b) is it genuinely non-coder-usable, (c) is pricing disciplined
above render cost. Yes to (a)+(b) → comfortably profitable as a lean product; the
margins are there. Venture-scale is less certain and depends entirely on the consumer
app + AI hit-rate.

**Metrics that answer it faster than any plan:** signup→first-render, first-render→
paid, render-cost-per-paid-user, organic-view→signup.

---

*Reminder: engine first. Everything here is smoother — and some of it only possible —
once the engine is broad and reliably generatable. Park it; don't chase it early.*

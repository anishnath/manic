# Machine learning — models made visible

Manic ML begins with a simple promise: the learner should see the values the
model actually computed, one meaningful flow at a time. A dense network is not
useful merely because every node and edge is present. The current layer should
be legible; the surrounding architecture should provide context without
becoming a mesh of noise.

ML1 covers deterministic feed-forward networks and activation functions. ML2
adds supervised loss, exact reverse-mode gradients, and explicit parameter
updates on the same persistent figure. ML3 adds tensors, convolution, pooling,
and a shared operator scan. ML4 adds one exact, focused self-attention head and
a deterministic top-k output view. ML5 makes the earlier text-to-model-input
journey visible through honest token boundaries, embeddings, and position.
ML6 carries those vectors through a complete multi-head transformer block.
ML7 keeps the final language-model projection separate, then makes temperature
and next-token sampling visible without pretending to run a pretrained model.

## The visual language is still ordinary Manic

The ML nouns compute the diagram. Core Manic supplies the cinematography. The
shipped examples use a small visual grammar consistently:

- cyan carries input or data;
- lime carries positive or retained signal;
- magenta carries negative contribution or reverse gradient;
- gold marks the active operation, residual result, or selected prediction;
- dim structure preserves context without letting every edge compete.

Attention outputs and Transformer stages contain real vector summaries or
signed mini bars, so a labelled box is never the only explanation. `forward`
leaves a quiet contribution-weighted trace, `backward` uses external gradient
badges, and `encode` flows through the main path plus both residual bypasses.
These are defaults of the kit; creators do not need to animate internal tags by
hand.

Camera motion is optional. When it helps, keep it small, focus one calculation,
and return to the complete result:

```manic
step("compute") {
  par {
    forward(net, "0.15 0.92 0.38", 4.2, smooth);
    seq {
      par { cam((w*0.24,h*0.47),0.55,smooth); zoom(1.06,0.55,smooth); }
      wait(0.55);
      cam((cx,h*0.47),0.8,smooth);
      wait(0.55);
      cam((w*0.76,h*0.47),0.8,smooth);
      par { cam((cx,cy),0.55,smooth); zoom(1,0.55,smooth); }
    }
  }
}
```

Avoid decorative particles around a network. Use a bounded packet or `flow`
only when it represents a token, activation, attention contribution, gradient,
or sampled choice. The settled frame must remain understandable with motion
paused and with the default mono template.

## Start with the story you need

You do not need to learn the whole ML kit. Choose one row, create the named
object, and use normal Manic `step`, `show`, `say`, and `pulse` calls around it.

| Story | Start with | Animate with |
| --- | --- | --- |
| Scalar → vector → matrix → tensor | `tensor` | core Manic verbs |
| One activation function | `activation` | core Manic verbs |
| Prediction through a dense model | `network` | `forward` |
| Why a prediction is wrong | `network` + `forward` | `loss` |
| How gradients assign credit | prediction + loss | `backward` |
| One visible learning correction | completed backward pass | `update` |
| Image → feature map | `tensor` + `kernel` + `convolve` | `scan` |
| Smaller feature map | tensor + `pool` | `scan` |
| Text → tokens → positioned vectors | `tokenize` + `embedding` | core Manic verbs |
| How one token finds context | `attention` | `attend` |
| Context → candidate probabilities | `attention` + `topk` | core Manic verbs |
| Complete transformer block | `transformer` | `encode` |
| Hidden state → logits → next token | `transformer` + `logits` | `sample` |

The smallest useful pattern is:

```manic
activation(view, (cx,cy), relu, 510, 260);
untraced(view.axes);
untraced(view.curve);

step("meet-the-rule") {
  draw(view.axes, 0.7);
  draw(view.curve, 1.2);
}
```

ML words compute or construct the truthful figure. Core Manic remains the
story language. This division keeps a tensor-only or activation-only lesson
small instead of forcing every creator into a complete neural network.

## The complete ML1–ML7 vocabulary

| Kind | Words | What the creator supplies |
| --- | --- | --- |
| Figures | `tensor`, `kernel`, `activation`, `network`, `tokenize`, `embedding`, `attention`, `transformer` | values, shape, text, boundaries, activation names, embeddings, compact block options, or a deterministic seed |
| Derived views | `convolve`, `pool`, `topk`, `logits` | a source figure plus operator, candidate, token, temperature, or projection choices |
| Computation | `forward`, `loss`, `backward`, `update`, `scan`, `attend`, `encode`, `sample` | input, target, learning rate, selected token/block, decoding strategy, duration, and easing |

`backward` is intentionally not standalone: it requires a `forward` prediction
and `loss`. `scan` similarly requires a `convolve` or `pool` result. Errors name
the missing prerequisite instead of drawing a plausible but false animation.

## Learn tensors without a network

Use a 1×1 grid for one displayed scalar, one row for a vector, rows for a
matrix, and `|` to stack channels. The accompanying title can name the familiar
rank while the same `tensor` noun supplies stable cells and values:

```manic
tensor(scalar, (180,360), "7");
tensor(vector, (460,360), "7 2 -1 4");
tensor(matrix, (760,360), "7 2 -1; 4 0 3; 1 5 6");
tensor(volume, (1060,360),
  "7 2 -1; 4 0 3; 1 5 6 | 2 4 8; 1 3 9; 0 5 6");
```

Every part is selectable: `volume.channel0`, `volume.row1`, `volume.col2`,
`volume.cells`, `volume.values`, or one cell such as `volume.c1.r0c2`.

## Explain one activation without a network

```manic
activation(view, (cx,cy), relu, 620, 320);
```

Standalone plots support `linear`, `relu`, `sigmoid`, and `tanh`. Use
`view.axes`, `view.curve`, and the ordinary line, point, equation, draw, and
pulse tools to test inputs or explain a region. `softmax` belongs inside a
network because it transforms a complete vector rather than one scalar.

## A complete forward pass

```manic
network(net, (cx, cy), "3 6 4 3", "relu tanh softmax", 820, 350, 21);
forward(net, "0.15 0.92 0.38", 4.2, smooth);
```

The two quoted lists serve different purposes:

- `"3 6 4 3"` defines input, hidden, hidden, and output layer sizes.
- `"relu tanh softmax"` defines what happens after each of the three affine
  transitions.
- `21` is the deterministic seed. The same file produces the same weights,
  activations, prediction, and frames every time.
- The input supplied to `forward` must contain exactly three finite values.

Manic uses Xavier-uniform weights for the seeded educational model. It computes
each affine layer, applies the named activation, and uses a numerically stable
softmax. The output bars and percentages are derived from that result.

## One complete learning step

```manic
forward(net, "0.15 0.92 0.38", 3.2, smooth);
loss(net, "1 0 0", crossentropy, 1.5, smooth);
backward(net, 3.2, smooth);
checkpoint(beforeUpdate, net);
update(net, 0.18, 2.3, smooth);
```

These four beats deliberately remain visible:

1. `forward` stores the real activations for the authored input.
2. `loss` compares the output with the target. `crossentropy` requires a
   softmax output and a non-negative target distribution that sums to one.
   `mse` supports other output activations and arbitrary finite targets.
3. `backward` calculates exact reverse-mode gradients for every visible and
   hidden parameter. The pulse travels output → input along the existing edges.
4. `update` applies `parameter -= learning_rate * gradient`, recomputes the
   same input, and replaces the output bars and loss with their new computed
   values.

The optional loss kind defaults to cross-entropy for a softmax output and MSE
otherwise. The default learning rate is `0.15`. A learning rate is not a visual
speed: changing it changes the mathematics, and a large value can truthfully
increase the loss.

## Undo one authored update exactly

Place a checkpoint after the prediction has been compared with its target and
before the parameter update. It takes no timeline time:

```manic
backward(net, 3.2, smooth);
checkpoint(beforeUpdate, net);
update(net, 0.18, 2.3, smooth);
restore(net, beforeUpdate, 2.3, smooth);
```

`checkpoint` saves every weight and bias plus the current layer values,
prediction, target, and loss. `restore` reverses the visible flow and returns
all of them to that exact saved state. It also clears active gradients; call
`backward` again before attempting another update.

This is precise **checkpoint rollback**. It is useful for explaining what one
gradient step changed, comparing before and after, or showing an undo action.
It is not general machine unlearning: restoring a saved state does not prove
that a data point's influence was removed from an otherwise trained model.

## From pixels to feature maps

Rows use `;`, values use spaces or commas, and channels use `|` inside one
quoted grid. This keeps small textbook tensors readable:

```manic
tensor(image, (250, 340), "0 0 1; 0 1 1; 0 0 1", 44, cyan);
kernel(edge, (540, 340), "-1 0 1; -2 0 2; -1 0 1", 44, magenta);

convolve(feature, image, edge, (820, 340), 1, 1, 0, relu, 44);
scan(feature, 4.0, smooth);

pool(compact, feature, (1080, 340), max, 2, 2, 0, 44);
scan(compact, 2.8, smooth);
```

`convolve` computes one output feature map. Its optional values are stride,
zero padding, bias, cellwise activation, and cell size. A multi-channel input
uses one kernel grid per input channel and sums every channel into each output
cell:

```manic
tensor(rgb, (300, 340), "1 2; 3 4 | 10 20; 30 40 | 2 0; 1 3");
kernel(k, (600, 340), "1 | 0.5 | -1");
convolve(feature, rgb, k, (900, 340));
```

For multiple feature detectors, author multiple kernels and outputs. That keeps
each receptive field explainable instead of hiding a filter bank behind one
visually dense call.

`pool` supports `max` and `average`, operating independently on each channel.
The default window is 2 and the default stride equals the window. Padding does
not fabricate candidate values: padded positions are excluded. Max-pool ties
select the first valid cell in row-major order, which makes selection stable
across renders and direct seeking.

## One scanner for convolution and pooling

`scan(output, duration, easing)` coordinates four identities that should never
drift apart:

- the receptive-field frame on the source;
- the kernel/operator focus;
- the truthful arithmetic summary;
- the destination frame and exact revealed value.

When a pooled tensor consumes a convolution result, starting the pooling scan
automatically quiets the completed convolution arithmetic strip. The figures
remain in place, so the learner sees continuity without two competing status
lines. Use normal `step`, caption, `show`, and `pulse` calls for the narrative;
let `scan` own the synchronized numerical choreography.

## Show how words gain position

Start with the sentence, choose honest boundaries, then turn those identities
into vectors:

```manic
tokenize(words, (cx, 150), "the cat chased the cat", word, w*0.70);
embedding(context, words, (cx, 470), "seeded 6 37", sinusoidal,
  w*0.90, h*0.46);
```

The optional token mode is:

- `word` — Unicode letters/numbers form words; punctuation stays separate;
- `character` — each character is a token and whitespace remains visible;
- `authored` — `|` marks every exact boundary for a hand-authored subword
  explanation.

Authored boundaries are not called BPE because Manic has not applied a merge
table. `embedding` accepts either one explicit numeric row per token (rows
separated with `;`) or `"seeded DIM [SEED]"`. Seeded values are reproducible
educational lookup vectors, not pretrained weights. Repeated copies of the
same token reuse the same base vector; adding exact sinusoidal position makes
their final model inputs different. Use `none` instead of `sinusoidal` when the
lesson should stop at the lookup table.

Reveal `context.vectors`, then `context.positions`, then `context.combined`.
For comparison, pulse stable rows such as `context.row1`; use `.dimN` to focus
one feature across the table. Manic caps the story at 12 tokens and eight
dimensions so the values remain teachable on a phone.

## Explain one transformer attention head

Give `attention` a short token list and one embedding row per token. Then focus
one token with its 1-based position:

```manic
attention(head, (cx, 360),
  "Art | ificial | intelligence | transforms | business",
  "1 0.2 -0.4 0.7; 0.8 0.1 -0.3 0.6; -0.2 1 0.5 0.3; 0.1 0.6 0.9 -0.2; 0.7 -0.1 0.4 1",
  980, 420, 23);

attend(head, 3, 5.2, smooth);
```

This computes one seeded Q/K/V projection, the scaled score matrix
`QK^T / sqrt(d)`, a stable row-wise softmax, and each exact weighted value mix.
`attend(head, 3, ...)` highlights the query for `intelligence`; it does not
rebuild or replace the surrounding tokens.

Add a small output ranking only when the story needs it:

```manic
topk(next, head, 3, (1540, 400),
  "business | work | world | industry | future | people",
  4, 420, 260, 29);
```

`topk` adds the selected embedding to its attention output, applies a seeded
educational output projection, and shows probabilities from the full softmax.
Those percentages are mathematically exact for the authored figure, but they
are not predictions from a pretrained language model. Keep the candidate list
small enough to read; the visual shows at most eight selected results.

## Walk through a complete transformer block

Pass the ML5 embedding directly into `transformer`; do not copy its rows:

```manic
transformer(block, context, (cx, 500),
  "heads=2 mask=causal mlp=12 activation=gelu norm=pre dropout=0 mode=inference seed=41",
  w*0.92, h*0.62);
encode(block, 6.2, smooth);
```

The one configuration sentence controls the choices that change the actual
calculation:

- `heads=1..4`; `d_model` must divide exactly across them;
- `mask=none|causal`; causal future cells receive zero probability;
- `mlp=WIDTH` up to 32 and `activation=gelu|relu|silu|tanh`;
- `norm=pre|post`, which changes where both layer normalizations happen;
- `dropout=0..less-than-1`, `mode=inference|training`, and a reproducible
  `seed`.

Each head computes scaled Q/K scores, applies its mask, normalizes with stable
softmax, and mixes V. Manic concatenates the heads, applies the output
projection, follows the first residual/norm stage, expands and contracts the
MLP, then follows the second residual/norm stage. Training dropout is a real
seeded boolean mask with inverted scaling; inference disables it completely.
`encode` reveals this existing computation and remains safe under direct seek.

## Turn a hidden state into one next token

The transformer's MLP produces another hidden representation. It does not
directly produce vocabulary probabilities. `logits` makes the separate
language-model head explicit:

```manic
logits(next, block, 5, (cx, 520),
  "reason | predict | learn | adapt | explain | .",
  0.8, 760, 440, 73);
sample(next, "top-p 0.90 seed=17", 3.8, smooth);
```

The third argument is a 1-based transformer token. Candidate labels are
`|`-separated and intentionally authored; Manic supports 2–12 in one readable
view. The optional temperature defaults to 1, followed by width, height, and
projection seed.

`logits` computes `W_lm h + b` from that final hidden row. It then divides every
logit by the positive temperature and applies one numerically stable softmax to
the complete candidate list. Reuse the same projection seed for a fair
temperature comparison: logits remain identical while every probability is
recomputed. Lower temperature sharpens the distribution; higher temperature
spreads it.

`sample` keeps four common choices behind one word:

| Strategy string | Exact behavior |
| --- | --- |
| `"greedy"` | selects the highest probability; the decoding distribution is one-hot |
| `"categorical seed=17"` | samples from the complete temperature-scaled distribution |
| `"top-k 3 seed=17"` | keeps exactly the three highest candidates, zeros the rest, then renormalizes |
| `"top-p 0.90 seed=17"` | keeps the smallest descending prefix reaching 90% mass, zeros the rest, then renormalizes |

An excluded candidate has exact probability zero and cannot be sampled. The
same seed and same distribution choose the same result. All displayed values
come from the deterministic educational projection declared in the scene; they
are not predictions from a pretrained language model.

## Introduce an activation first

```manic
activation(reluView, (cx, cy), relu, 510, 260);
untraced(reluView.axes);
untraced(reluView.curve);

par {
  draw(reluView.axes, 0.7);
  draw(reluView.curve, 1.2);
}
```

`activation` supports `linear`, `relu`, `sigmoid`, and `tanh`. A standalone
softmax curve would be misleading because softmax depends on all entries in a
vector; show it as the output activation of a `network` instead.

## Design details that make the result readable

- Inactive edges remain quiet. During `forward`, contribution magnitude drives
  emphasis and one pulse travels in the direction of computation.
- Weight sign and magnitude affect edge styling, but labels, brightness, and
  width keep the structure understandable under the monochrome template.
- Large numerical layers show their first and last units around an ellipsis.
  Computation still uses every unit; only the drawing uses level of detail.
- Input, hidden, and output nodes retain stable ids throughout the story. Manic
  updates values instead of clearing and rebuilding the network.
- Output bars grow from zero to the computed value. A softmax output is labelled
  as a percentage and the status strip names the selected class.
- `loss` places the target beside each output without replacing the prediction.
  Error magnitude focuses attention on the outputs that disagree.
- `backward` temporarily recolours connections by gradient sign and weights
  their emphasis by gradient magnitude. The settled network remains the same
  object, ready for the update.
- `update` shows the gradient direction first, then restores edge styling from
  the new weights and recomputes every node. Its final status preserves the old
  and new loss so the claimed learning outcome is inspectable.
- `restore` sends one readable reverse pulse through the same graph, settles
  edge styling from the saved weights, and restores the saved output bars and
  loss without rebuilding the network.

## Compose it like any other Manic scene

Every part is an ordinary entity carrying useful tags:

| Tag | Selects |
| --- | --- |
| `net` | the complete network figure |
| `net.nodes`, `net.edges`, `net.values` | one visual role |
| `net.layer0`, `net.layer1`, ... | one layer |
| `net.input`, `net.hidden`, `net.output` | semantic layer groups |
| `net.probabilities` | output bars and readouts |
| `net.loss` | supervised target readouts |
| `image.cells`, `image.values`, `image.labels` | tensor visual roles |
| `image.channel0`, `image.row0`, `image.col0` | tensor slices |
| `image.c0.r0c0` and `.value` | one cell and its numeric text |
| `feature.scan` | receptive-field/operator/destination overlays |
| `words.source`, `.tokens`, `.indices`, `.tokenN` | tokenization stages |
| `context.vectors`, `.positions`, `.combined` | embedding addition stages |
| `context.rowN`, `.dimN`, `.operators` | one token, feature, or operator |
| `block.heads`, `.headN`, `.mask`, `.matrix` | multi-head attention and masks |
| `block.concat`, `.projection`, `.residual1`, `.norm1` | first half of the block |
| `block.mlp`, `.activation`, `.dropout`, `.residual2`, `.norm2` | second half |
| `block.input`, `.output`, `.tokenN`, `.rowN` | persistent token lanes |
| `head.tokens`, `.q`, `.k`, `.v`, `.matrix` | attention stages |
| `head.connections`, `.outputs`, `.residual` | attention flow and residual lanes |
| `next.labels`, `.bars`, `.probabilities` | top-k output roles |

That means normal verbs still apply:

```manic
hidden(net);

step("meet-the-model") {
  show(net, 0.6);
}

step("compute") {
  forward(net, "0.15 0.92 0.38", 4.2, smooth);
}

step("decision") {
  pulse(net.output, 0.7);
}
```

Use named steps for question, intuition, computation, and takeaway. Let
`forward` own the dense numerical choreography; use captions and equations to
explain why the active operation matters.

For a learning story, keep the causal order clear:

```manic
step("predict") { forward(net, "0.15 0.92 0.38", 3.2); }
step("compare") { loss(net, "1 0 0", crossentropy, 1.5); }
step("credit")  { backward(net, 3.2); }
checkpoint(beforeUpdate, net);
step("learn")   { update(net, 0.18, 2.3); }
step("unlearn") { restore(net, beforeUpdate, 2.3); }
```

Calling `forward` with a new input starts a fresh learning beat and clears the
old target/gradient state. After `update`, another `backward` may compute fresh
gradients for the updated parameters and the same retained target. Every
`update` still requires a preceding `backward`; there is no invisible optimizer
loop. Name a `restore` step “rollback” or explain its exact boundary if you use
the playful label “unlearn”.

## Current boundary

The ML kit is intentionally for small educational models. It does not load
arbitrary PyTorch or TensorFlow programs, run hidden training loops, expose an
optimizer catalogue, train large models, or require a GPU. Explicit authored
weights, automatic filter banks, convolutional back-propagation, stacks of
multiple transformer blocks, model imports, and packaged pretrained tokenizers
remain planned work. ML4 deliberately computes one
educational attention head, not a hidden pretrained language model. ML5 offers
deterministic word/character splitting and exact authored boundaries; it does
not claim a heuristic is BPE. Token sequences accept at most 12 tokens and
eight embedding values each. Attention accepts 2–8 tokens with at most eight
embedding values each; its candidate vocabulary is capped at 16.
ML6 accepts 1–4 heads, requires exact division of the model dimension, caps the
MLP at 32 values, and computes one educational block rather than an imported
language model. ML7 accepts 2–12 authored candidates and a positive finite
temperature; its LM projection and sampling seed are educational and explicit.
Tensor axes are capped at 16 cells and a tensor at 2,048 values so unreadable
stories fail early instead of silently becoming visual noise.

## Review the shipped stories

The foundation story proves that `tensor` plus ordinary Manic is enough to
explain rank and dimensional growth:

```manic
{{#include ../../examples/manic-ml-scalar-to-tensor.manic}}
```

<div class="manic-video" data-video="ex-manic-ml-scalar-to-tensor"></div>

The activation story uses one real ReLU plot and core Manic probes to explain
negative and positive inputs:

```manic
{{#include ../../examples/manic-ml-activation-focus.manic}}
```

<div class="manic-video" data-video="ex-manic-ml-activation-focus"></div>

The gallery example combines activation, network, forward computation, named
story stages, Creator branding, and a professional restrained layout:

```manic
{{#include ../../examples/manic-ml-forward-pass.manic}}
```

<div class="manic-video" data-video="ex-manic-ml-forward-pass"></div>

The ML2 story keeps one network on screen through prediction, target, loss,
backward credit assignment, and a visibly recomputed gradient update:

```manic
{{#include ../../examples/manic-ml-learning-step.manic}}
```

<div class="manic-video" data-video="ex-manic-ml-learning-step"></div>

The ML3 story turns a small image into an edge-response feature map and then a
pooled summary, with the same scanner serving both operators:

```manic
{{#include ../../examples/manic-ml-cnn-edge-story.manic}}
```

<div class="manic-video" data-video="ex-manic-ml-cnn-edge-story"></div>

The ML5 story begins with a repeated word, proves that its base lookup vector
is reused, and then shows how sinusoidal position makes each occurrence unique:

```manic
{{#include ../../examples/manic-ml-token-embedding.manic}}
```

<div class="manic-video" data-video="ex-manic-ml-token-embedding"></div>

The ML4 story keeps the token lanes visible while one query reveals Q/K/V,
one truthful softmax row, its weighted value mix, the residual, and a small
candidate ranking:

```manic
{{#include ../../examples/manic-ml-transformer-attention.manic}}
```

<div class="manic-video" data-video="ex-manic-ml-transformer-attention"></div>

The ML6 story preserves one token lane through two causal heads, concatenation,
both residual/norm stages, a GELU MLP, and the exact settled block output:

```manic
{{#include ../../examples/manic-ml-transformer-block.manic}}
```

<div class="manic-video" data-video="ex-manic-ml-transformer-block"></div>

The ML7 story compares the same projection at cool and warm temperatures, then
shows greedy and top-p decoding as exact filtered distributions and one seeded
next-token choice:

```manic
{{#include ../../examples/manic-ml-logits-sampling.manic}}
```

<div class="manic-video" data-video="ex-manic-ml-logits-sampling"></div>

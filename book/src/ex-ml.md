# Machine learning

Small, deterministic models whose displayed values are computed rather than staged. The ML examples use progressive focus so forward values, supervised loss, reverse gradients, and parameter updates stay readable on one persistent network.

Each block is the whole file — copy it into `x.manic` and run `manic x.manic` (live) or `--record out` (video).

## manic-ml-scalar-to-tensor

Start with one value, extend it into a vector, arrange values into a matrix, then stack channels into a rank-3 tensor. The only ML noun is `tensor`; ordinary Manic steps, arrows, reveals, and captions tell the complete dimensional story.

```manic
{{#include ../../examples/manic-ml-scalar-to-tensor.manic}}
```

<div class="manic-video" data-video="ex-manic-ml-scalar-to-tensor"></div>

## manic-ml-activation-focus

A focused ReLU lesson: draw the truthful activation curve, test one negative and one positive input, then connect the bend to nonlinearity. `activation` supplies the mathematics while core Manic owns the probes, guides, equation, and pacing.

```manic
{{#include ../../examples/manic-ml-activation-focus.manic}}
```

<div class="manic-video" data-video="ex-manic-ml-activation-focus"></div>

## manic-ml-forward-pass

A creator-first neural-network story: introduce the ReLU activation, reveal a seeded 3→6→4→3 model, then follow one real forward pass into softmax probabilities. `network`, `activation`, and `forward` provide the truthful structure while ordinary named steps, captions, and Creator branding tell the lesson.

```manic
{{#include ../../examples/manic-ml-forward-pass.manic}}
```

<div class="manic-video" data-video="ex-manic-ml-forward-pass"></div>

## manic-ml-learning-step

One complete supervised learning beat on a persistent network: predict, compare with a target using cross-entropy, send exact reverse-mode gradients backward, update every weight and bias, then recompute the same input. `loss`, `backward`, and `update` expose the mathematics without turning the DSL into a training framework.

```manic
{{#include ../../examples/manic-ml-learning-step.manic}}
```

<div class="manic-video" data-video="ex-manic-ml-learning-step"></div>

## manic-ml-cnn-edge-story

A tiny image becomes an edge-response feature map and then a pooled summary. The shared `scan` choreography keeps each receptive field, kernel/operator, arithmetic line, and destination cell synchronized while `tensor`, `kernel`, `convolve`, and `pool` supply the exact values.

```manic
{{#include ../../examples/manic-ml-cnn-edge-story.manic}}
```

<div class="manic-video" data-video="ex-manic-ml-cnn-edge-story"></div>

## manic-ml-token-embedding

A sentence becomes honest word tokens, stable seeded educational lookup vectors, exact sinusoidal positions, and model-input vectors. Repeated words prove that token identity keeps one base embedding while position distinguishes each occurrence.

```manic
{{#include ../../examples/manic-ml-token-embedding.manic}}
```

<div class="manic-video" data-video="ex-manic-ml-token-embedding"></div>

## manic-ml-transformer-attention

One token finds context through a real scaled dot-product self-attention head. Explicit embeddings become Q/K/V, one selected softmax row drives the weighted value mix and residual, and a deterministic educational output projection produces exact top-k probabilities without pretending to be a pretrained language model.

```manic
{{#include ../../examples/manic-ml-transformer-attention.manic}}
```

<div class="manic-video" data-video="ex-manic-ml-transformer-attention"></div>

## manic-ml-transformer-block

One persistent token lane crosses a complete deterministic transformer block: two causal attention heads, concatenation and output projection, both residual/norm stages, a GELU MLP, and exact settled outputs. `transformer` owns the computation while `encode` provides a smooth directly seekable explanation.

```manic
{{#include ../../examples/manic-ml-transformer-block.manic}}
```

<div class="manic-video" data-video="ex-manic-ml-transformer-block"></div>

## manic-ml-logits-sampling

The same educational LM projection is viewed at low and high temperature before greedy and top-p decoding make their support explicit. `logits` computes every candidate from the final hidden row; `sample` filters, renormalizes, and selects one reproducible next token without pretending to run a pretrained model.

```manic
{{#include ../../examples/manic-ml-logits-sampling.manic}}
```

<div class="manic-video" data-video="ex-manic-ml-logits-sampling"></div>

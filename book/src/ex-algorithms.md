# Algorithms & data structures

Each block is the whole file — copy it into `x.manic` and run `manic x.manic` (live) or `--record out` (video).

## bubble_sort

Real sliding swaps; `array` + `compare` + `swap`.

```manic
{{#include ../../examples/bubble_sort.manic}}
```

<div class="manic-video" data-video="ex-bubble_sort"></div>

## two_pointer

`lo`/`hi` index carets scanning inward on a sorted array.

```manic
{{#include ../../examples/two_pointer.manic}}
```

<div class="manic-video" data-video="ex-two_pointer"></div>

## stack_queue

LIFO stack + FIFO queue, with action-point carets.

```manic
{{#include ../../examples/stack_queue.manic}}
```

<div class="manic-video" data-video="ex-stack_queue"></div>

## linked_list

Singly / doubly / circular — classic node anatomy + pointer re-threading.

```manic
{{#include ../../examples/linked_list.manic}}
```

<div class="manic-video" data-video="ex-linked_list"></div>

## textbook-doubly-list

The paper/`support` TEXTBOOK treatment on the algo kit — proof it's domain-agnostic:
`template("paper")` + a hatched `support` rule turn the doubly linked list into a flat
exam-paper figure (the theme's palette remap inks it automatically), then it inserts
and removes. Same styling that dressed the physics sims, on a data structure.

```manic
{{#include ../../examples/textbook-doubly-list.manic}}
```

<div class="manic-video" data-video="ex-textbook-doubly-list"></div>

## hashmap

Hash a key to a bucket; collisions chain on (separate chaining).

```manic
{{#include ../../examples/hashmap.manic}}
```

<div class="manic-video" data-video="ex-hashmap"></div>

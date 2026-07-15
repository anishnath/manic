# Statistics & probability

Each block is the whole file — copy it into `x.manic` and run `manic x.manic` (live) or `--record out` (video).

## statistics — the whole story in three ideas

A guided lesson, not a feature demo: describe a dataset, meet the normal curve,
then see *why* the bell is everywhere (the Central Limit Theorem). The stats
companion to the linear-algebra lesson. Start here.

```manic
{{#include ../../examples/statistics.manic}}
```

<div class="manic-video" data-video="ex-statistics"></div>

## histogram

The shape of a dataset: a list of numbers binned into bars that stagger in one
at a time, with the mean marked and the range labelled (`histogram`). Paste your
own numbers into the data string — grades, prices, heights, times.

```manic
{{#include ../../examples/histogram.manic}}
```

<div class="manic-video" data-video="ex-histogram"></div>

## summary

Describe a dataset in one call: the numbers as dots on a number line, with the
mean, median and mode marked, a ±1σ spread band, and readouts of the range,
variance and standard deviation (`summary`). Central tendency and dispersion,
together.

```manic
{{#include ../../examples/summary.manic}}
```

<div class="manic-video" data-video="ex-summary"></div>

## boxplot

The five-number summary as a box-and-whisker: the box spans Q1→Q3 (its width is
the interquartile range), a line marks the median, the whiskers reach the rest,
and a value far outside is flagged as an outlier (`boxplot`).

```manic
{{#include ../../examples/boxplot.manic}}
```

<div class="manic-video" data-video="ex-boxplot"></div>

## skew

Which way does the tail point? A histogram with the mean and median marked and a
labelled skewness — when the mean is dragged right of the median, the data is
right-skewed (`skew`).

```manic
{{#include ../../examples/skew.manic}}
```

<div class="manic-video" data-video="ex-skew"></div>

## bellcurve

The normal (Gaussian) bell curve and the 68-95-99.7 rule: the bell draws in,
then the ±1σ / ±2σ / ±3σ bands shade one at a time, showing that 68% of values
fall within one standard deviation, 95% within two, and 99.7% within three
(`bellcurve`, alias `gaussian`).

```manic
{{#include ../../examples/bellcurve.manic}}
```

<div class="manic-video" data-video="ex-bellcurve"></div>

## clt

The Central Limit Theorem — the flagship: however flat a single die is, the
*average* of five dice, taken 1200 times, piles into a bell that hugs the normal
curve (`clt`). Seeded, so it renders the same every time.

```manic
{{#include ../../examples/clt.manic}}
```

<div class="manic-video" data-video="ex-clt"></div>

## correlation

Do two things move together? The scatter of paired data, the best-fit line, and
the Pearson correlation `r` — near +1 a tight upward line, near −1 downward, near
0 a shapeless blob (`correlation`).

```manic
{{#include ../../examples/correlation.manic}}
```

<div class="manic-video" data-video="ex-correlation"></div>

## lln

The Law of Large Numbers: flip a fair coin over and over and track the running
proportion of heads. It swings wildly at first, then settles onto the true 0.5
as the trials pile up (`lln`). Draw the curve in to watch it converge.

```manic
{{#include ../../examples/lln.manic}}
```

<div class="manic-video" data-video="ex-lln"></div>

## hypothesis

Is a result surprising enough to be real? Under the null hypothesis the test
statistic follows the standard normal; the observed z cuts off tails whose area
is the p-value. Smaller than α, reject (`hypothesis`).

```manic
{{#include ../../examples/hypothesis.manic}}
```

<div class="manic-video" data-video="ex-hypothesis"></div>

## covariance

Covariance as signed area: a cross at the means, and a rectangle from each point
to the centre — cyan where x and y agree, magenta where they disagree. Their
balance is the covariance (`covariance`).

```manic
{{#include ../../examples/covariance.manic}}
```

<div class="manic-video" data-video="ex-covariance"></div>

## bayes

Bayesian updating: a prior belief about a coin's bias, the likelihood from the
data, and the posterior that combines them — pulled toward the evidence and
sharpening as it accumulates (`bayes`).

```manic
{{#include ../../examples/bayes.manic}}
```

<div class="manic-video" data-video="ex-bayes"></div>

## probability

A probability & sampling playground in four chapters: named distributions
(uniform / exponential / binomial / Poisson), a confidence interval, a
Monte-Carlo estimate of π, and a random walk (`distribution`, `confidence`,
`montecarlo`, `randomwalk`).

```manic
{{#include ../../examples/probability.manic}}
```

<div class="manic-video" data-video="ex-probability"></div>

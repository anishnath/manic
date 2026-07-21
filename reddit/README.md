# manic → Reddit

`manic_reddit.py` creates native Reddit video posts from the same publishing
data used by YouTube and the mdBook:

- title and video key: `../book/videos.txt`
- MP4: `../book/videos-out/<name>.mp4`
- description and exact source link: `../scripts/gen-gallery.py`

It defaults to `r/maniclang`, includes the Markdown description with the native
video post, prompts before any write, and records successful submissions in
`post_state.json` so the same example is not accidentally posted twice.

## Setup

Create a Reddit **script application**, then install PRAW:

```sh
python3 -m venv reddit/venv
source reddit/venv/bin/activate
pip install -r reddit/requirements.txt
```

Keep credentials outside the repository:

```sh
export REDDIT_CLIENT_ID="..."
export REDDIT_CLIENT_SECRET="..."
export REDDIT_USERNAME="anish2good"
export REDDIT_PASSWORD="..."
export REDDIT_USER_AGENT="manic-reddit-publisher/1.0 by u/anish2good"
```

Alternatively, create an ignored `reddit/praw.ini` profile and pass
`--profile manic`. Never commit the password, client secret, or `praw.ini`.

## Catalog posts

Always preview first:

```sh
python reddit/manic_reddit.py --only textbook-watermelon-sections --dry-run
python reddit/manic_reddit.py --only textbook-watermelon-sections
```

`--only` accepts names with or without `ex-`. The generated body is deliberately
Reddit-native: a direct explanation, a link to the complete copyable source, and
the browser playground. Override any part for a particular community:

```sh
python reddit/manic_reddit.py \
  --subreddit math \
  --only textbook-watermelon-sections \
  --title "What two perpendicular cuts do to a sphere" \
  --description-file reddit/post.md \
  --dry-run
```

Remove `--dry-run` only after reviewing the target community's rules. The tool
asks for confirmation; add `--yes` only in a controlled automation. `--force`
is required to create another post for a catalog item already recorded in that
subreddit.

## One-off posts

Catalog data is optional:

```sh
python reddit/manic_reddit.py \
  --video /path/to/clip.mp4 \
  --title "A title written for this community" \
  --description-file /path/to/post.md \
  --dry-run
```

Optional flags include `--thumbnail`, `--flair-id`, `--flair-text`, `--spoiler`,
`--nsfw`, `--videogif`, and `--no-replies`. Native video availability and flair
requirements are controlled by each subreddit.

# Bundled assets — portable files without path guessing

Manic scenes can use small, documented production assets through a stable
`asset:` URI. The URI works from the desktop CLI, a different working
directory, the Docker image, or the production backend. No render flag is
needed.

```manic
image(mark, (cx, 260), "asset:manic-logo.png", 180, 180);
model3(beacon, "asset:models/manic-pyramid.obj", (0,0,0), 1);
```

The same image URI can be used in a Creator profile:

```manic
creator(me, "@anish2good name=Manic logo=asset:manic-logo.png footer=signature");
socials(me);
```

## Available public assets

| Stable URI | Kind | Useful for |
|---|---|---|
| `asset:manic-logo.png` | PNG | `image(...)`, Creator `logo=`, a Manic-branded example |
| `asset:models/manic-pyramid.obj` | Geometry-only OBJ | `model3(...)`, a beacon, monument, marker, or placeholder model |

The catalog stays deliberately small. Do not invent an asset name that is not
listed here. Manic reports a clear error if a bundled URI is missing, and it
rejects `..` traversal. OBJ files also retain the normal file-size and geometry
safety limits.

## Your own images and models

Ordinary paths still work:

```manic
image(photo, (cx,cy), "uploads/my-photo.jpg", 720, 480);
model3(product, "uploads/my-product.obj", (0,0,0), 1);
```

The caller must make those files available to the renderer. This is the right
choice for uploads and private brand assets; `asset:` is for the small catalog
that ships with Manic.

## Adding a new bundled asset to Manic

1. Put it under `assets/` in a typed folder such as `models/`, with a lowercase,
   descriptive filename.
2. Add its stable URI to this page and `assets/README.md`.
3. Add or update a checked `.manic` example using the URI.
4. Keep models geometry-only; do not add scripts, arbitrary shaders, or remote
   dependencies.
5. Run the tests and mdBook build.

The release machinery copies the entire directory. Docker installs it at
`/usr/local/share/manic/assets`; Linux builds produce
`dist/manic-assets.tar.gz`; the EC2 deploy installs that archive; and the
playground sync mirrors the same catalog. Future entries therefore need no
per-file pipeline edit. A custom deployment may point `MANIC_ASSETS_DIR` at a
different catalog root.

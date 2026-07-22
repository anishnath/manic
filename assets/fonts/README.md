# Manic bundled fonts

Manic embeds every production font into the executable. Rendering must never
depend on fonts installed on the host machine.

| Order | File | Role | Immutable provenance | SHA-256 | License |
|---:|---|---|---|---|---|
| 1 | `IBMPlexMono-Regular.ttf` | Primary captions, labels, and data | IBM/plex, embedded font version 2.003 | `6a3412f058c7d8dfd9170c41e85ade48e5156ecb89356110ca57a0a27734af46` | SIL OFL 1.1 |
| 1 | `IBMPlexMono-Bold.ttf` | Primary headlines and emphasis | IBM/plex, embedded font version 2.003 | `ac27abd6450a64dd94467580a02fe6235156d5b92f2926ebbc8e7489df64e0be` | SIL OFL 1.1 |
| 2 | `NotoSansMath-Regular.ttf` | Mathematical relations, operators, and arrows | notofonts/noto-fonts `ffebf8c1ee449e544955a7e813c54f9b73848eac` | `ff5e5e7638e05bf7bc159d8801a28a40eddf76c155bec4fee53150babd795e1a` | SIL OFL 1.1 |
| 3 | `NotoSans-Regular.ttf` | Extended Latin and modifier-letter fallback | notofonts/noto-fonts `ffebf8c1ee449e544955a7e813c54f9b73848eac` | `b85c38ecea8a7cfb39c24e395a4007474fa5a4fc864f6ee33309eb4948d232d5` | SIL OFL 1.1 |
| 4 | `NotoSansSymbols-Regular.ttf` | Directional and technical symbols | notofonts/noto-fonts `ffebf8c1ee449e544955a7e813c54f9b73848eac` | `8f02f31959bbdf6061547a188248e13f84dc5fdd940326ec494675f453f072bb` | SIL OFL 1.1 |
| 5 | `NotoSansSymbols2-Regular.ttf` | Geometric marks, dingbats, and pictographs | notofonts/noto-fonts `ffebf8c1ee449e544955a7e813c54f9b73848eac` | `630846d528dbe4c4981370a4d0a9475a1fd1491a129bb411f8e157cdb5de13c6` | SIL OFL 1.1 |
| 6 | `NotoSansArabic-Regular.ttf` | Arabic joining, marks, and right-to-left lessons | notofonts/noto-fonts `ffebf8c1ee449e544955a7e813c54f9b73848eac` | `ceea25b464a656dc3b26849bab9356740401af62aedf1bfa8b7f0d9b75925b1b` | SIL OFL 1.1 |
| 7 | `NotoSansDevanagari-Regular.ttf` | Devanagari conjuncts, marks, and Indic lessons | notofonts/noto-fonts `ffebf8c1ee449e544955a7e813c54f9b73848eac` | `385e78e6359a9d88a0f243d53b1209d7548361ba2194e2b9ec779bcaa7e8949d` | SIL OFL 1.1 |

The numeric order is the deterministic fallback order after the requested IBM
Plex weight. Fallback is an internal renderer concern. A `.manic` author selects semantic
text styling (`mono`, `bold`, or `display`); the engine selects the first
bundled face selected by the advanced shaper for each complete extended
grapheme cluster. Arabic and Devanagari use script-specific fallback before the
common chain. Add future script faces to this manifest and the text engine
rather than introducing font-specific DSL vocabulary.

See the repository-level `LICENSE-FONTS` for copyright notices and the full
SIL Open Font License text.

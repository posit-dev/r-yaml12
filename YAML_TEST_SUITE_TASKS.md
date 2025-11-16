Outstanding work to pass the full YAML test suite
=================================================

- Tabs and indentation rules: reject/allow tabs according to YAML 1.2 (tab-indented flow/mappings/headers and inline tabs) for cases like 4EJS, 6CA3, K54U, KH5V, Y79Y (tab variants), DK95, Q5MG, UV7Q, 26DV.
- Block scalars: implement 1.2/1.3 chomping, indentation indicators, and folding/strip/keep semantics (with comments/empty lines) to cover 4Q9F, 4QFQ, 5GBF, 6BCT, 6FWR, 6WPF, 753E, 93WF, B3HG, DWX9, EX5H, F8F9, G992, H2RW, JEF9, K527, K858, L24T, M29M, M9B4, MJS9, MYW6, R4YG, T26H, T5N4, TS54, XV9V.
- Quoted flow folding and line-break rules: handle 1.2 and 1.3 double/single-quoted folding for 7A4E, 9TFX, 9YRD, NP9H, PRH3, Q8AD, T4YY, TL85.
- Trailing/leading whitespace and comment handling in flow/blocks: fix 4RWC, 6HB6, DC7X, DE56 (2–6), NB6Z, P94K.
- Tags and empty scalars: support verbatim/unknown/non-specific tags and tagging empty scalars (7FWL, S4JQ, FH7J).
- Anchors/alias edge cases: anchor on empty node and alias with surrounding whitespace (6KGN, 26DV).
- Directives/version parsing: handle header variants (DK95#8, MUS6#4, ZYU8#1–3).
- Explicit-key and mixed implicit/explicit mappings: A2M4, RR7F.
- Minimal/empty streams and empty keys: SM9W, NHX8.
- Flow collection edge cases and leading-space flows: 3RLN (2–3,5–6), 4ZYM, 6CA3, TL85, Y79Y#3, Y79Y#2.
- Comparison-only mismatches (parses but JSON normalization differs): 3RLN#3/#6, 4ZYM#1, 6CA3#1, 6KGN#1, 6WPF#1, 7A4E#1, 7FWL#1, 96NN#1, 9TFX#1, 9YRD#1, A2M4#1, DE56#2/#5/#6, DK95#1/#3/#6/#9, DWX9#1, EX5H#1, HS5T#1, K54U#1, KH5V#3, NAT4#1, NB6Z#1, NP9H#1, PRH3#1, Q5MG#1, Q8AD#1, RR7F#1, S4JQ#1, SM9W#1, T26H#1, T4YY#1, TL85#1, UV7Q#1, Y79Y#2/#11.

# Live validation for `#:schema` Nix configs

`~/.config/btc_line.nix` carries a `#:schema btc_line.module.nix` line. That
gives it completion, hover and go-to-definition from the generated module, and a
squiggle under any value the module says is the wrong type — the same thing
taplo gives the TOML config.

It is implemented in nixd, not here: `nixd/docs/configuration.md`, under
"Schema directive". Shipped from a fork, pinned by
`~/nix/overlays/nixd-schema-directive.nix`.

## Measurements

Kept because they are expensive to re-derive and each one overturned the
intuition that had been blocking the work.

| assumed | measured |
|---|---|
| an unsaved buffer needs a temp-file mirror on disk to have `_file` provenance | `lib.modules.setDefaultModuleLocation "<real path>" (<buffer text>)` gives correct provenance for text that was never on disk, for both the bare-attrset and `{ lib, ... }:` module forms |
| `evalModules` costs ~1.3 s, so debouncing is what makes it viable | 0.27 s for `import <nixpkgs>` alone vs 0.27 s with `evalModules` + `deepSeq`. The 1.3 s was a cold store; the worker already holds nixpkgs, so the marginal cost is ~1 ms |
| the failing option path must be regex'd out of nix's prose | not at all: `lib.optionAttrSetToDocList` + `tryEval` enumerate failing paths structurally, and a targeted `deepSeq` per path yields that path's own message |
| only the first error is reachable, since `deepSeq` stops there | all of them, in one pass, in 0.34 s |

Two things the enumeration does that are not obvious, and which the
implementation has to undo:

- a parent option fails whenever a child does, so `outputs` is reported
  alongside `outputs.buffer`
- one undeclared key fails *every* sibling under the same submodule, each with
  the identical "does not exist" message — five leaves for one typo

## Still open

`evalModules` reports a file, never a line: the module system tracks definition
provenance at `_file` granularity and never calls `unsafeGetAttrPos` on
definition sites. Ranges are therefore reconstructed on nixd's side by walking
the document's own AST. Teaching nix to attach the option path and position as
structured data would remove that step, and is a nix change, not a nixd one.

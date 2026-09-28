# nh-doctree — porting notes

neohugo `hugolib/doctree` (the page, resource, taxonomy-entry and template-store trees) and the
radix tree under it, `github.com/armon/go-radix` v1.0.1-0.20221118154546-54df44f2176c.

Crate lead and owner: T27 (doctree). Status: every function of `hugolib/doctree/*.go` is ported
(checklists: all `OK`); go-radix is ported except four functions doctree never calls.

## Go file → Rust module

| Rust module | Go source(s) | Note |
|---|---|---|
| `dimensions` | `hugolib/doctree/dimensions.go` | ported |
| `nodeshifttree` | `hugolib/doctree/nodeshifttree.go` | ported; `WalkConfig`/`WalkState`/`walk`/`walk_mut` are the Rust form of `NodeShiftTreeWalker` |
| `simpletree` | `hugolib/doctree/simpletree.go` | ported; `SimpleThreadSafeTree` is a type alias (Rust borrows are the lock) |
| `support` | `hugolib/doctree/support.go` | ported |
| `treeshifttree` | `hugolib/doctree/treeshifttree.go` | ported |
| `radix` (private) | `github.com/armon/go-radix/radix.go` | ported, node for node (see below); `Minimum`, `Maximum`, `ToMap`, `NewFromMap` not ported (unused) |

## Why the tree is a radix port and not a `BTreeMap`

The plan (HUGO_LAYER.md §4.4) stores the trees in a `BTreeMap<String, _>`, because go-radix walks
its keys in byte order. That is true while nothing modifies the tree during a walk (the `order`
oracle checks it on every key set). But Hugo modifies `treePages` while walking it:
`assembleTerms` and `addMissingRootSections` insert pages, `applyAggregatesToTaxonomiesAndTerms`
deletes the visited term page, `DeleteAll`/`DeletePrefixAll` delete while walking. What Go's
`recursiveWalk` visits then depends on the radix node structure:

- a key inserted ahead of the walk is visited only if it lands in a subtree the walk has not
  passed (a key that splits a node the walk stands on is never visited);
- deleting the visited key re-visits its parent key when that was the parent's last child
  (`len(n.edges) == 0 → recursiveWalk(n)`), and re-visits a sibling when the parent is merged;
- deleting other keys can make the walk read a zeroed edge through a slice shared by
  `mergeChild`, and Go panics with a nil dereference.

The `walkmut` oracle records 3,044 such walks. The port reproduces all of them (visits, Go's 15
panics, final trees). A `BTreeMap` key snapshot re-read at visit time would get 898 wrong, a live
byte-order cursor 521, and among the 44 Hugo-shaped walks (assembleTerms on the docs site, term
deletes like `/tags/a` + `/tags/ab` where deleting `ab` makes Go visit `a` twice) the snapshot
gets 17 wrong and the live cursor 15. So `radix.rs` ports go-radix with Go's pointer semantics:
nodes, leaves and edge arrays live in arenas and are never freed (a node Go detaches keeps its
state for a walk that still stands on it); `[]edge` is a Go slice header over a shared backing
array with go1.27.1's `append` growth (2, 4, 8, 16, 32, 71, 143, 303, measured on amd64 and arm64),
so `mergeChild`'s slice sharing behaves as in Go. `recursiveWalk` is a resumable state machine
(`radix::Cursor`) so that the handle can modify the tree between two steps.

## Public API (for T13, T20, T21)

### `nodeshifttree` (T20 capture, T21 assembly: `treePages`, `treeResources`)

- `NodeShiftTree::new(Arc<dyn Shifter<T>>)`. Go's `Shape(d, v)` returns a shaped copy that shares
  the tree; in Rust every dimension-dependent method takes the shape `dims: Dimension` explicitly
  (`tree.shape(DIMENSION_LANGUAGE, lang_index)` → `[lang_index]`, `increment(dims, d)`).
- Insert: `insert_into_values_dimension(dims, s, v)` (+ `_with_lock`), `insert_into_current_dimension(dims, s, v)`
  → `(stored, replaced, updated)`; `insert_raw`, `insert_raw_with_lock`. Keys are cleaned (`"/"` → `""`)
  and validated; an invalid key panics like Go's `mustValidateKey`.
- Read: `get(dims, s)`, `has(dims, s)`, `get_raw(s)`, `for_each_in_dimension(s, d, f)`,
  `longest_prefix(dims, s, exact, predicate)` (character level, `path.Dir` retry),
  `longest_prefix_all(s)`, `walk_prefix_raw(prefix, f)`, `keys_with_prefix(prefix)`, `len()`.
- Delete: `delete(dims, key)` / `delete_with_status` (Go's `(T, bool)`), `delete_raw`, `delete_prefix(dims, prefix)`,
  `delete_all(key)`, `delete_prefix_all(prefix)`.
- Walks (Go `NodeShiftTreeWalker{Tree, Prefix, NoShift, Exact, Handle}.Walk`):
  `tree.walk(&WalkConfig { dims, prefix, no_shift, exact, .. }, |w, key, value, flag| Ok(terminate))`
  for read-only walks and `tree.walk_mut(&cfg, handle)` when the handle modifies the tree. The
  handle gets `w: &mut WalkState`: `w.skip_prefix(p)` (Go `w.SkipPrefix`), `w.tree()` (Go `w.Tree`),
  and for `walk_mut` `w.tree_mut()` to insert/delete. Nested walks: the handle captures the other
  tree (split the `PageTrees` borrow) and, to skip in the outer walk from an inner handle, the
  outer `w`. `WalkConfig::extend()` is Go's `Extend()`; skipped prefixes start empty per walk.
  The skeleton's `NodeShiftTreeWalker` still works for read-only walks whose handle needs neither
  `SkipPrefix` nor the tree.
- `Shifter<T>` (the skeleton trait) gained `delete_in_place(&self, v: &mut T, dims)`: Go's
  `Delete` mutates a multi-language node in place and the tree keeps it unless it is empty; the
  consuming `delete(v)` cannot return the remaining slots. The trees call `delete_in_place`; its
  default forwards to `delete` and panics on a partial delete. **T20's `ContentNodeShifter` must
  implement `delete_in_place`** (its `delete` can then be `unreachable!()`).
- `Lock`/`CanLock`/`LockType`: kept for the API, no-ops (borrows lock). `MutableTree`,
  `WalkableTree`, `MutableTrees`, `WalkableTrees` (support.go) are implemented for `NodeShiftTree`.

### `simpletree` (T13 template store, `WalkContext::data`)

`SimpleTree::new()`, `get`, `get_mut`, `insert` (returns the previous value), `longest_prefix`,
`walk`, `walk_prefix`, `walk_path` (callback `FnMut(&str, &T) -> Result<bool>`: `Ok(true)` stops,
`Err` stops and is returned, as Go's wrapper), `all()` (iterator in byte order), `len`, `delete`,
`delete_prefix`. Walks borrow the tree, so their callbacks cannot modify it (Hugo never does).

### `treeshifttree` (T20/T21 `treeTaxonomyEntries`)

`TreeShiftTree::new(num_languages)` (Go `NewTreeShiftTree(DimensionLanguage.Index(), n)`),
`with_dimension(d, n)`. Go's `Shape(d, v)` selects tree `v`; in Rust the value `v` is the first
argument of `get`, `insert`, `longest_prefix`, `walk_prefix`, `walk_path`, `all`, `lock`
(`shape(d, v)` validates it with Go's panics). `walk_prefix_raw`, `len_raw`, `delete`,
`delete_prefix`, `delete_all_func` act on every language.

### `support`

`WalkContext<T, D = go_value::Value>`: `data` / `data()` (Go `Data()`, a `SimpleTree<D>`; use
`WalkContext::<T, D>::default()` for another data type, e.g. the cascade), `add_event_listener`,
`send_event`, `event_sender()` (a cloneable `EventSender` for handlers that send events: Go's
handlers call `ctx.SendEvent`; a Rust handler cannot borrow the context), `handle_events`,
`add_post_hook`, `handle_events_and_hooks`. `Event::new(name, path, source)`,
`stop_propagation()`. `validate_key` (Go's messages, `%q` via go-strconv).

## Deliberate deviations

1. **Shapes are values** (`Dimension` / `usize`), not tree copies; see the API above.
2. **Locks.** Go's `sync.RWMutex` transactions (`Lock(writable)`, `LockType`, `*WithLock`,
   `SimpleThreadSafeTree`'s lock types, `LockTree`) are Rust borrows; the methods are kept as
   no-ops. No lock is held while a handle, predicate or event handler runs (HUGO_LAYER.md §4.8).
3. **`LongestPrefix` of a relative path.** Go's retry loop never ends when `path.Dir` reaches
   `"."` without a match (a fixed point); the port returns `None` there. Hugo passes absolute keys.
4. **`Shifter::delete_in_place`** (above).
5. **Zero values.** Go's `Get`/`LongestPrefix` return the zero `T` (and `""`) when nothing is
   found; Rust returns `Option`. `SimpleTree::insert` and `TreeShiftTree::insert` return the
   previous value (Go returns the inserted one). `DeleteAllFunc` skips a stored zero value in Go;
   Rust has none.
6. **A shifter that reports a match without a value** (`(None, true, _)`, Go: a nil `T` with
   `ok`) is treated as no match by the walks and `LongestPrefix`; Hugo's shifter never does this.
7. **Go panics.** A walk that would dereference a nil node in Go returns
   `Err("runtime error: invalid memory address or nil pointer dereference")`. The invariant
   panics are kept with Go's messages: `mustValidateKey`, `DimensionFlag::index` of 0
   ("dimension flag not set"), `NewTreeShiftTree` with length 0, `TreeShiftTree::shape`
   ("dimension mismatch", "value out of range").
8. **`resource.MarkStale`** in `DeleteAll`/`DeletePrefixAll` (rebuilds) is not ported; this is
   why the crate does not depend on nh-resource.
9. **Events.** Listeners are `Box<dyn FnMut(&mut Event<T>)>` without access to the context; they
   send through an `EventSender` (the queue is a `Mutex<VecDeque>` locked only to push or pop).
   Handler order, the path-prefix filter (`path + "/"`), `StopPropagation` and FIFO handling of
   events sent by handlers are Go's.
10. **Memory.** The radix arenas never free deleted nodes, leaves or edge arrays (a deleted value
    is kept, cloned out on delete). Fine for a one-shot build; a long-running server would want
    compaction between walks.
11. **Keys are `&str`.** Go keys can hold invalid UTF-8 (file names); Rust `str` cannot. Radix
    node prefixes are bytes and may end inside a UTF-8 sequence, as in Go.

## Dependencies

- nh-*: nh-common (`Error`, `Result`).
- Wave A: go-value (`Value`, the default data type of `WalkContext`), go-path (`path.Dir` in
  `LongestPrefix`), go-strconv (`%q` in `ValidateKey`'s messages).
- crates.io: none. Dev: `serde_json` (fixtures), `flate2` with `rust_backend` (decompression of
  the gzip fixtures; README rule 2).

## Verification

Go oracles in `tools/go-oracle/nh-doctree/<topic>/` (package `main` in the neohugo module,
importing `hugolib/doctree` directly; shared code in `dtcommon`) write
`crates/nh-doctree/tests/fixtures/<topic>/<topic>.json.gz`; the Rust tests are
`tests/<topic>.rs`. `cargo test` needs neither Go nor the network.

Key sets (substituted for the private seeksnack corpus): the tree keys of every content file of
this repository's sites (`docs/content/en`, `hugolib/testsite/content` + `content_nn` as Thai,
`create/skeletons/theme/content`), derived like Hugo (`paths.PathParser` with the default content
types and output formats, languages en/th, `Base()`, leaf-bundle content files as page resources,
home `""`); the template paths and the template-store keys (`toKeyCategoryAndDescriptor`'s
directory / partial / shortcode / `_default` / `_markup` rules) of the embedded templates, the
docs site's layouts and the skeleton theme; the paths quoted in `docs/rust-port/specs/*.md`
(seeksnack keys like `/biscuit/koalas-march-chocolate`, `/brands/lay's`, `/tags/-ขนมเปี๊ยะ`, and
their normalized form); synthetic keys (shared prefixes, `/` vs `-` vs `.` vs `_` neighbours,
`//`, trailing slashes, Thai/Japanese/emoji/combining marks, a 40-level path, siblings differing
after a 85-byte common prefix, all 94 printable ASCII labels under one node to grow an edge
slice past 71); and dense random tries over `/ a b - . ข`.

| topic | what | checks |
|---|---|---|
| `order` | 13 key sets (docs pages 945, resources 72, all content 1,017, templates 183, specs 160, synthetic 226, mixed 1,564, 6 random); Walk, All, WalkPrefixRaw order; per query (35,066: each key, key + `/`, `/x`, `-`, `.`, `~`, minus its last rune, `path.Dir`): Get, LongestPrefix, WalkPrefix, WalkPath, LongestPrefixAll, prefix walks (skeleton walker and `walk`); byte-order == walk order | 245,514 |
| `nodeshift` | Hugo's contentNodeShifter semantics: the docs/testsite/skeleton pages (every third also Thai, Thai-only pages) with walks per language × NoShift × Exact, section walks, Get/Has, CurrentSection/Parent-style LongestPrefix (branch predicate, `path.Dir` retry), resource ownership lookups, LongestPrefixAll chains, ForEeachInDimension, WalkPrefixRaw, per-language deletes; the resources tree with forEachResourceInPage walks and assembleResources' InsertIntoCurrentDimension; 60 random sequences | 34,150 operations, 20,035 walk visits |
| `walkmut` | walks whose handle modifies the tree: assembleTerms and addMissingRootSections (docs pages, both languages), 40 term-delete walks, 3,000 random walks (insert children/siblings/prefixes/other keys, delete visited/other keys, DeleteAll, SkipPrefix, stop, error) | 3,044 scenarios, 19,470 visits, 15 Go panics |
| `treeshift` | TreeShiftTree and SimpleTree walks: Insert, Get, LongestPrefix, WalkPrefix/WalkPath (stop and error at a given visit), WalkPrefixRaw, All, LenRaw, Delete, DeletePrefix (leaves empty radix nodes), DeleteAllFunc; Hugo's taxonomy-entry keys | 46,098 operations |
| `events` | Go's TestTreeEvents + 400 random trees: listeners per branch, re-sent events, StopPropagation, one or two event names, failing post hooks | 2,379 handler/hook calls + weights |

Results: 0 differences. The Go tests (`dimensions_test.go`, `nodeshiftree_test.go`,
`treeshifttree_test.go`) are ported in `tests/go_tables.rs` (TestTreePara with 8 threads over a
`Mutex`), go-radix's `radix_test.go` in `radix.rs` (unit tests), plus Rust-only edges (invalid key
panic, relative `LongestPrefix`, the `delete_in_place` default, skip-prefix reset, walk stop and
error).

The doctree code has no platform-dependent behaviour; the `walkmut` and `nodeshift` oracles built
for linux/arm64 (qemu) write the same bytes as on amd64. Regenerate (byte-identical):

```sh
export GOTOOLCHAIN=go1.27.1
for t in order nodeshift walkmut treeshift events; do go run ./tools/go-oracle/nh-doctree/$t; done
```

## Known gaps and requests to other crates

- **T20** (`content_map_trees.rs`): implement `Shifter::delete_in_place` for `ContentNodeShifter`
  (`contentNodeIs`/`resourceSources` clear the slot in place). I01 may then change `delete` to
  take `&mut T` and drop the default.
- **T20/T21**: use `walk_mut` for the walks that modify `treePages` (assembleTerms,
  addMissingRootSections, the term delete in applyAggregatesToTaxonomiesAndTerms, and
  assembleResources' `InsertIntoCurrentDimension` on `treeResources`), not a snapshot of keys.
- **HUGO_LAYER.md §4.4** says the storage is a `BTreeMap`; it is the go-radix port (above).
- go-radix `Minimum`, `Maximum`, `ToMap`, `NewFromMap`: not ported (doctree does not use them).

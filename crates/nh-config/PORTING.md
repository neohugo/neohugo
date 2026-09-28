# nh-config — porting notes

neohugo config (base), config/{security,privacy,services}, common/hexec, common/neohugo, and a
general port of `mitchellh/mapstructure` (`WeakDecode`, `Decoder`). Owner and crate lead: Wave B
task T04 (config-base-media).

## Go file → Rust module

| Rust module | Go source(s) | Status / note |
|---|---|---|
| `common_config` | `config/commonConfig.go` | ported |
| `config_loader` | `config/configLoader.go` | ported |
| `config_provider` | `config/configProvider.go` | ported |
| `default_config_provider` | `config/defaultConfigProvider.go` | ported |
| `env` | `config/env.go` | ported |
| `namespace` | `config/namespace.go` | ported |
| `decode` | `github.com/mitchellh/mapstructure@v1.5.1-0.20231216201459-8508981c8b6c` (`mapstructure.go`, `error.go`, `decode_hooks.go`: `StringToTimeDurationHookFunc`) | ported for Rust targets (the `Decode` trait) |
| `goregexp` | Go `regexp` | re-export of `nh_common::goregexp` (the port of Go's `regexp` and `regexp/syntax`; see nh-common deviation 32) |
| `security::security_config` | `config/security/securityConfig.go` | ported |
| `security::whitelist` | `config/security/whitelist.go` | ported |
| `privacy` | `config/privacy/privacyConfig.go` | ported |
| `services` | `config/services/servicesConfig.go` | ported |
| `hexec` | `common/hexec/exec.go` | ported |
| `neohugo::neohugo` | `common/neohugo/neohugo.go` | ported |
| `neohugo::version` | `common/neohugo/version.go`, `common/neohugo/version_current.go` | ported |

Every GO PORTING CHECKLIST entry is `OK`.

## Dependencies

- nh-*: nh-common, nh-parser (config file decoding), nh-langs.
- Wave A: go-value, go-time, go-json, go-fmt, go-strconv, go-unicode, go-path, go-sort.
- crates.io: none. Go's `regexp` (security whitelists, cache busters, server header and redirect
  globs) is nh-common's `goregexp`, a port of Go's package; the former `regex`/`regex-syntax`
  translation layer is gone.
- dev: `serde_json`, `flate2` (`rust_backend`, gunzip of fixtures).

## mapstructure (`decode`)

`decode` is a function-by-function port of mapstructure's `Decoder` for the targets Hugo
decodes: a Rust type implements `Decode` (primitives, `GoString`, `String`, `go_time::Duration`,
`Vec<T>`, `[T; N]`, `Option<Box<T>>` for `*T`, `BTreeMap<String, T>`, `go_value::Map`, `Value`
and `AnyValue` for `interface{}`, `Int64`/`Uint64`), and a struct declares its fields with
`decode_struct!` (`FieldRef::new(name, &mut field)` with the `mapstructure` tag, `,squash`,
`,remain`, unexported and embedded fields). `DecoderConfig` has `decode_hook`, `error_unused`,
`error_unset`, `zero_fields`, `weakly_typed_input` and `squash`; `weak_decode_into` is
`mapstructure.WeakDecode` and `decode_into` is `mapstructure.Decode`. All error texts are
mapstructure's (`'<name>' expected type 'string', got unconvertible type '[]interface {}', value:
'[a]'`, the sorted `* ` list of `mapstructure.Error`). A Go reflect panic in the middle of a decode
(e.g. an array element assigned from an incompatible map) is `DecodeError::Panic` and re-raised by
the callers that Go lets panic.

## Deliberate deviations

1. **nil vs empty.** Go distinguishes a nil map or slice from an empty one; the Rust config
   types use empty `Vec`s and maps (`GetStringMap` of a missing key is an empty map, nh-common
   deviation 13). JSON dumps and hashes are unaffected (nh-common hashes both alike).
2. **Map order.** Go iterates maps in random order: when several keys fold to one struct field,
   or several map entries fail to decode, Go picks one at random; the port uses byte order.
   Hugo's provider lower-cases keys, so the first case cannot happen with config input. The
   oracles mark results that depend on the order (`nondet`, found by 20–100 reruns) and do not
   compare them.
3. **String fields** of config structs are `String`: keys and values that are not valid UTF-8
   are converted lossily. `json.Number` inputs (never produced by Hugo's decoders) are not
   handled. `decodeBasic` into an interface that already holds a value decodes into that
   dynamic type for scalars, maps and slices; other dynamic types (pointers, structs, typed
   nils) are replaced by the input. `decodeMapFromStruct` (a struct input decoded into a
   map) handles the struct objects the config produces, not arbitrary Go structs.
4. (Resolved; the number is kept.) Regexp errors and matching are Go's: `goregexp` is now a
   port of Go's `regexp` (nh-common).
5. **`FromConfigString`** errors carry nh-parser's TOML error position (nh-parser deviation 1).
6. **HugoInfo.** `CommitHash` and `BuildDate` are empty (no VCS stamp) and `GoVersion` is the
   constant `go1.27.1` (the toolchain of the golden build).
7. **Markup scope.** `hugo.Context.MarkupScope` reads a getter that the page layer registers
   with `neohugo::set_markup_scope_getter` (Go reads it from the context value directly).
8. **hexec.** `Exec::new` reads `os.Environ()` and `$PATH` when created; `Exec::new_with_env`
   takes both explicitly (the tests cannot change the process environment safely). The
   executable check uses the mode bits (Go asks `faccessat`; the same for root and for files the
   user owns). Environment entries without `=` are dropped when starting the process (Go passes
   them to `execve`; `env` does not print them either, see the hexec fixture).
9. **`load_config_from_dir`** takes a `&Path` on the OS filesystem (Go takes an `afero.Fs`) and
   returns `(Option<Box<dyn Provider>>, Vec<String>)`.
10. **`GetExecEnviron`** takes the `_jsconfig` file list as a parameter (Go reads it from the
    `BaseConfig` of the caller's `config.AllProvider`).
11. **Provider locks** are poison tolerant: a Go panic reproduced by `merge` (bad merge strategy,
    `interface conversion` on a non-map value) does not make later calls fail.

## Known gaps / requests

- T13 (page/markup) registers the markup-scope getter (deviation 7).
- T09 (allconfig) decodes its config sections with `decode_struct!`/`weak_decode_into`; the
  minify config mirror in `tests/decode.rs` shows the pattern for T07's `minifiers.MinifyConfig`.
- nh-langs could switch its private mapstructure port to `decode` (same error texts): declare
  `LanguageConfig` with `decode_struct!` and decode `BTreeMap<String, LanguageConfig>` with
  `weak_decode_into`. It would then depend on nh-config (nh-config already depends on
  nh-langs, so the shared part would have to move below both, e.g. to nh-common).

`neohugo.Version`'s fields are `i64` (Go `int`): `parseVersion` keeps `strconv.Atoi`'s value
when it fails (clamped to the int range, 0 for bad syntax), so comparing a `VersionString` with
a number out of range (e.g. `uint64` max) gives Go's result (`version.rs` unit test; nh-tplfuncs'
16 `lt`/`le`/`gt`/`ge` cases now pass).

## Verification

| topic | inputs | Rust test | checks |
|---|---|---|---|
| `provider/provider.json.gz` | 1,532 op scripts on `config.New()`/`config.NewFrom()`: the scripts of `defaultConfigProvider_test.go`, the seeksnack `[imaging]` section (SourceHash `4bf645f71319dd1d`), the seeksnack config dumps (`config-{en,th}.json`, floats and TOML-like int64), `docs/hugo.toml`, 1,500 seeded random scripts (case-mixed and dotted keys, `_merge` strategies, typed maps, every value kind) | `tests/provider.rs` | 24,685 ops: Set/Get*/Merge/SetDefaults/SetDefaultMergeStrategy/WalkParams/IsSet/Keys results, the whole root after mutating ops, Go panics, section hashes |
| `decode/decode.json.gz` | mapstructure over 7 test structs (every kind, squash, remain, embedded, pointers, arrays, maps, durations) × 7 decoder configs, and the provider path of BaseConfig, BuildConfig, Pagination, PageConfig, Sitemap, Server, Security, Services, Privacy and a Minify mirror with the seeksnack sections, `docs/hugo.toml`, adversarial and random inputs | `tests/decode.rs` | 20,575 cases: decoded struct dumps, error texts, 24 Go panics |
| `misc/misc.json.gz` | env (worker multiplier, memory limit, SetEnvVars), versions and comparisons, HugoInfo fields and methods, GetExecEnviron, config loader (file names, config strings, directory trees), security policies and TOML, cache busters, server headers/redirects, DecodeNamespace hashes, Go regexp matching and errors | `tests/misc.rs` | 254 cases |
| `hexec/hexec.json.gz` | 7 scenarios with real shell scripts in a temporary tree (node_modules/.bin, npx, PATH, tailwindcss order, non-executable files and directories, empty and missing PATH entries, the npx cache, default and custom security policies) | `tests/hexec.rs` | 39 runs: the binary run, its arguments, stdin and environment (osEnv filter plus WithEnviron), and the errors of New/Npx/Run (`IsNotFound`) |

Results: 0 mismatches.

Regenerate (the decode and misc fixtures depend on arm64 float→int conversions, like nh-langs;
the provider and hexec fixtures are platform independent):

```sh
export GOTOOLCHAIN=go1.27.1
go run ./tools/go-oracle/nh-config/provider -root . -out crates/nh-config/tests/fixtures/provider
go run ./tools/go-oracle/nh-config/hexec -out crates/nh-config/tests/fixtures/hexec
GOARCH=arm64 CGO_ENABLED=0 go build -o /tmp/nhc-decode-arm64 ./tools/go-oracle/nh-config/decode
GOARCH=arm64 CGO_ENABLED=0 go build -o /tmp/nhc-misc-arm64 ./tools/go-oracle/nh-config/misc
qemu-aarch64-static /tmp/nhc-decode-arm64 -root . -out crates/nh-config/tests/fixtures/decode
qemu-aarch64-static /tmp/nhc-misc-arm64 -root . -out crates/nh-config/tests/fixtures/misc
```

The fixtures regenerate byte for byte.

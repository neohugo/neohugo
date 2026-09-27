# nh-config — porting notes

neohugo config (base), config/{security,privacy,services}, common/hexec, common/neohugo.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `common_config` | `config/commonConfig.go` | T04 config-base-media |  |
| `config_loader` | `config/configLoader.go` | T04 config-base-media |  |
| `config_provider` | `config/configProvider.go` | T04 config-base-media |  |
| `default_config_provider` | `config/defaultConfigProvider.go` | T04 config-base-media |  |
| `env` | `config/env.go` | T04 config-base-media |  |
| `namespace` | `config/namespace.go` | T04 config-base-media |  |
| `decode` | — | T04 config-base-media | NEW: mitchellh/mapstructure WeakDecode semantics (case-insensitive field match, weak conversions) over go_value::Value |
| `security::security_config` | `config/security/securityConfig.go` | T04 config-base-media |  |
| `security::whitelist` | `config/security/whitelist.go` | T04 config-base-media |  |
| `privacy` | `config/privacy/privacyConfig.go` | T04 config-base-media |  |
| `services` | `config/services/servicesConfig.go` | T04 config-base-media |  |
| `hexec` | `common/hexec/exec.go` | T04 config-base-media |  |
| `neohugo::neohugo` | `common/neohugo/neohugo.go` | T04 config-base-media |  |
| `neohugo::version` | `common/neohugo/version.go`, `common/neohugo/version_current.go` | T04 config-base-media |  |

## Dependencies

- nh-*: nh-common, nh-parser, nh-langs
- Wave A (to add when available): go-time, go-json
- crates.io (justify each): regex (security whitelist patterns; RE2-compatible subset)

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._

# nh-resources — porting notes

neohugo resources/*.go (Spec, genericResource, resourceAdapter + transformation chain, caches, image resource), resources/postpub, resources/jsconfig.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `resource` | `resources/resource.go` | T14 resources-core |  |
| `resource_spec` | `resources/resource_spec.go` | T14 resources-core |  |
| `transform` | `resources/transform.go` | T14 resources-core |  |
| `resource_cache` | `resources/resource_cache.go` | T14 resources-core |  |
| `resource_metadata` | `resources/resource_metadata.go` | T14 resources-core |  |
| `image` | `resources/image.go` | T14 resources-core |  |
| `image_cache` | `resources/image_cache.go` | T14 resources-core |  |
| `post_publish` | `resources/post_publish.go` | T14 resources-core |  |
| `postpub::postpub` | `resources/postpub/postpub.go` | T14 resources-core |  |
| `postpub::fields` | `resources/postpub/fields.go` | T14 resources-core |  |
| `jsconfig` | `resources/jsconfig/jsconfig.go` | T14 resources-core |  |

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-hugofs, nh-helpers, nh-resource, nh-images, nh-page, nh-allconfig, nh-tpl
- Wave A (to add when available): go-hashstructure, go-json
- crates.io (justify each): sha2, md-5, base64, hex

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._

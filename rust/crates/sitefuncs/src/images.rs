//! Images: `resize`, `fill`, `fit`, `crop`, `process`, `image_filter` (queued in the
//! [`ImageQueue`]: the result's name and size are known at once, its pixels are processed in
//! phase E6), `exif`; `image_colors` and `qr_code` are not implemented yet.

use std::sync::Arc;

use neohugo_images::{Action, Anchor, ImageFilter, ImageFormat, ImageQueue, ImageSpec, Resample};
use neohugo_resources::ResourceStore;
use tera::{Kwargs, State, TeraResult, Value};

use crate::Handles;
use crate::call::{
    Registrar, SiteFilter, SiteFunction, chain, field, list, map_value, msg, resource_id, to_json,
};
use crate::resources::view_value;

pub(crate) fn register(r: &mut Registrar<'_>, h: &Handles) {
    for (name, action) in [
        ("resize", Some(Action::Resize)),
        ("fill", Some(Action::Fill)),
        ("fit", Some(Action::Fit)),
        ("crop", Some(Action::Crop)),
        ("process", None),
    ] {
        r.filter(
            name,
            Process {
                store: Arc::clone(&h.store),
                images: Arc::clone(&h.images),
                name,
                action,
            },
        );
    }
    r.filter(
        "image_filter",
        Filter {
            store: Arc::clone(&h.store),
            images: Arc::clone(&h.images),
        },
    );
    r.filter(
        "exif",
        Exif {
            store: Arc::clone(&h.store),
            images: Arc::clone(&h.images),
        },
    );
    r.filter("image_colors", NotImplemented("image_colors"));
    r.function("qr_code", NotImplemented("qr_code"));
}

/// Queues `spec` then `filters` on image resource `v`; its resource view.
fn enqueue(
    store: &ResourceStore,
    images: &ImageQueue,
    v: &Value,
    name: &str,
    spec: Option<&ImageSpec>,
    filters: &[ImageFilter],
) -> TeraResult<Value> {
    let id = resource_id(store, v, name)?;
    let input = store.image_input(id).ok_or_else(|| {
        msg(format!(
            "{name}: {} is not an image that can be processed",
            store.resource(id).name
        ))
    })?;
    let e = images
        .enqueue(&input, spec, filters)
        .map_err(|e| chain(name, e))?;
    Ok(view_value(store, store.register_image(id, &e)))
}

/// `resize`, `fill`, `fit`, `crop` and `process` with `width=`, `height=`, `format=`,
/// `quality=`, `filter=`, `anchor=` and/or a Hugo `spec=` string (`"600x400 webp q75"`; the
/// kwargs win). `process` takes its action from the spec.
struct Process {
    store: Arc<ResourceStore>,
    images: Arc<ImageQueue>,
    name: &'static str,
    action: Option<Action>,
}

fn parsed<T: std::str::FromStr<Err = String>>(
    kw: &Kwargs,
    key: &str,
    name: &str,
) -> TeraResult<Option<T>> {
    kw.get::<&str>(key)?
        .map(|s| {
            s.parse::<T>()
                .map_err(|e| msg(format!("{name}({key}=): {e}")))
        })
        .transpose()
}

fn dimension(kw: &Kwargs, key: &str, name: &str) -> TeraResult<Option<u32>> {
    kw.get::<i64>(key)?
        .map(|n| {
            u32::try_from(n)
                .ok()
                .filter(|n| *n > 0)
                .ok_or_else(|| msg(format!("{name}({key}={n}): expected a positive integer")))
        })
        .transpose()
}

impl Process {
    fn spec(&self, kw: &Kwargs) -> TeraResult<ImageSpec> {
        let name = self.name;
        let mut spec = match kw.get::<&str>("spec")? {
            // The filter's action goes first, so a spec that names another one overrides it.
            Some(s) => match self.action {
                Some(a) => format!("{} {s}", a.name()),
                None => s.to_owned(),
            }
            .parse::<ImageSpec>()
            .map_err(|e| chain(format!("{name}(spec=\"{s}\")"), e))?,
            None => ImageSpec::default(),
        };
        if let Some(a) = self.action {
            if spec.action.is_some_and(|b| b != a) {
                return Err(msg(format!(
                    "{name}(spec=): the spec names another action; use `process`"
                )));
            }
            spec.action = Some(a);
        }
        if let Some(w) = dimension(kw, "width", name)? {
            spec.width = Some(w);
        }
        if let Some(h) = dimension(kw, "height", name)? {
            spec.height = Some(h);
        }
        if let Some(f) = kw.get::<&str>("format")? {
            spec.format = Some(
                ImageFormat::from_extension(f)
                    .ok_or_else(|| msg(format!("{name}(format=\"{f}\"): unknown image format")))?,
            );
        }
        if let Some(q) = kw.get::<i64>("quality")? {
            spec.quality = Some(
                u8::try_from(q)
                    .ok()
                    .filter(|q| (1..=100).contains(q))
                    .ok_or_else(|| msg(format!("{name}(quality={q}): expected 1–100")))?,
            );
        }
        if let Some(f) = parsed::<Resample>(kw, "filter", name)? {
            spec.filter = Some(f);
        }
        if let Some(a) = parsed::<Anchor>(kw, "anchor", name)? {
            spec.anchor = Some(a);
        }
        Ok(spec)
    }
}

impl SiteFilter for Process {
    fn call(&self, v: Value, kw: &Kwargs, _: &State) -> TeraResult<Value> {
        let spec = self.spec(kw)?;
        enqueue(&self.store, &self.images, &v, self.name, Some(&spec), &[])
    }
}

/// `r | image_filter(filters=[{"op": …}, …])`: `overlay` and `mask` take a resource view as
/// `image`; `process` a spec string.
struct Filter {
    store: Arc<ResourceStore>,
    images: Arc<ImageQueue>,
}

impl Filter {
    fn filter(&self, f: &Value) -> TeraResult<ImageFilter> {
        let mut json = to_json(f);
        if let Some(image) = field(f, "image").filter(|i| field(i, "__rid").is_some()) {
            let id = resource_id(&self.store, image, "image_filter(image=)")?;
            let input = self.store.image_input(id).ok_or_else(|| {
                msg(format!(
                    "image_filter: {} is not an image",
                    self.store.resource(id).name
                ))
            })?;
            let input = serde_json::to_value(&input).map_err(|e| chain("image_filter", e))?;
            if let Some(m) = json.as_object_mut() {
                m.insert("image".to_owned(), input);
            }
        }
        serde_json::from_value(json).map_err(|e| chain("image_filter(filters=)", e))
    }
}

impl SiteFilter for Filter {
    fn call(&self, v: Value, kw: &Kwargs, _: &State) -> TeraResult<Value> {
        let filters = kw.must_get::<Value>("filters")?;
        let filters = list(&filters, "image_filter(filters=)")?
            .iter()
            .map(|f| self.filter(f))
            .collect::<TeraResult<Vec<_>>>()?;
        enqueue(
            &self.store,
            &self.images,
            &v,
            "image_filter",
            None,
            &filters,
        )
    }
}

/// `r | exif`: `{date, lat, long, tags}` of an image with EXIF data, else none.
struct Exif {
    store: Arc<ResourceStore>,
    images: Arc<ImageQueue>,
}

impl SiteFilter for Exif {
    fn call(&self, v: Value, _: &Kwargs, _: &State) -> TeraResult<Value> {
        let id = resource_id(&self.store, &v, "exif")?;
        let bytes = self.store.content(id).map_err(|e| chain("exif", e))?;
        let Some(x) = neohugo_images::exif::read(&bytes, &self.images.imaging().exif) else {
            return Ok(Value::none());
        };
        let (lat, long) = x.lat_long.map_or((Value::none(), Value::none()), |(a, b)| {
            (Value::from(a), Value::from(b))
        });
        let tags = neohugo_base::Value::map(x.tags.into_iter().collect());
        Ok(map_value([
            (
                "date",
                x.date
                    .map_or_else(Value::none, |d| neohugo_base::Value::Date(d).to_tera()),
            ),
            ("lat", lat),
            ("long", long),
            ("tags", tags.to_tera()),
        ]))
    }
}

/// A name of the spec whose implementation does not exist yet.
struct NotImplemented(&'static str);

impl NotImplemented {
    fn fail(&self) -> tera::Error {
        msg(format!(
            "`{}` is not implemented in this build of neohugo",
            self.0
        ))
    }
}

impl SiteFilter for NotImplemented {
    fn call(&self, _: Value, _: &Kwargs, _: &State) -> TeraResult<Value> {
        Err(self.fail())
    }
}

impl SiteFunction for NotImplemented {
    fn call(&self, _: &Kwargs, _: &State) -> TeraResult<Value> {
        Err(self.fail())
    }
}

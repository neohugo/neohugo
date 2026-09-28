//! Port of `navigation/menu.go`.
//!
//! Owner: Wave B task T12 (page-collections).

//! Go `navigation`: menus from config (`[languages.X.menus.main]`) and front matter. Not
//! referenced by seeksnack templates (`.Site.Menus` unused) but decoded.
//!
//! Go menus are `[]*MenuEntry` whose entries hugolib mutates while assembling (`Children`,
//! `SetPageValues`); here an entry is an `Arc<MenuEntry>` built before it is shared, and entry
//! identity (Go pointer equality) is `Arc::ptr_eq`.

use std::any::Any;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

use go_value::{GoString, HostCtx, Map, MapType, Object, SafeKind, SliceType, Value};
use nh_common::Result;
use nh_common::object::{GoResult, NamedMethods, args};
use nh_config::decode::{
    Decode, DecodeError, Decoder, FieldRef, OutKind, decode_string, weak_decode_into,
};
use nh_config::namespace::{ConfigNamespace, decode_namespace};

use super::menu_cache::smc;
use crate::page::{PageRef, page_from_value};

/// Go type strings.
pub const MENU_ENTRY_TYPE: &str = "*navigation.MenuEntry";
pub const MENU_TYPE: &str = "navigation.Menu";
pub const MENUS_TYPE: &str = "navigation.Menus";
pub const PAGE_MENUS_TYPE: &str = "navigation.PageMenus";

/// Go: `navigation.MenuConfig`.
#[derive(Clone, Debug, Default)]
pub struct MenuConfig {
    pub identifier: String,
    pub parent: String,
    pub name: String,
    pub pre: GoString,
    pub post: GoString,
    pub url: String,
    pub page_ref: String,
    pub weight: i64,
    pub title: String,
    pub params: Option<Map>,
}

/// A `template.HTML` decode target (a string kind).
#[derive(Clone, Default)]
struct HtmlTarget(GoString);

impl Decode for HtmlTarget {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Borrowed("template.HTML")
    }
    fn out_kind(&self) -> OutKind {
        OutKind::String
    }
    fn decode_kind(
        &mut self,
        d: &Decoder<'_>,
        name: &str,
        data: &Value,
    ) -> std::result::Result<(), DecodeError> {
        decode_string(d, name, data, &mut self.0, "template.HTML")
    }
    fn set_zero(&mut self) {
        self.0 = GoString::empty();
    }
    fn is_zero_value(&self) -> bool {
        self.0.is_empty()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The mapstructure decode target of `MenuConfig` (Go field types).
#[derive(Clone)]
struct MenuConfigTarget {
    identifier: String,
    parent: String,
    name: String,
    pre: HtmlTarget,
    post: HtmlTarget,
    url: String,
    page_ref: String,
    weight: i64,
    title: String,
    params: Map,
}

impl Default for MenuConfigTarget {
    fn default() -> Self {
        MenuConfigTarget {
            identifier: String::new(),
            parent: String::new(),
            name: String::new(),
            pre: HtmlTarget::default(),
            post: HtmlTarget::default(),
            url: String::new(),
            page_ref: String::new(),
            weight: 0,
            title: String::new(),
            params: Map::new(MapType::Params),
        }
    }
}

nh_config::decode_struct!(MenuConfigTarget, "navigation.MenuConfig", |s| vec![
    FieldRef::new("Identifier", &mut s.identifier),
    FieldRef::new("Parent", &mut s.parent),
    FieldRef::new("Name", &mut s.name),
    FieldRef::new("Pre", &mut s.pre),
    FieldRef::new("Post", &mut s.post),
    FieldRef::new("URL", &mut s.url),
    FieldRef::new("PageRef", &mut s.page_ref),
    FieldRef::new("Weight", &mut s.weight),
    FieldRef::new("Title", &mut s.title),
    FieldRef::new("Params", &mut s.params),
]);

/// Whether the decoded map sets `params` to a non-nil value (Go's `Params` stays nil otherwise).
fn has_params(input: &Value) -> bool {
    match input {
        Value::Map(m) => nh_config::decode::field(m, "Params").is_some_and(|v| !v.is_nil()),
        _ => false,
    }
}

impl MenuConfig {
    /// Go: `mapstructure.WeakDecode(entry, &menuConfig)` on top of `self`.
    pub fn weak_decode(&mut self, input: &Value) -> Result<()> {
        let mut t = MenuConfigTarget {
            identifier: self.identifier.clone(),
            parent: self.parent.clone(),
            name: self.name.clone(),
            pre: HtmlTarget(self.pre.clone()),
            post: HtmlTarget(self.post.clone()),
            url: self.url.clone(),
            page_ref: self.page_ref.clone(),
            weight: self.weight,
            title: self.title.clone(),
            params: self
                .params
                .clone()
                .unwrap_or_else(|| Map::new(MapType::Params)),
        };
        let had = self.params.is_some();
        weak_decode_into(input, &mut t)?;
        self.identifier = t.identifier;
        self.parent = t.parent;
        self.name = t.name;
        self.pre = t.pre.0;
        self.post = t.post.0;
        self.url = t.url;
        self.page_ref = t.page_ref;
        self.weight = t.weight;
        self.title = t.title;
        self.params = if had || has_params(input) {
            Some(t.params)
        } else {
            None
        };
        Ok(())
    }
}

/// Go: `navigation.MenuEntry`.
#[derive(Clone, Default)]
pub struct MenuEntry {
    pub config: MenuConfig,
    pub menu: String,
    pub configured_url: String,
    /// The page (a page.Page value) if the entry points to one.
    pub page: Option<Value>,
    pub children: Menu,
}

impl MenuEntry {
    /// The entry's page, if it has one (Go `!types.IsNil(m.Page)`).
    pub fn page_ref(&self) -> Option<PageRef> {
        self.page.as_ref().and_then(page_from_value)
    }

    /// Go: `m.URL()` — the page's RelPermalink, else the configured URL.
    // Go: navigation/menu.go:URL
    pub fn url(&self) -> String {
        // Check page first.
        // In Hugo 0.86.0 we added `pageRef`, a way to connect menu items in site config to
        // pages. This means that you now can have both a Page and a configured URL. Having the
        // configured URL as a fallback if the Page isn't found is obviously more useful,
        // especially in multilingual sites.
        if let Some(p) = self.page_ref() {
            return p.0.rel_permalink();
        }

        self.configured_url.clone()
    }

    /// Go: `m.HasChildren()` (Go: `Children != nil`; hugolib only sets children by adding).
    // Go: navigation/menu.go:HasChildren
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Go: `m.KeyName()`.
    // Go: navigation/menu.go:KeyName
    pub fn key_name(&self) -> String {
        if !self.config.identifier.is_empty() {
            return self.config.identifier.clone();
        }
        self.config.name.clone()
    }

    // Go: navigation/menu.go:hopefullyUniqueID
    fn hopefully_unique_id(&self) -> String {
        if !self.config.identifier.is_empty() {
            self.config.identifier.clone()
        } else if !self.url().is_empty() {
            self.url()
        } else {
            self.config.name.clone()
        }
    }

    /// Go: `m.isEqual(inme)`.
    // Go: navigation/menu.go:isEqual
    pub fn is_equal(&self, inme: &MenuEntry) -> bool {
        self.hopefully_unique_id() == inme.hopefully_unique_id()
            && self.config.parent == inme.config.parent
    }

    /// Go: `m.isSameResource(inme)`.
    // Go: navigation/menu.go:isSameResource
    pub fn is_same_resource(&self, inme: &MenuEntry) -> bool {
        if self.is_same_page(inme.page_ref().as_ref()) {
            return self.page_ref().map(|p| p.0.page_id())
                == inme.page_ref().map(|p| p.0.page_id());
        }
        let (murl, inmeurl) = (self.url(), inme.url());
        !murl.is_empty() && !inmeurl.is_empty() && murl == inmeurl
    }

    /// Go: `m.isSamePage(p)`.
    // Go: navigation/menu.go:isSamePage
    pub fn is_same_page(&self, p: Option<&PageRef>) -> bool {
        if let (Some(mp), Some(p)) = (self.page_ref(), p) {
            return mp.0.page_id() == p.0.page_id();
        }
        false
    }
}

/// Go: `SetPageValues(m, p)`.
// Go: navigation/menu.go:SetPageValues
pub fn set_page_values(m: &mut MenuEntry, p: &PageRef) {
    m.page = Some(p.to_value());
    if m.config.name.is_empty() {
        m.config.name = p.0.link_title();
    }
    if m.config.title.is_empty() {
        m.config.title = crate::page::Page::title(&*p.0);
    }
    if m.config.weight == 0 {
        m.config.weight = p.0.weight();
    }
}

/// Go: `navigation.Menu` (sorted: weight (0 last), `compare.Strings(Name)`, Identifier).
pub type Menu = Vec<Arc<MenuEntry>>;
/// Go: `navigation.Menus`.
pub type Menus = Arc<BTreeMap<String, Menu>>;
/// Go: `navigation.PageMenus`.
pub type PageMenus = BTreeMap<String, Arc<MenuEntry>>;

/// Go: `Menu.Add(me)` — append, then sort.
// Go: navigation/menu.go:Add
pub fn add(m: &mut Menu, me: Arc<MenuEntry>) {
    m.push(me);
    // TODO(bep)
    sort(m);
}

/// Go: `menuEntryBy.Sort(menu)` — `sort.Stable`.
// Go: navigation/menu.go:Sort
fn menu_entry_by_sort(by: &dyn Fn(&MenuEntry, &MenuEntry) -> bool, menu: &mut Menu) {
    go_sort::stable_by(menu, |a, b| by(a, b));
}

/// Go: `defaultMenuEntrySort`.
// Go: navigation/menu.go:defaultMenuEntrySort
pub fn default_menu_entry_sort(m1: &MenuEntry, m2: &MenuEntry) -> bool {
    if m1.config.weight == m2.config.weight {
        let c = nh_common::compare::strings(m1.config.name.as_bytes(), m2.config.name.as_bytes());
        if c == 0 {
            return m1.config.identifier.as_bytes() < m2.config.identifier.as_bytes();
        }
        return c < 0;
    }

    if m2.config.weight == 0 {
        return true;
    }

    if m1.config.weight == 0 {
        return false;
    }

    m1.config.weight < m2.config.weight
}

/// Go: `Menu.Sort()` — in place by weight, name and then identifier.
// Go: navigation/menu.go:Sort
pub fn sort(m: &mut Menu) {
    menu_entry_by_sort(&default_menu_entry_sort, m);
}

/// Go: `Menu.Limit(n)`.
// Go: navigation/menu.go:Limit
pub fn limit(m: &Menu, n: usize) -> Menu {
    if m.len() > n {
        return m[0..n].to_vec();
    }
    m.clone()
}

/// Go: `Menu.ByWeight()` (cached: `menuSort.ByWeight`).
// Go: navigation/menu.go:ByWeight
pub fn by_weight(m: &Menu) -> Menu {
    const KEY: &str = "menuSort.ByWeight";
    let (menus, _) = smc().get(
        KEY,
        &|m| menu_entry_by_sort(&default_menu_entry_sort, m),
        &[m],
    );

    menus
}

/// Go: `Menu.ByName()` (cached: `menuSort.ByName`).
// Go: navigation/menu.go:ByName
pub fn by_name(m: &Menu) -> Menu {
    const KEY: &str = "menuSort.ByName";
    let title = |m1: &MenuEntry, m2: &MenuEntry| {
        nh_common::compare::less_strings(m1.config.name.as_bytes(), m2.config.name.as_bytes())
    };

    let (menus, _) = smc().get(KEY, &|m| menu_entry_by_sort(&title, m), &[m]);

    menus
}

/// Go: `Menu.Reverse()` (cached: `menuSort.Reverse`).
// Go: navigation/menu.go:Reverse
pub fn reverse(m: &Menu) -> Menu {
    const KEY: &str = "menuSort.Reverse";
    let reverse_func = |menu: &mut Menu| menu.reverse();
    let (menus, _) = smc().get(KEY, &reverse_func, &[m]);

    menus
}

/// Go: `Menu.Clone()`.
// Go: navigation/menu.go:Clone
pub fn clone_menu(m: &Menu) -> Menu {
    m.clone()
}

/// Go: `navigation.DecodeConfig(in)` — the menus (`config`).
// Go: navigation/menu.go:DecodeConfig
pub fn decode_config(input: &Value) -> Result<BTreeMap<String, Menu>> {
    Ok(decode_config_namespace(input)?.config)
}

/// Go: `navigation.DecodeConfig(in)` — the whole `ConfigNamespace` (source structure: the
/// cleaned menus map; source hash of `in`).
// Go: navigation/menu.go:DecodeConfig
pub fn decode_config_namespace(
    input: &Value,
) -> Result<ConfigNamespace<Map, BTreeMap<String, Menu>>> {
    let build_config = |input: &Value| -> Result<(BTreeMap<String, Menu>, Option<Value>)> {
        let mut ret: BTreeMap<String, Menu> = BTreeMap::new();

        if input.is_nil() {
            return Ok((ret, Some(Value::map(Map::new(MapType::StringAny)))));
        }

        let menus = nh_common::maps::maps::to_string_map_e(input)?;
        let menus = nh_common::maps::params::clean_config_string_map(&menus);

        // Go iterates the map in random order; only which error is returned can differ.
        for (name, menu) in &menus.entries {
            let name = name.to_str_lossy().into_owned();
            let m = nh_common::cast::caste::to_slice_e(menu)?;
            for entry in &m {
                let mut menu_config = MenuConfig::default();
                menu_config.weak_decode(entry)?;
                if let Some(p) = menu_config.params.as_mut() {
                    nh_common::maps::params::prepare_params(p);
                }
                let mut menu_entry = MenuEntry {
                    menu: name.clone(),
                    config: menu_config,
                    ..Default::default()
                };
                menu_entry.configured_url = menu_entry.config.url.clone();

                let list = ret.entry(name.clone()).or_default();
                add(list, Arc::new(menu_entry));
            }
        }

        Ok((ret, Some(Value::map(menus))))
    };

    decode_namespace(input, build_config)
}

// ---------------------------------------------------------------------------
// Template values

/// Template value of `*navigation.MenuEntry`.
#[derive(Clone)]
pub struct MenuEntryRef(pub Arc<MenuEntry>);

nh_common::go_methods!(MenuEntryRef {
    "URL" => |m, _c, a| {
        args::exactly(a, 0, "URL")?;
        Ok(Value::string(m.0.url()))
    },
    "HasChildren" => |m, _c, a| {
        args::exactly(a, 0, "HasChildren")?;
        Ok(Value::Bool(m.0.has_children()))
    },
    "KeyName" => |m, _c, a| {
        args::exactly(a, 0, "KeyName")?;
        Ok(Value::string(m.0.key_name()))
    },
});

fn params_value(p: &Option<Map>) -> Value {
    match p {
        Some(m) => Value::map(m.clone()),
        None => Value::TypedNil(Arc::from("maps.Params")),
    }
}

impl MenuEntryRef {
    fn config_field(&self, name: &str) -> Option<Value> {
        let c = &self.0.config;
        Some(match name {
            "Identifier" => Value::string(c.identifier.clone()),
            "Parent" => Value::string(c.parent.clone()),
            "Name" => Value::string(c.name.clone()),
            "Pre" => Value::Safe(SafeKind::Html, c.pre.clone()),
            "Post" => Value::Safe(SafeKind::Html, c.post.clone()),
            "PageRef" => Value::string(c.page_ref.clone()),
            "Weight" => Value::int(c.weight),
            "Title" => Value::string(c.title.clone()),
            "Params" => params_value(&c.params),
            _ => return None,
        })
    }
}

impl Object for MenuEntryRef {
    nh_common::object_basics!(MENU_ENTRY_TYPE);

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "MenuConfig" => Some(Value::object(MenuConfigValue(self.0.config.clone()))),
            "Menu" => Some(Value::string(self.0.menu.clone())),
            "ConfiguredURL" => Some(Value::string(self.0.configured_url.clone())),
            "Page" => Some(
                self.0
                    .page
                    .clone()
                    .unwrap_or_else(|| Value::TypedNil(Arc::from("navigation.Page"))),
            ),
            "Children" => Some(menu_to_value_opt(&self.0.children, self.0.has_children())),
            // Promoted from the embedded MenuConfig (`URL` is shadowed by the method).
            _ => self.config_field(name),
        }
    }

    fn identity(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }
}

/// `navigation.MenuConfig` as a struct value (`.MenuConfig`).
#[derive(Clone)]
pub struct MenuConfigValue(pub MenuConfig);

nh_common::go_methods!(MenuConfigValue {});

impl Object for MenuConfigValue {
    nh_common::object_basics!("navigation.MenuConfig");

    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }

    fn field(&self, name: &str) -> Option<Value> {
        let c = &self.0;
        Some(match name {
            "Identifier" => Value::string(c.identifier.clone()),
            "Parent" => Value::string(c.parent.clone()),
            "Name" => Value::string(c.name.clone()),
            "Pre" => Value::Safe(SafeKind::Html, c.pre.clone()),
            "Post" => Value::Safe(SafeKind::Html, c.post.clone()),
            "URL" => Value::string(c.url.clone()),
            "PageRef" => Value::string(c.page_ref.clone()),
            "Weight" => Value::int(c.weight),
            "Title" => Value::string(c.title.clone()),
            "Params" => params_value(&c.params),
            _ => return None,
        })
    }
}

/// `navigation.Menu` as a template value.
pub fn menu_to_value(m: &Menu) -> Value {
    Value::list(
        SliceType::Named(Arc::from(MENU_TYPE)),
        m.iter()
            .map(|e| Value::object(MenuEntryRef(e.clone())))
            .collect(),
    )
}

fn menu_to_value_opt(m: &Menu, non_nil: bool) -> Value {
    if !non_nil {
        return Value::TypedNil(Arc::from(MENU_TYPE));
    }
    menu_to_value(m)
}

/// `navigation.Menu` from a template value.
pub fn menu_from_value(v: &Value) -> Option<Menu> {
    match v {
        Value::List(l) if matches!(&l.ty, SliceType::Named(n) if &**n == MENU_TYPE) => l
            .items
            .iter()
            .map(|it| it.downcast::<MenuEntryRef>().map(|e| e.0.clone()))
            .collect(),
        Value::TypedNil(t) if &**t == MENU_TYPE => Some(Vec::new()),
        _ => None,
    }
}

/// `navigation.Menus` as a template value.
pub fn menus_to_value(m: &Menus) -> Value {
    let mut out = Map::new(MapType::Named(Arc::from(MENUS_TYPE)));
    for (k, v) in m.iter() {
        out.insert(GoString::from(k.as_str()), menu_to_value(v));
    }
    Value::map(out)
}

/// `navigation.PageMenus` as a template value.
pub fn page_menus_to_value(m: &PageMenus) -> Value {
    let mut out = Map::new(MapType::Named(Arc::from(PAGE_MENUS_TYPE)));
    for (k, v) in m {
        out.insert(
            GoString::from(k.as_str()),
            Value::object(MenuEntryRef(v.clone())),
        );
    }
    Value::map(out)
}

pub fn menu_has_method(name: &str) -> bool {
    matches!(
        name,
        "Add" | "Sort" | "Limit" | "ByWeight" | "ByName" | "Reverse" | "Clone"
    )
}

pub fn menu_call_method(
    _ctx: HostCtx<'_>,
    recv: &Value,
    name: &str,
    a: &[Value],
) -> Option<GoResult<Value>> {
    if !menu_has_method(name) {
        return None;
    }
    let m = menu_from_value(recv)?;
    Some((|| -> GoResult<Value> {
        match name {
            "Add" => {
                args::exactly(a, 1, name)?;
                let v = args::get(a, 0)?;
                let Some(me) = v.downcast::<MenuEntryRef>() else {
                    return Err(args::wrong_type(MENU_ENTRY_TYPE, &v));
                };
                let mut m = m.clone();
                add(&mut m, me.0.clone());
                Ok(menu_to_value(&m))
            }
            "Sort" => {
                args::exactly(a, 0, name)?;
                let mut m = m.clone();
                sort(&mut m);
                Ok(menu_to_value(&m))
            }
            "Limit" => {
                args::exactly(a, 1, name)?;
                let n = args::int(a, 0)?;
                if (m.len() as i64) > n {
                    if n < 0 {
                        return Err(go_value::Error::new(format!(
                            "runtime error: slice bounds out of range [:{n}]"
                        )));
                    }
                    return Ok(menu_to_value(&limit(&m, n as usize)));
                }
                Ok(recv.clone())
            }
            "ByWeight" => {
                args::exactly(a, 0, name)?;
                Ok(menu_to_value(&by_weight(&m)))
            }
            "ByName" => {
                args::exactly(a, 0, name)?;
                Ok(menu_to_value(&by_name(&m)))
            }
            "Reverse" => {
                args::exactly(a, 0, name)?;
                Ok(menu_to_value(&reverse(&m)))
            }
            _ => {
                args::exactly(a, 0, name)?;
                Ok(menu_to_value(&clone_menu(&m)))
            }
        }
    })())
}

/// Methods of `navigation.Menu`.
pub const MENU_METHODS: NamedMethods = NamedMethods {
    has_method: menu_has_method,
    call: menu_call_method,
};

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: navigation/menu.go (318 lines; 7/20 funcs executed)
//   types: MenuEntry, Page, Menu, Menus, PageMenus, MenuConfig, menuSorter, menuEntryBy
// OK L53-66: (m *MenuEntry) URL() string
// OK L69-80: SetPageValues(m *MenuEntry, p Page)
// OK L106-108: (m *MenuEntry) HasChildren() bool
// OK L111-116: (m *MenuEntry) KeyName() string
// OK L118-126: (m *MenuEntry) hopefullyUniqueID() string
// OK L129-131: (m *MenuEntry) isEqual(inme *MenuEntry) bool
// OK L135-141: (m *MenuEntry) isSameResource(inme *MenuEntry) bool
// OK L143-148: (m *MenuEntry) isSamePage(p Page) bool
// OK L168-173: (m Menu) Add(me *MenuEntry) Menu
// OK L188-194: (by menuEntryBy) Sort(menu Menu)
// OK L216-216: (ms *menuSorter) Len() int
// OK L217-217: (ms *menuSorter) Swap(i, j int)
// OK L220-220: (ms *menuSorter) Less(i, j int) bool
// OK L223-226: (m Menu) Sort() Menu
// OK L229-234: (m Menu) Limit(n int) Menu
// OK L237-242: (m Menu) ByWeight() Menu
// OK L245-254: (m Menu) ByName() Menu
// OK L257-267: (m Menu) Reverse() Menu
// OK L271-273: (m Menu) Clone() Menu
// OK L275-318: DecodeConfig(in any) (*config.ConfigNamespace[map[string]MenuConfig, Menus], error)
// ---------------------------------------------------------------------------

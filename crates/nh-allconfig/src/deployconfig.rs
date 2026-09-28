//! Port of `deploy/deployconfig/deployConfig.go`.
//!
//! STUB (decode only)
//!
//! Owner: Wave B task T09 (allconfig-modules).
//!
//! The deployment configuration is decoded and validated like Go (it is part of the site
//! config and of `hugo config`); `hugo deploy` itself is not ported.

use nh_common::glob::glob::Glob;
use nh_common::{Error, Result};
use nh_config::config_provider::Provider;
use nh_config::decode::FieldRef;
use nh_config::decode_struct;
use nh_config::goregexp::Regexp;

use crate::allconfig::GoSlice;

/// Go: `deployconfig.DeploymentConfigKey`.
pub const DEPLOYMENT_CONFIG_KEY: &str = "deployment";

/// Go: `deployconfig.DeployConfig` — the complete configuration for deployment.
#[derive(Clone, Debug, Default)]
pub struct DeployConfig {
    pub targets: GoSlice<Target>,
    pub matchers: GoSlice<Matcher>,
    pub order: GoSlice<String>,

    /// Usually set via flags. Target deployment Name; defaults to the first one.
    pub target: String,
    /// Show a confirm prompt before deploying.
    pub confirm: bool,
    /// DryRun will try the deployment without any remote changes.
    pub dry_run: bool,
    /// Force will re-upload all files.
    pub force: bool,
    /// Invalidate the CDN cache listed in the deployment target.
    pub invalidate_cdn: bool,
    /// MaxDeletes is the maximum number of files to delete.
    pub max_deletes: i64,
    /// Number of concurrent workers to use when uploading files.
    pub workers: i64,

    /// Compiled Order (`json:"-"`).
    pub ordering: Vec<Regexp>,
}

decode_struct!(DeployConfig, "deployconfig.DeployConfig", |s| vec![
    FieldRef::new("Targets", &mut s.targets),
    FieldRef::new("Matchers", &mut s.matchers),
    FieldRef::new("Order", &mut s.order),
    FieldRef::new("Target", &mut s.target),
    FieldRef::new("Confirm", &mut s.confirm),
    FieldRef::new("DryRun", &mut s.dry_run),
    FieldRef::new("Force", &mut s.force),
    FieldRef::new("InvalidateCDN", &mut s.invalidate_cdn),
    FieldRef::new("MaxDeletes", &mut s.max_deletes),
    FieldRef::new("Workers", &mut s.workers),
]);

/// Go: `deployconfig.Target`.
#[derive(Clone, Debug, Default)]
pub struct Target {
    pub name: String,
    pub url: String,
    pub cloud_front_distribution_id: String,
    /// GoogleCloudCDNOrigin specifies the Google Cloud project and CDN origin to invalidate
    /// when deploying this target. It is specified as <project>/<origin>.
    pub google_cloud_cdn_origin: String,
    /// Optional patterns of files to include/exclude for this target.
    pub include: String,
    pub exclude: String,
    /// Parsed versions of Include/Exclude (`json:"-"`).
    pub include_glob: Option<Glob>,
    pub exclude_glob: Option<Glob>,
    /// If true, any local path matching <dir>/index.html will be mapped to the remote path
    /// <dir>/.
    pub strip_index_html: bool,
}

decode_struct!(Target, "deployconfig.Target", |s| vec![
    FieldRef::new("Name", &mut s.name),
    FieldRef::new("URL", &mut s.url),
    FieldRef::new(
        "CloudFrontDistributionID",
        &mut s.cloud_front_distribution_id
    ),
    FieldRef::new("GoogleCloudCDNOrigin", &mut s.google_cloud_cdn_origin),
    FieldRef::new("Include", &mut s.include),
    FieldRef::new("Exclude", &mut s.exclude),
    FieldRef::new("StripIndexHTML", &mut s.strip_index_html),
]);

impl Target {
    fn is_zero(&self) -> bool {
        self.name.is_empty()
            && self.url.is_empty()
            && self.cloud_front_distribution_id.is_empty()
            && self.google_cloud_cdn_origin.is_empty()
            && self.include.is_empty()
            && self.exclude.is_empty()
            && self.include_glob.is_none()
            && self.exclude_glob.is_none()
            && !self.strip_index_html
    }

    // Go: deploy/deployconfig/deployConfig.go:ParseIncludeExclude
    pub fn parse_include_exclude(&mut self) -> Result<()> {
        if !self.include.is_empty() {
            self.include_glob = Some(nh_common::glob::glob::get_glob(&self.include).map_err(
                |err| {
                    Error::new(format!(
                        "invalid deployment.target.include {}: {}",
                        go_strconv::quote(self.include.as_bytes()),
                        err
                    ))
                },
            )?);
        }
        if !self.exclude.is_empty() {
            self.exclude_glob = Some(nh_common::glob::glob::get_glob(&self.exclude).map_err(
                |err| {
                    Error::new(format!(
                        "invalid deployment.target.exclude {}: {}",
                        go_strconv::quote(self.exclude.as_bytes()),
                        err
                    ))
                },
            )?);
        }
        Ok(())
    }
}

/// Go: `deployconfig.Matcher` — configuration to be applied to files whose paths match a
/// specified pattern.
#[derive(Clone, Debug, Default)]
pub struct Matcher {
    pub pattern: String,
    pub cache_control: String,
    pub content_encoding: String,
    pub content_type: String,
    pub gzip: bool,
    pub force: bool,
    /// Pattern compiled (`json:"-"`).
    pub re: Option<Regexp>,
}

decode_struct!(Matcher, "deployconfig.Matcher", |s| vec![
    FieldRef::new("Pattern", &mut s.pattern),
    FieldRef::new("CacheControl", &mut s.cache_control),
    FieldRef::new("ContentEncoding", &mut s.content_encoding),
    FieldRef::new("ContentType", &mut s.content_type),
    FieldRef::new("Gzip", &mut s.gzip),
    FieldRef::new("Force", &mut s.force),
]);

impl Matcher {
    fn is_zero(&self) -> bool {
        self.pattern.is_empty()
            && self.cache_control.is_empty()
            && self.content_encoding.is_empty()
            && self.content_type.is_empty()
            && !self.gzip
            && !self.force
            && self.re.is_none()
    }

    // Go: deploy/deployconfig/deployConfig.go:Matches
    pub fn matches(&self, path: &str) -> bool {
        self.re.as_ref().is_some_and(|re| re.match_string(path))
    }
}

/// Go: `deployconfig.DefaultConfig`.
pub fn default_config() -> DeployConfig {
    DeployConfig {
        workers: 10,
        invalidate_cdn: true,
        max_deletes: 256,
        ..Default::default()
    }
}

/// DecodeConfig creates a config from a given Hugo configuration.
// Go: deploy/deployconfig/deployConfig.go:DecodeConfig
pub fn decode_config(cfg: &dyn Provider) -> Result<DeployConfig> {
    let mut dcfg = default_config();

    if !cfg.is_set(DEPLOYMENT_CONFIG_KEY) {
        return Ok(dcfg);
    }
    let m = crate::allconfig::get_string_map_opt(cfg, DEPLOYMENT_CONFIG_KEY)
        .unwrap_or_else(|| go_value::Map::new(go_value::MapType::StringAny));
    nh_config::decode::weak_decode_into(&go_value::Value::map(m), &mut dcfg)?;

    if dcfg.workers <= 0 {
        dcfg.workers = 10;
    }

    if let Some(targets) = dcfg.targets.0.as_mut() {
        for tgt in targets.iter_mut() {
            if tgt.is_zero() {
                return Err(Error::new("empty deployment target"));
            }
            tgt.parse_include_exclude()?;
        }
    }
    if let Some(matchers) = dcfg.matchers.0.as_mut() {
        for m in matchers.iter_mut() {
            if m.is_zero() {
                return Err(Error::new("empty deployment matcher"));
            }
            match Regexp::compile(&m.pattern) {
                Ok(re) => m.re = Some(re),
                Err(err) => {
                    return Err(Error::new(format!(
                        "invalid deployment.matchers.pattern: {err}"
                    )));
                }
            }
        }
    }
    let order: Vec<String> = dcfg.order.iter().cloned().collect();
    for o in order {
        match Regexp::compile(&o) {
            Ok(re) => dcfg.ordering.push(re),
            Err(err) => {
                return Err(Error::new(format!(
                    "invalid deployment.orderings.pattern: {err}"
                )));
            }
        }
    }

    Ok(dcfg)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: deploy/deployconfig/deployConfig.go (not found in signature dump)
// OK L78-94: (tgt *Target) ParseIncludeExclude() error
// OK L128-130: (m *Matcher) Matches(path string) bool
// OK L139-181: DecodeConfig(cfg config.Provider) (DeployConfig, error)
// ---------------------------------------------------------------------------

//! Ports of Go test tables of the packages owned by this crate
//! (deploy/deployconfig/deployConfig_test.go).

use nh_allconfig::deployconfig::decode_config;
use nh_config::config_loader::from_config_string;
use nh_config::default_config_provider::DefaultConfigProvider;

// Go: deploy/deployconfig/deployConfig_test.go:TestDecodeConfigFromTOML
#[test]
fn decode_config_from_toml() {
    let toml_config = r#"

someOtherValue = "foo"

[deployment]

order = ["o1", "o2"]

# All lowercase.
[[deployment.targets]]
name = "name0"
url = "url0"
cloudfrontdistributionid = "cdn0"
include = "*.html"

# All uppercase.
[[deployment.targets]]
NAME = "name1"
URL = "url1"
CLOUDFRONTDISTRIBUTIONID = "cdn1"
INCLUDE = "*.jpg"

# Camelcase.
[[deployment.targets]]
name = "name2"
url = "url2"
cloudFrontDistributionID = "cdn2"
exclude = "*.png"

# All lowercase.
[[deployment.matchers]]
pattern = "^pattern0$"
cachecontrol = "cachecontrol0"
contentencoding = "contentencoding0"
contenttype = "contenttype0"

# All uppercase.
[[deployment.matchers]]
PATTERN = "^pattern1$"
CACHECONTROL = "cachecontrol1"
CONTENTENCODING = "contentencoding1"
CONTENTTYPE = "contenttype1"
GZIP = true
FORCE = true

# Camelcase.
[[deployment.matchers]]
pattern = "^pattern2$"
cacheControl = "cachecontrol2"
contentEncoding = "contentencoding2"
contentType = "contenttype2"
gzip = true
force = true
"#;
    let cfg = from_config_string(toml_config, "toml").unwrap();
    let dcfg = decode_config(&cfg).unwrap();

    // Order.
    let order: Vec<&String> = dcfg.order.iter().collect();
    assert_eq!(order, ["o1", "o2"]);
    assert_eq!(dcfg.ordering.len(), 2);

    // Targets.
    let targets: Vec<_> = dcfg.targets.iter().collect();
    assert_eq!(targets.len(), 3);
    let want_include = ["*.html", "*.jpg", ""];
    let want_exclude = ["", "", "*.png"];
    for (i, tgt) in targets.iter().enumerate() {
        assert_eq!(tgt.name, format!("name{i}"));
        assert_eq!(tgt.url, format!("url{i}"));
        assert_eq!(tgt.cloud_front_distribution_id, format!("cdn{i}"));
        assert_eq!(tgt.include, want_include[i]);
        if !want_include[i].is_empty() {
            assert!(tgt.include_glob.is_some());
        }
        assert_eq!(tgt.exclude, want_exclude[i]);
        if !want_exclude[i].is_empty() {
            assert!(tgt.exclude_glob.is_some());
        }
    }

    // Matchers.
    let matchers: Vec<_> = dcfg.matchers.iter().collect();
    assert_eq!(matchers.len(), 3);
    for (i, m) in matchers.iter().enumerate() {
        assert_eq!(m.pattern, format!("^pattern{i}$"));
        assert!(m.re.is_some());
        assert_eq!(m.cache_control, format!("cachecontrol{i}"));
        assert_eq!(m.content_encoding, format!("contentencoding{i}"));
        assert_eq!(m.content_type, format!("contenttype{i}"));
        assert_eq!(m.gzip, i != 0);
        assert_eq!(m.force, i != 0);
        assert!(m.matches(&format!("pattern{i}")));
    }
}

// Go: deploy/deployconfig/deployConfig_test.go:TestInvalidOrderingPattern
#[test]
fn invalid_ordering_pattern() {
    let cfg = from_config_string(
        r#"

someOtherValue = "foo"

[deployment]
order = ["["]  # invalid regular expression
"#,
        "toml",
    )
    .unwrap();
    assert!(decode_config(&cfg).is_err());
}

// Go: deploy/deployconfig/deployConfig_test.go:TestInvalidMatcherPattern
#[test]
fn invalid_matcher_pattern() {
    let cfg = from_config_string(
        r#"

someOtherValue = "foo"

[deployment]
[[deployment.matchers]]
Pattern = "["  # invalid regular expression
"#,
        "toml",
    )
    .unwrap();
    assert!(decode_config(&cfg).is_err());
}

// Go: deploy/deployconfig/deployConfig_test.go:TestDecodeConfigDefault
#[test]
fn decode_config_default() {
    let dcfg = decode_config(&DefaultConfigProvider::new()).unwrap();
    assert_eq!(dcfg.targets.len(), 0);
    assert_eq!(dcfg.matchers.len(), 0);
    assert_eq!(dcfg.workers, 10);
    assert_eq!(dcfg.max_deletes, 256);
    assert!(dcfg.invalidate_cdn);
}

// Go: deploy/deployconfig/deployConfig_test.go:TestEmptyTarget
#[test]
fn empty_target() {
    let cfg = from_config_string("\n[deployment]\n[[deployment.targets]]\n", "toml").unwrap();
    assert_eq!(
        decode_config(&cfg).unwrap_err().to_string(),
        "empty deployment target"
    );
}

// Go: deploy/deployconfig/deployConfig_test.go:TestEmptyMatcher
#[test]
fn empty_matcher() {
    let cfg = from_config_string("\n[deployment]\n[[deployment.matchers]]\n", "toml").unwrap();
    assert_eq!(
        decode_config(&cfg).unwrap_err().to_string(),
        "empty deployment matcher"
    );
}

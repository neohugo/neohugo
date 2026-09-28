//! The `*resources.resourceAdapter` template method table (`ResourceRef` -> `tpl_call_method`)
//! agrees with the Rust API the `resources` oracle checks, over the synthetic site's assets;
//! image methods on non-image resources give Go's panic messages; the post-publish resource's
//! object gives its placeholders.

mod support;

use std::sync::Arc;

use go_value::Value;
use nh_resource::resourcetypes::Resource;
use nh_resources::resource::ResourceSourceDescriptor;
use serde_json::Value as J;
use support::*;

fn call(v: &Value, name: &str, args: &[Value]) -> Result<Value, String> {
    let o = v.as_object().unwrap();
    assert!(o.has_method(name), "{name}");
    let ctx = nh_tpl::template::TplContext::default();
    o.call_method(ctx.as_host(), name, args)
        .unwrap()
        .map_err(|e| e.message().to_string())
}

fn s(v: Result<Value, String>) -> String {
    v.unwrap()
        .as_go_string()
        .unwrap()
        .to_str_lossy()
        .into_owned()
}

#[test]
fn resource_adapter_methods() {
    let fx = fixture("resources/synth.json.gz");
    let root = repo_root();
    let dir = root.join("crates/nh-resources/tests/fixtures/site");
    let site = load_site(&dir.to_string_lossy(), None);

    let mut n = 0;
    for rc in fx["records"].as_array().unwrap() {
        if rc["kind"] != "asset" {
            continue;
        }
        let rd = &rc["rd"];
        let mut d = ResourceSourceDescriptor {
            open_read_seek_closer: Some(open_file(root.join(rd["file"].as_str().unwrap()))),
            name_normalized: rd["nameNormalized"].as_str().unwrap().to_string(),
            name_original: rd["nameOriginal"].as_str().unwrap().to_string(),
            title: rd["title"].as_str().unwrap().to_string(),
            target_path: rd["targetPath"].as_str().unwrap().to_string(),
            lazy_publish: true,
            ..Default::default()
        };
        let r = site.specs[0].new_resource_adapter(&mut d).unwrap();
        let v = r.clone().to_value();
        let o = v.as_object().unwrap();
        assert_eq!(o.type_name(), "*resources.resourceAdapter");
        let want = &rc["base"];
        let what = rd["file"].as_str().unwrap();
        assert_eq!(J::String(s(call(&v, "Name", &[]))), want["name"], "{what}");
        assert_eq!(
            J::String(s(call(&v, "Title", &[]))),
            want["title"],
            "{what}"
        );
        assert_eq!(J::String(s(call(&v, "Key", &[]))), want["key"], "{what}");
        assert_eq!(
            J::String(s(call(&v, "ResourceType", &[]))),
            want["resourceType"],
            "{what}"
        );
        assert_eq!(
            J::String(s(call(&v, "RelPermalink", &[]))),
            want["relPermalink"],
            "{what}"
        );
        assert_eq!(
            J::String(s(call(&v, "Permalink", &[]))),
            want["permalink"],
            "{what}"
        );
        assert_eq!(
            J::String(s(call(&v, "String", &[]))),
            want["name"],
            "{what}"
        );
        assert_eq!(
            o.go_string().unwrap().to_str_lossy(),
            want["name"].as_str().unwrap()
        );
        let mt = call(&v, "MediaType", &[]).unwrap();
        assert_eq!(mt.as_object().unwrap().type_name(), "media.Type");
        let params = call(&v, "Params", &[]).unwrap();
        assert_eq!(params.go_type_name(), "maps.Params");
        let data = call(&v, "Data", &[]).unwrap();
        assert_eq!(
            String::from_utf8(go_json::marshal(&data).unwrap()).unwrap(),
            want["data"].as_str().unwrap()
        );
        if let Some(c) = want.get("content") {
            assert_eq!(J::String(s(call(&v, "Content", &[]))), *c, "{what}");
        }
        // The resource from the value (what the template functions receive).
        let back = nh_resource::resourcetypes::resource_from_value_any(&v).unwrap();
        assert!(Arc::ptr_eq(&(back.as_any_arc()), &(r.clone().as_any_arc())));

        match want["resourceType"].as_str().unwrap() {
            // GIF sizes: nh-images does not decode GIF configs (see the resources test).
            "image" if want["mediaType"] == "image/gif" => {}
            "image" if want.get("width").is_some() => {
                assert_eq!(
                    call(&v, "Width", &[]).unwrap(),
                    Value::int(want["width"].as_i64().unwrap())
                );
                assert_eq!(
                    call(&v, "Height", &[]).unwrap(),
                    Value::int(want["height"].as_i64().unwrap())
                );
                let resized = call(&v, "Resize", &[Value::string("10x")]).unwrap();
                assert_eq!(
                    resized.as_object().unwrap().type_name(),
                    "*resources.resourceAdapter"
                );
                assert!(nh_resources::transform::image_source_from_value(&v).is_some());
            }
            "image" => {
                // SVG: Go panics with the raster-image message.
                let e = call(&v, "Width", &[]).unwrap_err();
                assert_eq!(J::String(e), want["whPanic"], "{what}");
            }
            _ => {
                let e = call(&v, "Resize", &[Value::string("10x")]).unwrap_err();
                assert_eq!(e, "this method is only available for image resources");
                assert!(nh_resources::transform::image_source_from_value(&v).is_some());
            }
        }
        n += 1;
    }
    assert!(n >= 20, "{n}");
}

#[test]
fn post_publish_methods() {
    let root = repo_root();
    let dir = root.join("crates/nh-resources/tests/fixtures/site");
    let site = load_site(&dir.to_string_lossy(), None);
    let mut d = ResourceSourceDescriptor {
        open_read_seek_closer: Some(open_file(dir.join("assets/css/a.css"))),
        target_path: "/css/a.css".to_string(),
        lazy_publish: true,
        ..Default::default()
    };
    let spec = site.specs[0].clone();
    let r = spec.new_resource_adapter(&mut d).unwrap();
    let pp = spec.post_process(r.clone() as Arc<dyn Resource>).unwrap();
    let pp2 = spec.post_process(r as Arc<dyn Resource>).unwrap();
    assert!(Arc::ptr_eq(&pp, &pp2));
    assert_eq!(pp.prefix, "__h_pp_l1_1_");
    let v = pp.to_value();
    assert_eq!(
        v.as_object().unwrap().type_name(),
        "*postpub.PostPublishResource"
    );
    assert_eq!(s(call(&v, "Content", &[])), "__h_pp_l1_1_Content__e=");
    assert_eq!(
        s(call(&v, "RelPermalink", &[])),
        "__h_pp_l1_1_RelPermalink__e="
    );
    assert_eq!(
        call(&v, "Params", &[]).unwrap_err(),
        "method .Params is currently not supported in post-publish transformations."
    );
    let origin = call(&v, "Origin", &[]).unwrap();
    assert_eq!(s(call(&origin, "Name", &[])), "/css/a.css");
    // The value is not a resource.Resource (its MediaType is a map in Go).
    assert!(nh_resource::resourcetypes::resource_ref_from_value(&v).is_none());
}

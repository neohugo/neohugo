//! Image operations are queued: names and sizes are known at once.

use crate::support;

#[test]
fn resize_fill_fit_crop_process() {
    let site = support::load();
    let s = site.home_scope();
    let r = |src: &str| site.render(src, &s);
    let logo = "get_asset(path=\"img/logo.png\")";
    let out = r(&format!(
        "{{% set i = {logo} | resize(width=35) %}}{{{{ i.width }}}}x{{{{ i.height }}}}|{{{{ i.rel_permalink }}}}"
    ));
    assert!(
        out.starts_with("35x35|/sub/img/logo_hu_") && out.ends_with(".png"),
        "{out}"
    );
    assert_eq!(
        r(&format!(
            "{{% set i = {logo} | fill(width=20, height=10, anchor=\"top\") %}}{{{{ i.width }}}}x{{{{ i.height }}}}"
        )),
        "20x10"
    );
    assert_eq!(
        r(&format!(
            "{{% set i = {logo} | fit(width=100, height=10) %}}{{{{ i.width }}}}x{{{{ i.height }}}}"
        )),
        "10x10"
    );
    assert_eq!(
        r(&format!(
            "{{% set i = {logo} | crop(spec=\"30x20\") %}}{{{{ i.width }}}}x{{{{ i.height }}}}"
        )),
        "30x20"
    );
    let webp = r(&format!(
        "{{% set i = {logo} | process(spec=\"resize 20x webp q50\") %}}{{{{ i.width }}}}|{{{{ i.media_type.type }}}}"
    ));
    assert_eq!(webp, "20|image/webp");
    // The same operation is the same resource.
    assert_eq!(
        r(&format!(
            "{{% set a = {logo} | resize(width=35) %}}{{% set b = {logo} | resize(spec=\"35x\") %}}{{{{ a.rel_permalink == b.rel_permalink }}}}"
        )),
        "true"
    );
    let e = site
        .try_render(&format!("{{{{ {logo} | resize(width=0) }}}}"), &s)
        .expect_err("width 0");
    assert!(e.to_string().contains("positive integer"), "{e}");
    let e = site
        .try_render(
            &format!("{{{{ {logo} | resize(spec=\"fill 10x10\") }}}}"),
            &s,
        )
        .expect_err("action mismatch");
    assert!(e.to_string().contains("use `process`"), "{e}");
    let e = site
        .try_render("{{ get_asset(path=\"css/a.css\") | resize(width=10) }}", &s)
        .expect_err("not an image");
    assert!(e.to_string().contains("not an image"), "{e}");
}

#[test]
fn image_filter_with_overlays_and_exif() {
    let site = support::load();
    let s = site.home_scope();
    let out = site.render(
        "{% set logo = get_asset(path=\"img/logo.png\") %}{% set small = logo | resize(width=10) %}{% set i = logo | image_filter(filters=[{\"op\": \"grayscale\"}, {\"op\": \"overlay\", \"image\": small, \"x\": 1, \"y\": 2}, {\"op\": \"process\", \"spec\": \"resize 40x\"}]) %}{{ i.width }}|{{ i.rel_permalink }}",
        &s,
    );
    assert!(out.starts_with("40|/sub/img/logo_hu_"), "{out}");
    let e = site
        .try_render(
            "{{ get_asset(path=\"img/logo.png\") | image_filter(filters=[{\"op\": \"wobble\"}]) }}",
            &s,
        )
        .expect_err("unknown op");
    assert!(e.to_string().contains("image_filter(filters=)"), "{e}");
    // The PNG has no EXIF data.
    assert_eq!(
        site.render("{{ get_asset(path=\"img/logo.png\") | exif is none }}", &s),
        "true"
    );
    let e = site
        .try_render("{{ get_asset(path=\"img/logo.png\") | image_colors }}", &s)
        .expect_err("not implemented");
    assert!(e.to_string().contains("not implemented"), "{e}");
}

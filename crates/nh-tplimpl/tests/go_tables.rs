//! Go's unit tests of tpl/tplimpl ported (templatedescriptor_test.go), plus the small helpers:
//! needsBaseTemplate, removeLeadingBOM, the Category/SubCategory stringers, ParseInfo.

use nh_tplimpl::category::{Category, SubCategory, category_string, sub_category_string};
use nh_tplimpl::template_info::{ParseInfo, TEMPLATE_VERSION, default_parse_info};
use nh_tplimpl::templatedescriptor::{DescriptorHandler, TemplateDescriptor};
use nh_tplimpl::templates::{
    needs_base_template, needs_base_template_bytes, remove_leading_bom, remove_leading_bom_bytes,
};

fn d(kind: &str, layout: &str, lang: &str, of: &str, mt: &str) -> TemplateDescriptor {
    TemplateDescriptor {
        kind: kind.into(),
        layout_from_template: layout.into(),
        lang: lang.into(),
        output_format: of.into(),
        media_type: mt.into(),
        ..Default::default()
    }
}

// Go: tpl/tplimpl/templatedescriptor_test.go:TestTemplateDescriptorCompare
#[test]
fn template_descriptor_compare() {
    let dh = DescriptorHandler {
        default_content_language: String::new(),
        default_output_format: "html".into(),
    };

    let less = |category: Category,
                this: &TemplateDescriptor,
                other1: &TemplateDescriptor,
                other2: &TemplateDescriptor| {
        let result1 = dh.compare_descriptors(category, false, this, other1);
        let result2 = dh.compare_descriptors(category, false, this, other2);
        assert!(result1.w1 < result2.w1, "{result1:?} < {result2:?}");
    };

    let check =
        |category: Category, this: &TemplateDescriptor, other: &TemplateDescriptor, less: bool| {
            let result = dh.compare_descriptors(category, false, this, other);
            if less {
                assert!(result.w1 < 0, "{result:?}");
            } else {
                assert!(result.w1 >= 0, "{result:?}");
            }
        };

    check(
        Category::Baseof,
        &d("", "", "", "404", "text/html"),
        &d("", "", "", "html", "text/html"),
        false,
    );

    check(
        Category::Layout,
        &d("", "", "en", "404", "text/html"),
        &d("", "", "", "alias", "text/html"),
        true,
    );

    less(
        Category::Layout,
        &d("home", "list", "", "html", ""),
        &d("", "list", "", "html", ""),
        &d("home", "", "", "html", ""),
    );

    check(
        Category::Layout,
        &d("home", "list", "", "html", "text/html"),
        &d("home", "list", "", "myformat", "text/html"),
        false,
    );
}

#[test]
fn needs_base_template_cases() {
    for (s, want) in [
        ("", false),
        ("plain", false),
        ("{{ define \"main\" }}x{{ end }}", true),
        ("{{- define \"main\" }}x{{ end }}", true),
        ("{{-define \"main\" }}x{{ end }}", true),
        ("{{\tdefine \"main\" }}", true),
        ("  \n\t{{ define \"main\" }}", true),
        ("\u{a0}{{ define \"main\" }}", true),
        ("x{{ define \"main\" }}", false),
        ("{{ .Title }}{{ define \"main\" }}", false),
        ("{{/* comment */}}{{ define \"main\" }}", true),
        ("{{- /* comment */ -}}\n{{ define \"main\" }}", true),
        ("{{/* a */}} {{- /* b */ -}} {{ define \"x\" }}", true),
        ("{{/* unterminated {{ define \"x\" }}", false),
        ("{{ /* spaced comment */ }}{{ define \"x\" }}", false),
        ("{{ block \"main\" . }}{{ end }}", false),
        ("{{ definex }}", true),
        ("{{ `define` }}", false),
    ] {
        assert_eq!(needs_base_template(s), want, "{s:?}");
    }
    assert!(!needs_base_template_bytes(b"\xff{{ define \"x\" }}"));
}

#[test]
fn remove_leading_bom_cases() {
    assert_eq!(remove_leading_bom("\u{feff}abc"), "abc");
    assert_eq!(remove_leading_bom("abc"), "abc");
    // Go returns the string unchanged when the BOM is the only rune.
    assert_eq!(remove_leading_bom("\u{feff}"), "\u{feff}");
    assert_eq!(remove_leading_bom(""), "");
    assert_eq!(remove_leading_bom("\u{feff}\u{feff}x"), "\u{feff}x");
    assert_eq!(remove_leading_bom_bytes(b"\xff\xfeabc"), b"\xff\xfeabc");
    assert_eq!(remove_leading_bom_bytes(b"\xef\xbb\xbf\xff"), b"\xff");
}

#[test]
fn stringers() {
    assert_eq!(Category::Layout.string(), "CategoryLayout");
    assert_eq!(Category::Hugo.string(), "CategoryHugo");
    assert_eq!(Category::Shortcode.string(), "CategoryShortcode");
    assert_eq!(category_string(0), "Category(0)");
    assert_eq!(category_string(8), "Category(8)");
    assert_eq!(category_string(-1), "Category(-1)");
    assert_eq!(SubCategory::Main.string(), "SubCategoryMain");
    assert_eq!(SubCategory::Embedded.string(), "SubCategoryEmbedded");
    assert_eq!(SubCategory::Inline.string(), "SubCategoryInline");
    assert_eq!(sub_category_string(3), "SubCategory(3)");
    assert_eq!(sub_category_string(-2), "SubCategory(-2)");
}

#[test]
fn parse_info() {
    assert!(ParseInfo::default().is_zero());
    let d = default_parse_info();
    assert!(!d.is_zero());
    assert_eq!(d.config.version, TEMPLATE_VERSION);
    assert!(!d.is_inner && !d.has_return);
}

#[test]
fn descriptor_plus_v() {
    let mut x = d("page", "single", "en", "html", "text/html");
    x.is_plain_text = true;
    assert_eq!(
        x.go_string_plus_v(),
        "{Kind:page LayoutFromTemplate:single LayoutFromUser: OutputFormat:html MediaType:text/html Lang:en Variant1: Variant2: LayoutFromUserMustMatch:false IsPlainText:true AlwaysAllowPlainText:false}"
    );
    assert!(TemplateDescriptor::default().is_zero());
}

//! Port of `golang.org/x/net@v0.41.0/html/foreign.go` and `const.go`.

use super::node::{Document, NodeId, NodeType};
use super::token::Attribute;

// Go: html/foreign.go:adjustAttributeNames
pub fn adjust_attribute_names(aa: &mut [Attribute], name_map: fn(&[u8]) -> Option<&'static [u8]>) {
    for a in aa.iter_mut() {
        if let Some(new_name) = name_map(&a.key) {
            a.key = new_name.to_vec();
        }
    }
}

// Go: html/foreign.go:adjustForeignAttributes
pub fn adjust_foreign_attributes(aa: &mut [Attribute]) {
    for a in aa.iter_mut() {
        if a.key.is_empty() || a.key[0] != b'x' {
            continue;
        }
        match a.key.as_slice() {
            b"xlink:actuate" | b"xlink:arcrole" | b"xlink:href" | b"xlink:role" | b"xlink:show"
            | b"xlink:title" | b"xlink:type" | b"xml:base" | b"xml:lang" | b"xml:space"
            | b"xmlns:xlink" => {
                let j = a.key.iter().position(|&c| c == b':').expect("colon");
                a.namespace = a.key[..j].to_vec();
                a.key = a.key[j + 1..].to_vec();
            }
            _ => {}
        }
    }
}

// Go: html/foreign.go:htmlIntegrationPoint
pub fn html_integration_point(doc: &Document, n: NodeId) -> bool {
    let n = &doc.nodes[n];
    if n.typ != NodeType::Element {
        return false;
    }
    match n.namespace.as_slice() {
        b"math" => {
            if n.data == b"annotation-xml" {
                for a in &n.attr {
                    if a.key == b"encoding"
                        && (go_unicode::strings::equal_fold(&a.val, b"text/html")
                            || go_unicode::strings::equal_fold(&a.val, b"application/xhtml+xml"))
                    {
                        return true;
                    }
                }
            }
        }
        b"svg" => {
            if matches!(n.data.as_slice(), b"desc" | b"foreignObject" | b"title") {
                return true;
            }
        }
        _ => {}
    }
    false
}

// Go: html/foreign.go:mathMLTextIntegrationPoint
pub fn math_ml_text_integration_point(doc: &Document, n: NodeId) -> bool {
    let n = &doc.nodes[n];
    if n.namespace != b"math" {
        return false;
    }
    matches!(n.data.as_slice(), b"mi" | b"mo" | b"mn" | b"ms" | b"mtext")
}

/// Go: `breakout` — Section 12.2.6.5.
pub fn breakout(s: &[u8]) -> bool {
    matches!(
        s,
        b"b" | b"big"
            | b"blockquote"
            | b"body"
            | b"br"
            | b"center"
            | b"code"
            | b"dd"
            | b"div"
            | b"dl"
            | b"dt"
            | b"em"
            | b"embed"
            | b"h1"
            | b"h2"
            | b"h3"
            | b"h4"
            | b"h5"
            | b"h6"
            | b"head"
            | b"hr"
            | b"i"
            | b"img"
            | b"li"
            | b"listing"
            | b"menu"
            | b"meta"
            | b"nobr"
            | b"ol"
            | b"p"
            | b"pre"
            | b"ruby"
            | b"s"
            | b"small"
            | b"span"
            | b"strong"
            | b"strike"
            | b"sub"
            | b"sup"
            | b"table"
            | b"tt"
            | b"u"
            | b"ul"
            | b"var"
    )
}

/// Go: `svgTagNameAdjustments` — Section 12.2.6.5.
pub fn svg_tag_name_adjustments(s: &[u8]) -> Option<&'static [u8]> {
    Some(match s {
        b"altglyph" => b"altGlyph",
        b"altglyphdef" => b"altGlyphDef",
        b"altglyphitem" => b"altGlyphItem",
        b"animatecolor" => b"animateColor",
        b"animatemotion" => b"animateMotion",
        b"animatetransform" => b"animateTransform",
        b"clippath" => b"clipPath",
        b"feblend" => b"feBlend",
        b"fecolormatrix" => b"feColorMatrix",
        b"fecomponenttransfer" => b"feComponentTransfer",
        b"fecomposite" => b"feComposite",
        b"feconvolvematrix" => b"feConvolveMatrix",
        b"fediffuselighting" => b"feDiffuseLighting",
        b"fedisplacementmap" => b"feDisplacementMap",
        b"fedistantlight" => b"feDistantLight",
        b"feflood" => b"feFlood",
        b"fefunca" => b"feFuncA",
        b"fefuncb" => b"feFuncB",
        b"fefuncg" => b"feFuncG",
        b"fefuncr" => b"feFuncR",
        b"fegaussianblur" => b"feGaussianBlur",
        b"feimage" => b"feImage",
        b"femerge" => b"feMerge",
        b"femergenode" => b"feMergeNode",
        b"femorphology" => b"feMorphology",
        b"feoffset" => b"feOffset",
        b"fepointlight" => b"fePointLight",
        b"fespecularlighting" => b"feSpecularLighting",
        b"fespotlight" => b"feSpotLight",
        b"fetile" => b"feTile",
        b"feturbulence" => b"feTurbulence",
        b"foreignobject" => b"foreignObject",
        b"glyphref" => b"glyphRef",
        b"lineargradient" => b"linearGradient",
        b"radialgradient" => b"radialGradient",
        b"textpath" => b"textPath",
        _ => return None,
    })
}

/// Go: `mathMLAttributeAdjustments` — Section 12.2.6.1.
pub fn math_ml_attribute_adjustments(s: &[u8]) -> Option<&'static [u8]> {
    match s {
        b"definitionurl" => Some(b"definitionURL"),
        _ => None,
    }
}

/// Go: `svgAttributeAdjustments`.
pub fn svg_attribute_adjustments(s: &[u8]) -> Option<&'static [u8]> {
    Some(match s {
        b"attributename" => b"attributeName",
        b"attributetype" => b"attributeType",
        b"basefrequency" => b"baseFrequency",
        b"baseprofile" => b"baseProfile",
        b"calcmode" => b"calcMode",
        b"clippathunits" => b"clipPathUnits",
        b"diffuseconstant" => b"diffuseConstant",
        b"edgemode" => b"edgeMode",
        b"filterunits" => b"filterUnits",
        b"glyphref" => b"glyphRef",
        b"gradienttransform" => b"gradientTransform",
        b"gradientunits" => b"gradientUnits",
        b"kernelmatrix" => b"kernelMatrix",
        b"kernelunitlength" => b"kernelUnitLength",
        b"keypoints" => b"keyPoints",
        b"keysplines" => b"keySplines",
        b"keytimes" => b"keyTimes",
        b"lengthadjust" => b"lengthAdjust",
        b"limitingconeangle" => b"limitingConeAngle",
        b"markerheight" => b"markerHeight",
        b"markerunits" => b"markerUnits",
        b"markerwidth" => b"markerWidth",
        b"maskcontentunits" => b"maskContentUnits",
        b"maskunits" => b"maskUnits",
        b"numoctaves" => b"numOctaves",
        b"pathlength" => b"pathLength",
        b"patterncontentunits" => b"patternContentUnits",
        b"patterntransform" => b"patternTransform",
        b"patternunits" => b"patternUnits",
        b"pointsatx" => b"pointsAtX",
        b"pointsaty" => b"pointsAtY",
        b"pointsatz" => b"pointsAtZ",
        b"preservealpha" => b"preserveAlpha",
        b"preserveaspectratio" => b"preserveAspectRatio",
        b"primitiveunits" => b"primitiveUnits",
        b"refx" => b"refX",
        b"refy" => b"refY",
        b"repeatcount" => b"repeatCount",
        b"repeatdur" => b"repeatDur",
        b"requiredextensions" => b"requiredExtensions",
        b"requiredfeatures" => b"requiredFeatures",
        b"specularconstant" => b"specularConstant",
        b"specularexponent" => b"specularExponent",
        b"spreadmethod" => b"spreadMethod",
        b"startoffset" => b"startOffset",
        b"stddeviation" => b"stdDeviation",
        b"stitchtiles" => b"stitchTiles",
        b"surfacescale" => b"surfaceScale",
        b"systemlanguage" => b"systemLanguage",
        b"tablevalues" => b"tableValues",
        b"targetx" => b"targetX",
        b"targety" => b"targetY",
        b"textlength" => b"textLength",
        b"viewbox" => b"viewBox",
        b"viewtarget" => b"viewTarget",
        b"xchannelselector" => b"xChannelSelector",
        b"ychannelselector" => b"yChannelSelector",
        b"zoomandpan" => b"zoomAndPan",
        _ => return None,
    })
}

/// Go: `isSpecialElementMap` (const.go).
fn is_special_element_map(s: &[u8]) -> bool {
    matches!(
        s,
        b"address"
            | b"applet"
            | b"area"
            | b"article"
            | b"aside"
            | b"base"
            | b"basefont"
            | b"bgsound"
            | b"blockquote"
            | b"body"
            | b"br"
            | b"button"
            | b"caption"
            | b"center"
            | b"col"
            | b"colgroup"
            | b"dd"
            | b"details"
            | b"dir"
            | b"div"
            | b"dl"
            | b"dt"
            | b"embed"
            | b"fieldset"
            | b"figcaption"
            | b"figure"
            | b"footer"
            | b"form"
            | b"frame"
            | b"frameset"
            | b"h1"
            | b"h2"
            | b"h3"
            | b"h4"
            | b"h5"
            | b"h6"
            | b"head"
            | b"header"
            | b"hgroup"
            | b"hr"
            | b"html"
            | b"iframe"
            | b"img"
            | b"input"
            | b"keygen"
            | b"li"
            | b"link"
            | b"listing"
            | b"main"
            | b"marquee"
            | b"menu"
            | b"meta"
            | b"nav"
            | b"noembed"
            | b"noframes"
            | b"noscript"
            | b"object"
            | b"ol"
            | b"p"
            | b"param"
            | b"plaintext"
            | b"pre"
            | b"script"
            | b"section"
            | b"select"
            | b"source"
            | b"style"
            | b"summary"
            | b"table"
            | b"tbody"
            | b"td"
            | b"template"
            | b"textarea"
            | b"tfoot"
            | b"th"
            | b"thead"
            | b"title"
            | b"tr"
            | b"track"
            | b"ul"
            | b"wbr"
            | b"xmp"
    )
}

// Go: html/const.go:isSpecialElement
pub fn is_special_element(doc: &Document, element: NodeId) -> bool {
    let e = &doc.nodes[element];
    match e.namespace.as_slice() {
        b"" | b"html" => is_special_element_map(&e.data),
        b"math" => matches!(
            e.data.as_slice(),
            b"mi" | b"mo" | b"mn" | b"ms" | b"mtext" | b"annotation-xml"
        ),
        b"svg" => matches!(e.data.as_slice(), b"foreignObject" | b"desc" | b"title"),
        _ => false,
    }
}

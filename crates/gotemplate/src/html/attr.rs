//! Go: tpl/internal/go_templates/htmltemplate/attr.go

use go_unicode::strings;

use super::content::ContentType;

// Go: attr.go:attrTypeMap
/// attrTypeMap[n] describes the value of the given attribute.
/// If an attribute affects (or can mask) the encoding or interpretation of
/// other content, or affects the contents, idempotency, or credentials of a
/// network message, then the value in this map is contentTypeUnsafe.
/// This map is derived from HTML5, specifically
/// <https://www.w3.org/TR/html5/Overview.html#attributes-1>
/// as well as "%URI"-typed attributes from
/// <https://www.w3.org/TR/html4/index/attributes.html>
pub(crate) fn attr_type_map(name: &[u8]) -> Option<ContentType> {
    use ContentType::*;
    Some(match name {
        b"accept" => Plain,
        b"accept-charset" => Unsafe,
        b"action" => Url,
        b"alt" => Plain,
        b"archive" => Url,
        b"async" => Unsafe,
        b"autocomplete" => Plain,
        b"autofocus" => Plain,
        b"autoplay" => Plain,
        b"background" => Url,
        b"border" => Plain,
        b"checked" => Plain,
        b"cite" => Url,
        b"challenge" => Unsafe,
        b"charset" => Unsafe,
        b"class" => Plain,
        b"classid" => Url,
        b"codebase" => Url,
        b"cols" => Plain,
        b"colspan" => Plain,
        b"content" => Unsafe,
        b"contenteditable" => Plain,
        b"contextmenu" => Plain,
        b"controls" => Plain,
        b"coords" => Plain,
        b"crossorigin" => Unsafe,
        b"data" => Url,
        b"datetime" => Plain,
        b"default" => Plain,
        b"defer" => Unsafe,
        b"dir" => Plain,
        b"dirname" => Plain,
        b"disabled" => Plain,
        b"draggable" => Plain,
        b"dropzone" => Plain,
        b"enctype" => Unsafe,
        b"for" => Plain,
        b"form" => Unsafe,
        b"formaction" => Url,
        b"formenctype" => Unsafe,
        b"formmethod" => Unsafe,
        b"formnovalidate" => Unsafe,
        b"formtarget" => Plain,
        b"headers" => Plain,
        b"height" => Plain,
        b"hidden" => Plain,
        b"high" => Plain,
        b"href" => Url,
        b"hreflang" => Plain,
        b"http-equiv" => Unsafe,
        b"icon" => Url,
        b"id" => Plain,
        b"ismap" => Plain,
        b"keytype" => Unsafe,
        b"kind" => Plain,
        b"label" => Plain,
        b"lang" => Plain,
        b"language" => Unsafe,
        b"list" => Plain,
        b"longdesc" => Url,
        b"loop" => Plain,
        b"low" => Plain,
        b"manifest" => Url,
        b"max" => Plain,
        b"maxlength" => Plain,
        b"media" => Plain,
        b"mediagroup" => Plain,
        b"method" => Unsafe,
        b"min" => Plain,
        b"multiple" => Plain,
        b"name" => Plain,
        b"novalidate" => Unsafe,
        // Skip handler names from
        // https://www.w3.org/TR/html5/webappapis.html#event-handlers-on-elements,-document-objects,-and-window-objects
        // since we have special handling in attrType.
        b"open" => Plain,
        b"optimum" => Plain,
        b"pattern" => Unsafe,
        b"placeholder" => Plain,
        b"poster" => Url,
        b"profile" => Url,
        b"preload" => Plain,
        b"pubdate" => Plain,
        b"radiogroup" => Plain,
        b"readonly" => Plain,
        b"rel" => Unsafe,
        b"required" => Plain,
        b"reversed" => Plain,
        b"rows" => Plain,
        b"rowspan" => Plain,
        b"sandbox" => Unsafe,
        b"spellcheck" => Plain,
        b"scope" => Plain,
        b"scoped" => Plain,
        b"seamless" => Plain,
        b"selected" => Plain,
        b"shape" => Plain,
        b"size" => Plain,
        b"sizes" => Plain,
        b"span" => Plain,
        b"src" => Url,
        b"srcdoc" => Html,
        b"srclang" => Plain,
        b"srcset" => Srcset,
        b"start" => Plain,
        b"step" => Plain,
        b"style" => Css,
        b"tabindex" => Plain,
        b"target" => Plain,
        b"title" => Plain,
        b"type" => Unsafe,
        b"usemap" => Url,
        b"value" => Unsafe,
        b"width" => Plain,
        b"wrap" => Plain,
        b"xmlns" => Url,
        _ => return None,
    })
}

// Go: attr.go:attrType
/// attrType returns a conservative (upper-bound on authority) guess at the
/// type of the lowercase named attribute.
pub(crate) fn attr_type(name: &[u8]) -> ContentType {
    let mut name = name;
    if strings::has_prefix(name, b"data-") {
        // Strip data- so that custom attribute heuristics below are
        // widely applied.
        // Treat data-action as URL below.
        name = &name[5..];
    } else {
        let (prefix, short, ok) = strings::cut(name, b":");
        if ok {
            if prefix == b"xmlns" {
                return ContentType::Url;
            }
            // Treat svg:href and xlink:href as href below.
            name = short;
        }
    }
    if let Some(t) = attr_type_map(name) {
        return t;
    }
    // Treat partial event handler names as script.
    if strings::has_prefix(name, b"on") {
        return ContentType::Js;
    }

    // Heuristics to prevent "javascript:..." injection in custom
    // data attributes and custom attributes like g:tweetUrl.
    // https://www.w3.org/TR/html5/dom.html#embedding-custom-non-visible-data-with-the-data-*-attributes
    // "Custom data attributes are intended to store custom data
    //  private to the page or application, for which there are no
    //  more appropriate attributes or elements."
    // Developers seem to store URL content in data URLs that start
    // or end with "URI" or "URL".
    if strings::contains(name, b"src")
        || strings::contains(name, b"uri")
        || strings::contains(name, b"url")
    {
        return ContentType::Url;
    }
    ContentType::Plain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attr_types() {
        assert_eq!(attr_type(b"href"), ContentType::Url);
        assert_eq!(attr_type(b"data-href"), ContentType::Url);
        assert_eq!(attr_type(b"data-bs-target"), ContentType::Plain);
        assert_eq!(attr_type(b"data-ad-client"), ContentType::Plain);
        assert_eq!(attr_type(b"xmlns:foo"), ContentType::Url);
        assert_eq!(attr_type(b"svg:href"), ContentType::Url);
        assert_eq!(attr_type(b"onclick"), ContentType::Js);
        assert_eq!(attr_type(b"data-onx"), ContentType::Js);
        assert_eq!(attr_type(b"g:tweeturl"), ContentType::Url);
        assert_eq!(attr_type(b"content"), ContentType::Unsafe);
        assert_eq!(attr_type(b"style"), ContentType::Css);
        assert_eq!(attr_type(b"srcset"), ContentType::Srcset);
        assert_eq!(attr_type(b"srcdoc"), ContentType::Html);
        assert_eq!(attr_type(b"data-"), ContentType::Plain);
        assert_eq!(attr_type(b""), ContentType::Plain);
    }
}

//! `nh-config`: neohugo config (base), config/{security,privacy,services}, common/hexec, common/neohugo.
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

// Lints that fight a faithful port (README rule 11).
#![allow(
    clippy::too_many_arguments,
    clippy::new_without_default,
    clippy::type_complexity
)]

pub mod common_config;
pub mod config_loader;
pub mod config_provider;
pub mod decode;
pub mod default_config_provider;
pub mod env;
pub mod goregexp;
pub mod hexec;
pub mod namespace;
pub mod neohugo;
pub mod privacy;
pub mod security;
pub mod services;

/// Implements [`go_value::Object`] for a Go struct value (`Kind::Struct`, no methods): the
/// exported fields in declaration order (for `%v` and JSON), plus fields promoted from embedded
/// structs (reachable as `.Field`, not printed on their own).
#[macro_export]
macro_rules! struct_object {
    ($ty:ty, $go_type:expr, |$s:ident| [$(($name:expr, $val:expr)),* $(,)?] $(promoted [$(($pname:expr, $pval:expr)),* $(,)?])?) => {
        impl ::go_value::Object for $ty {
            fn type_name(&self) -> ::std::borrow::Cow<'_, str> {
                ::std::borrow::Cow::Borrowed($go_type)
            }
            fn kind(&self) -> ::go_value::Kind {
                ::go_value::Kind::Struct
            }
            fn has_method(&self, _name: &str) -> bool {
                false
            }
            fn call_method(
                &self,
                _ctx: ::go_value::HostCtx<'_>,
                _name: &str,
                _args: &[::go_value::Value],
            ) -> Option<::go_value::Result<::go_value::Value>> {
                None
            }
            #[allow(unused_variables)]
            fn field(&self, name: &str) -> Option<::go_value::Value> {
                let $s = self;
                $(if name == $name {
                    return Some($val);
                })*
                $($(if name == $pname {
                    return Some($pval);
                })*)?
                None
            }
            #[allow(unused_variables)]
            fn struct_fields(
                &self,
            ) -> Option<Vec<(::std::borrow::Cow<'_, str>, ::go_value::Value)>> {
                let $s = self;
                Some(vec![$((::std::borrow::Cow::Borrowed($name), $val)),*])
            }
            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }
        }
    };
}

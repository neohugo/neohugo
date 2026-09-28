//! Port of `commands/config.go`.
//!
//! Owner: Wave B task T25 (commands-cli).

use std::sync::Arc;

use nh_common::Result;
use nh_common::herrors::Error;

use crate::cobra::{CmdKind, CobraCommand};
use crate::commandeer::{ConfigKey, RootCommand};

/// Go: `commands.configCommand` — prints the resolved config of one language as
/// toml/yaml/json (debugging aid; not part of the byte-parity target).
pub struct ConfigCommand {
    pub format: String,
    pub lang: String,
    pub print_zero: bool,
}

/// Go: `newConfigCommand()` + `(*configCommand).Init` + `(*configMountsCommand).Init`.
// Go: commands/config.go:newConfigCommand
pub fn new_config_command() -> CobraCommand {
    let mut c = CobraCommand::new("config", "Display site configuration", CmdKind::Config);
    // Go: (*configCommand).Init
    c.local_flags.string_p(
        "format",
        "",
        "toml",
        "preferred file format (toml, yaml or json)",
    );
    c.local_flags.string_p(
        "lang",
        "",
        "",
        "the language to display config for. Defaults to the first language defined.",
    );
    c.local_flags.bool_p(
        "printZero",
        "",
        false,
        "include config options with zero values (e.g. false, 0, \"\") in the output",
    );
    crate::commandeer::apply_local_flags_build_config(&mut c.local_flags);

    // Go: (*configMountsCommand).Init
    let mut m = CobraCommand::new(
        "mounts",
        "Print the configured file mounts",
        CmdKind::ConfigMounts,
    );
    crate::commandeer::apply_local_flags_build_config(&mut m.local_flags);
    c.commands = vec![m];
    c
}

impl ConfigCommand {
    /// Go: `(*configCommand).Run`.
    // Go: commands/config.go:(*configCommand).Run
    pub fn run(&self, root: &RootCommand) -> Result<()> {
        let conf = root.config_from_provider(
            ConfigKey {
                counter: root
                    .config_version_id
                    .load(std::sync::atomic::Ordering::SeqCst),
                ignore_modules_does_not_exists: false,
            },
            crate::helpers::flags_to_new_cfg(&root.flags),
        )?;
        // Go: the config of --lang (or the first of LanguageConfigSlice), json.Encoder with
        // SetIndent("", "  ") and SetEscapeHTML(false) of the ReplacingJSONMarshaller.
        let buf = nh_allconfig::json::config_dump(&conf.configs, &self.lang, self.print_zero)?;

        let format = go_unicode::strings::to_lower_str(&self.format).into_owned();

        match format.as_str() {
            "json" => {
                let mut w = root.opts.stdout.lock().unwrap_or_else(|e| e.into_inner());
                let _ = w.write_all(buf.as_bytes());
            }
            _ => {
                // Go decodes the JSON to a map[string]interface{}, converts whole floats to ints
                // and encodes it again in the requested format.
                let mut m = match go_json::unmarshal(buf.as_bytes()) {
                    Ok(go_value::Value::Map(m)) => (*m).clone(),
                    Ok(_) => return Err(Error::new("config: not a JSON object")),
                    Err(e) => return Err(Error::new(e.to_string())),
                };
                nh_common::maps::maps::convert_float64_with_no_decimals_to_int(&mut m);
                let v = go_value::Value::Map(Arc::new(m));
                let mut out = Vec::new();
                match format.as_str() {
                    "yaml" => nh_parser::frontmatter::interface_to_config(
                        &v,
                        nh_parser::metadecoders::format::Format::Yaml,
                        &mut out,
                    )?,
                    "toml" => nh_parser::frontmatter::interface_to_config(
                        &v,
                        nh_parser::metadecoders::format::Format::Toml,
                        &mut out,
                    )?,
                    _ => {
                        return Err(Error::new(format!(
                            "unsupported format: {}",
                            go_strconv::quote(&format)
                        )));
                    }
                }
                let mut w = root.opts.stdout.lock().unwrap_or_else(|e| e.into_inner());
                let _ = w.write_all(&out);
            }
        }
        Ok(())
    }
}

/// Go: `commands.configMountsCommand`.
pub struct ConfigMountsCommand;

impl ConfigMountsCommand {
    /// Go: `(*configMountsCommand).Run` — JSON dump of module mounts (Go: `configModMounts.MarshalJSON`).
    // Go: commands/config.go:(*configMountsCommand).Run
    pub fn run(&self, root: &RootCommand) -> Result<()> {
        let conf = root.config_from_provider(
            ConfigKey {
                counter: root
                    .config_version_id
                    .load(std::sync::atomic::Ordering::SeqCst),
                ignore_modules_does_not_exists: false,
            },
            crate::helpers::flags_to_new_cfg(&root.flags),
        )?;
        if root.is_verbose() {
            // Go: the verbose form adds the module's meta params and hugoVersion.
            return Err(Error::new(
                "neohugo-rs: config mounts with --logLevel info/debug (verbose module dump) is not supported",
            ));
        }
        let out = nh_allconfig::json::mounts_dump(&conf.configs)?;
        let mut w = root.opts.stdout.lock().unwrap_or_else(|e| e.into_inner());
        let _ = w.write_all(out.as_bytes());
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: commands/config.go (239 lines; 7/12 funcs executed)
//   types: configCommand, configModMount, configModMounts, configMountsCommand
// OK L35-41: newConfigCommand() *configCommand
// OK L53-55: (c *configCommand) Commands() []simplecobra.Commander
// OK L57-59: (c *configCommand) Name() string
// OK L61-109: (c *configCommand) Run(ctx context.Context, cd *simplecobra.Commandeer, args []string) error (toml/yaml: nh-parser has no encoder, see PORTING.md)
// OK L111-124: (c *configCommand) Init(cd *simplecobra.Commandeer) error
// OK L126-128: (c *configCommand) PreRun(cd, runner *simplecobra.Commandeer) error
// OK L142-197: (m *configModMounts) MarshalJSON() ([]byte, error) (nh_allconfig::json::mounts_dump; verbose form: unsupported)
// OK L204-206: (c *configMountsCommand) Commands() []simplecobra.Commander
// OK L208-210: (c *configMountsCommand) Name() string
// OK L212-225: (c *configMountsCommand) Run(ctx context.Context, cd *simplecobra.Commandeer, args []string) error
// OK L227-234: (c *configMountsCommand) Init(cd *simplecobra.Commandeer) error
// OK L236-239: (c *configMountsCommand) PreRun(cd, runner *simplecobra.Commandeer) error
// ---------------------------------------------------------------------------

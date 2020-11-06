//! Parses the `--cfg` and `--check-cfg` options.
//! See RFC (TODO)

use rustc_ast as ast;
use rustc_ast::token;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_session::config::ErrorOutputType;
use rustc_session::early_error;
use rustc_session::parse::ParseSess;
use rustc_span::source_map::FileName;
use tracing::info;

pub struct CrateCfg {
    pub cfg: FxHashSet<(String, Option<String>)>,

    /// If this field is `None`, then we do not check condition names.
    /// If this field is `Some`, then we check condition names, and this value
    /// contains the set of legal condition names.
    pub valid_names: Option<FxHashSet<String>>,

    /// If an entry in this map exists, then value checking has been enabled for that key,
    /// and the map value contains the list of legal values.
    ///
    /// For example, if `values` contained an entry with key = `features`, then
    /// `values["features"].contains_key(value)` would serve to validate all uses of
    /// `#[cfg(feature = "...")]`.
    pub valid_values: FxHashMap<String, FxHashSet<String>>,
}

/// Parses strings provided as `--check-cfg` options into a CheckCfgSpec.
/// `names(n1, n2, ... n3)`: specifies valid condition names
/// `names()`: is valid, specifies no names, turns on checking
/// `values(foo, "a", "b", ... "z")`: specifies valid values for a key-value cfg
pub fn parse_cfgspecs(cfgspecs: Vec<String>, check_cfg_specs: Vec<String>) -> CrateCfg {
    rustc_span::with_default_session_globals(move || {
        let cfg_symbols = cfgspecs
            .into_iter()
            .map(|s| {
                let sess = ParseSess::with_silent_emitter();
                let filename = FileName::cfg_spec_source_code(&s);
                let mut parser =
                    rustc_parse::new_parser_from_source_str(&sess, filename, s.to_string());

                macro_rules! error {
                    ($reason: expr) => {
                        early_error(
                            ErrorOutputType::default(),
                            &format!(concat!("invalid `--cfg` argument: `{}` (", $reason, ")"), s),
                        );
                    };
                }

                match &mut parser.parse_meta_item() {
                    Ok(meta_item) if parser.token == token::Eof => {
                        if meta_item.path.segments.len() != 1 {
                            error!("argument key must be an identifier");
                        }
                        match &meta_item.kind {
                            ast::MetaItemKind::List(..) => {
                                error!(r#"expected `key` or `key="value"`"#);
                            }
                            ast::MetaItemKind::NameValue(lit) if !lit.kind.is_str() => {
                                error!("argument value must be a string");
                            }
                            ast::MetaItemKind::NameValue(..) | ast::MetaItemKind::Word => {
                                let ident = meta_item.ident().expect("multi-segment cfg key");
                                return (ident.name, meta_item.value_str());
                            }
                        }
                    }
                    Ok(..) => {}
                    Err(err) => err.cancel(),
                }

                error!(r#"expected `key` or `key="value"`"#);
            })
            .collect::<FxHashSet<(rustc_span::Symbol, Option<rustc_span::Symbol>)>>();
        let cfg = cfg_symbols
            .into_iter()
            .map(|(a, b)| (a.to_string(), b.map(|b| b.to_string())))
            .collect();

        let mut valid_names: Option<FxHashSet<String>> = None;
        let mut valid_values: FxHashMap<String, FxHashSet<String>> =
            FxHashMap::with_capacity_and_hasher(0, Default::default());

        for s in check_cfg_specs.iter() {
            let sess = ParseSess::with_silent_emitter();
            let filename = FileName::cfg_spec_source_code(&s);
            let mut parser =
                rustc_parse::new_parser_from_source_str(&sess, filename, s.to_string());

            macro_rules! error {
                ($reason: expr) => {
                    early_error(
                        ErrorOutputType::default(),
                        &format!(
                            concat!("invalid `--check-cfg` argument: `{}` (", $reason, ")"),
                            s
                        ),
                    );
                };
            }

            let meta_item = match parser.parse_meta_item() {
                Ok(m) => m,
                Err(mut e) => {
                    e.emit();
                    continue;
                }
            };

            if parser.token != token::Eof {
                error!(r#"expected no items after metadata"#);
            }

            let directive_kind = if let Some(t) = meta_item.ident() {
                t.as_str()
            } else {
                error!("first word should be either 'names' or 'values'");
            };

            if directive_kind == "names" {
                if let Some(items) = meta_item.meta_item_list() {
                    let valid_names = valid_names.get_or_insert_with(Default::default);
                    for item in items.iter() {
                        if let Some(name) = item.ident() {
                            info!("adding '{}' to list of valid condition names", name);
                            valid_names.insert(name.to_string());
                        } else {
                            error!("expected identifier");
                        }
                    }
                } else {
                    error!("expected list of valid names");
                }
            } else if directive_kind == "values" {
                if let Some(list) = meta_item.meta_item_list() {
                    // values(...)
                    // The first item in the list is required to be the name of the
                    // condition key.
                    let mut ii = list.iter();
                    if let Some(item0) = ii.next() {
                        if let Some(key_name) = item0.ident() {
                            // the rest of the items must be strings
                            let key_name_string = key_name.to_string();
                            info!("enabling checking for this set: {}", key_name_string);
                            let this_valid_values =
                                valid_values.entry(key_name_string).or_default();
                            for v in ii {
                                match v {
                                    ast::NestedMetaItem::Literal(ast::Lit {
                                        kind: ast::LitKind::Str(s, ast::StrStyle::Cooked),
                                        ..
                                    }) => {
                                        info!("inserting: {}", s);
                                        this_valid_values.insert(s.to_string());
                                    }
                                    _ => {
                                        error!("unexpected node in list");
                                    }
                                }
                            }
                        } else {
                            error!("needs to be an ident");
                        }
                    } else {
                        error!("must specify the condition key, e.g. 'feature', as an identifier (no quotes)");
                    }
                } else {
                    error!("expected list, containing condition name and list of valid values");
                }
            } else {
                error!(r"expected names(...) or values(...)");
            }
        }

        info!("cfg values: {:?}", cfg);
        info!("valid cfg names: {:?}", valid_names);
        info!("valid cfg values: {:?}", valid_values);

        CrateCfg { cfg, valid_names, valid_values }
    })
}

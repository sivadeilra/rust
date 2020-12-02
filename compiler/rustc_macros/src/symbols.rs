//! Proc macro which builds the Symbol table
//!
//! # Debugging
//!
//! Since this proc-macro does some non-trivial work, debugging it is important.
//! This proc-macro can be invoked as an ordinary unit test, like so:
//!
//! ```bash
//! cd compiler/rustc_macros
//! cargo test symbols::test_symbols -- --nocapture
//! ```
//!
//! This unit test finds the `symbols!` invocation in `compiler/rustc_span/src/symbol.rs`
//! and runs it. It verifies that the output token stream can be parsed as valid module
//! items and that no errors were produced.
//!
//! You can also view the generated code by using `cargo expand`:
//!
//! ```bash
//! cargo install cargo-expand          # this is necessary only once
//! cd compiler/rustc_span
//! cargo expand > /tmp/rustc_span.rs   # it's a big file
//! ```

use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::collections::HashMap;
use syn::parse::{Parse, ParseStream, Result};
use syn::{braced, punctuated::Punctuated, Ident, LitStr, Token};

#[cfg(test)]
mod tests;

mod kw {
    syn::custom_keyword!(Keywords);
    syn::custom_keyword!(Symbols);
    syn::custom_keyword!(Common);
}

struct Keyword {
    name: Ident,
    value: LitStr,
}

impl Parse for Keyword {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let value = input.parse()?;

        Ok(Keyword { name, value })
    }
}

struct Symbol {
    name: Ident,
    value: Option<LitStr>,
}

impl Parse for Symbol {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        let value = match input.parse::<Token![:]>() {
            Ok(_) => Some(input.parse()?),
            Err(_) => None,
        };

        Ok(Symbol { name, value })
    }
}

struct CommonWord {
    value: String,
    span: Span,
}

impl Parse for CommonWord {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let la = input.lookahead1();
        if la.peek(Ident) {
            let id: Ident = input.parse()?;
            Ok(Self { value: id.to_string(), span: id.span() })
        } else if la.peek(LitStr) {
            let str: LitStr = input.parse()?;
            Ok(Self { value: str.value(), span: str.span() })
        } else {
            Err(la.error())
        }
    }
}

struct Input {
    keywords: Punctuated<Keyword, Token![,]>,
    symbols: Punctuated<Symbol, Token![,]>,
    commons: Punctuated<CommonWord, Token![,]>,
}

impl Parse for Input {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<kw::Keywords>()?;
        let content;
        braced!(content in input);
        let keywords = Punctuated::parse_terminated(&content)?;

        input.parse::<kw::Symbols>()?;
        let content;
        braced!(content in input);
        let symbols = Punctuated::parse_terminated(&content)?;

        input.parse::<kw::Common>()?;
        let content;
        braced!(content in input);
        let commons = Punctuated::parse_terminated(&content)?;

        Ok(Input { keywords, symbols, commons })
    }
}

#[derive(Default)]
struct Errors {
    list: Vec<syn::Error>,
}

impl Errors {
    fn error(&mut self, span: Span, message: String) {
        self.list.push(syn::Error::new(span, message));
    }
}

/// Checks whether `s` contains exactly one character, and that character is ASCII.
/// If so, returns `Some(symbol_index)` where `symbol_index` is the symbol index of
/// that character.
fn is_ascii_symbol(s: &str) -> Option<u32> {
    let b = s.as_bytes();
    if b.len() == 1 {
        // All single-byte encodings of UTF-8 are ASCII.
        Some(b[0] as u32 + ASCII_SYMBOL_BASE)
    } else {
        None
    }
}

// Each 'Symbol' value can address either static values (known at compile time)
// or dynamic values (discovered at runtime). The static values use identifiers
// that are stored contiguously, numbering from 0 to NUM_SYMBOLS - 1. All values
// starting at NUM_SYMBOLS and higher are used for dynamic values.
//
// Within the set of static symbols, we choose the assignment of symbols in order
// to make some things easier.
//
//      * The empty string "" symbol has index 0.
//      * The ASCII character set (0x00 to 0x7f inclusive) are assigned immediately
//        after the empty string, and so use values 0x01 to 0x80. Converting
//        between the symbol index and ASCII requires adding/subtracting 1.

const ASCII_SYMBOL_BASE: u32 = 1;
const ASCII_SYMBOL_LEN: u32 = 0x80;

// This is the base symbol index for keyword/symbol/common strings,
// except for "" and ASCII single-character strings.
const STATIC_STRING_SYMBOL_BASE: u32 = ASCII_SYMBOL_BASE + ASCII_SYMBOL_LEN;

pub fn symbols(input: TokenStream) -> TokenStream {
    let (mut output, errors) = symbols_with_errors(input);

    // If we generated any errors, then report them as compiler_error!() macro calls.
    // This lets the errors point back to the most relevant span. It also allows us
    // to report as many errors as we can during a single run.
    output.extend(errors.into_iter().map(|e| e.to_compile_error()));

    output
}

fn symbols_with_errors(input: TokenStream) -> (TokenStream, Vec<syn::Error>) {
    let mut errors = Errors::default();

    let input: Input = match syn::parse2(input) {
        Ok(input) => input,
        Err(e) => {
            // This allows us to display errors at the proper span, while minimizing
            // unrelated errors caused by bailing out (and not generating code).
            errors.list.push(e);
            Input {
                keywords: Default::default(),
                symbols: Default::default(),
                commons: Default::default(),
            }
        }
    };

    let mut keyword_stream = quote! {};
    let mut symbols_stream = quote! {};
    let mut keys =
        HashMap::<String, Span>::with_capacity(input.keywords.len() + input.symbols.len() + 10);
    let mut prev_key: Option<(Span, String)> = None;

    let mut check_dup = |span: Span, str: &str, errors: &mut Errors| {
        if let Some(prev_span) = keys.get(str) {
            errors.error(span, format!("Symbol `{}` is duplicated", str));
            errors.error(*prev_span, format!("location of previous definition"));
        } else {
            keys.insert(str.to_string(), span);
        }
    };

    let mut check_order = |span: Span, str: &str, errors: &mut Errors| {
        if let Some((prev_span, ref prev_str)) = prev_key {
            if str < prev_str {
                errors.error(span, format!("Symbol `{}` must precede `{}`", str, prev_str));
                errors.error(prev_span, format!("location of previous symbol `{}`", prev_str));
            }
        }
        prev_key = Some((span, str.to_string()));
    };

    let mut symbol_names: Vec<String> = Vec::new();

    // Generate the listed keywords.
    for keyword in input.keywords.iter() {
        let name = &keyword.name;
        let value = &keyword.value;
        let value_string = value.value();
        check_dup(keyword.name.span(), &value_string, &mut errors);

        let symbol_index = if value_string.is_empty() {
            // Special case. Symbol index is always zero.
            0
        } else if let Some(ascii_symbol_index) = is_ascii_symbol(&value_string) {
            // This symbol is a single ASCII character.
            // It is represented differently.
            // We use the symbol index of the ASCII character, instead of assigning
            // a new symbol index.
            ascii_symbol_index
        } else {
            let symbol_index = STATIC_STRING_SYMBOL_BASE + symbol_names.len() as u32;
            symbol_names.push(value_string);
            symbol_index
        };

        keyword_stream.extend(quote! {
            pub const #name: Symbol = Symbol::new(#symbol_index);
        });
    }

    let non_keyword_symbol_base = STATIC_STRING_SYMBOL_BASE + symbol_names.len() as u32;

    // Generate the listed symbols.
    for symbol in input.symbols.iter() {
        let name = &symbol.name;
        let value = match &symbol.value {
            Some(value) => value.value(),
            None => name.to_string(),
        };
        check_dup(symbol.name.span(), &value, &mut errors);
        check_order(symbol.name.span(), &name.to_string(), &mut errors);

        // The empty string is covered by the Keywords { Invalid: "" } case.
        assert!(!value.is_empty());

        let symbol_index = if let Some(ascii_symbol_index) = is_ascii_symbol(&value) {
            // This symbol is a single ASCII character.
            // It is represented differently.
            ascii_symbol_index
        } else {
            let symbol_index = STATIC_STRING_SYMBOL_BASE + symbol_names.len() as u32;
            symbol_names.push(value);
            symbol_index
        };
        symbols_stream.extend(quote! {
            #[allow(rustc::default_hash_types)]
            #[allow(non_upper_case_globals)]
            pub const #name: Symbol = Symbol::new(#symbol_index);
        });
    }

    // Add common words. These are added to the static set of strings that
    // we recognize, but we do not define any symbol that points to the
    // symbol index.
    for common in input.commons.iter() {
        let common_value = &common.value;
        if is_ascii_symbol(common_value).is_some() {
            errors.error(common.span, format!("common string {:?} is unnecessary; all strings consisting of a single ASCII character are interned.", common_value));
            continue;
        }
        check_dup(common.span, common_value, &mut errors);
        symbol_names.push(common_value.to_string());
    }

    let symbol_names_len = symbol_names.len();

    let symbol_names_tokens: proc_macro2::TokenStream =
        symbol_names.iter().map(|s| quote!(#s,)).collect();

    // Build the PHF map. This translates from strings to Symbol values.
    let mut phf_map = phf_codegen::Map::<&str>::new();
    phf_map.entry("", "Symbol::new(0)");
    for (index, symbol) in symbol_names.iter().enumerate() {
        let real_symbol_index = STATIC_STRING_SYMBOL_BASE + index as u32;
        phf_map.entry(symbol, format!("Symbol::new({})", real_symbol_index).as_str());
    }
    let phf_map_built = phf_map.build();
    let phf_map_text = phf_map_built.to_string();
    let phf_map_expr = syn::parse_str::<syn::Expr>(&phf_map_text).unwrap();

    let mut output = quote! {
        macro_rules! keywords {
            () => {
                #keyword_stream
            }
        }

        macro_rules! define_symbols {
            () => {
                #symbols_stream
            }
        }

        const DIGITS_BASE_INDEX: u32 = ASCII_SYMBOL_BASE + '0' as u32;
        const ASCII_SYMBOL_BASE: u32 = #ASCII_SYMBOL_BASE;
        const ASCII_SYMBOL_LEN: u32 = #ASCII_SYMBOL_LEN;
        const STATIC_STRING_SYMBOL_BASE: u32 = #STATIC_STRING_SYMBOL_BASE;
        const DYNAMIC_SYMBOL_BASE: u32 = 1 + ASCII_SYMBOL_LEN + #symbol_names_len as u32;
        const NON_KEYWORD_SYMBOL_BASE: u32 = #non_keyword_symbol_base;

        pub static SYMBOL_NAMES: [&str; #symbol_names_len as usize] = [
            #symbol_names_tokens
                ];

        static STATIC_SYMBOLS_PHF: ::phf::Map<&'static str, Symbol> = #phf_map_expr;
    };

    // Generate the ASCII_STR static string.
    let mut ascii_str = String::with_capacity(0x80);
    for c in 0u8..0x80u8 {
        ascii_str.push(char::from(c));
        }
    output.extend(quote! {
        /// This string contains all of the characters of ASCII, in order.
        /// This is necessary for the implementation of `Symbol::as_str`.
        static ASCII_STR: &str = #ascii_str;
    });

    (output, errors.list)

    // To see the generated code, use the "cargo expand" command.
    // Do this once to install:
    //      cargo install cargo-expand
    //
    // Then, cd to rustc_span and run:
    //      cargo expand > /tmp/rustc_span_expanded.rs
    //
    // and read that file.
}

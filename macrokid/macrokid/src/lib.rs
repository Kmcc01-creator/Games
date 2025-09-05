use proc_macro::TokenStream;
use syn::{parse::{Parse, ParseStream}, parse_macro_input, ItemFn, LitBool, LitStr, Token, Ident};

// =====================
// Attribute macro: #[trace]  
// Wraps the function body to time execution and log duration
// =====================
#[proc_macro_attribute]
pub fn trace(attr: TokenStream, item: TokenStream) -> TokenStream {
    let func: ItemFn = parse_macro_input!(item as ItemFn);
    let args = parse_macro_input!(attr as TraceArgs);

    let mut cfg = macrokid_core::attr::trace::TraceConfig::default();
    if let Some(prefix) = args.prefix { cfg.prefix = prefix.value(); }
    if let Some(rel) = args.release { cfg.release = rel.value; }
    if let Some(logger) = args.logger {
        let s = logger.value();
        cfg.logger = match s.as_str() {
            "log" => macrokid_core::attr::trace::TraceConfig { logger: macrokid_core::attr::trace::TraceLogger::Log, ..cfg.clone() }.logger,
            _ => macrokid_core::attr::trace::TraceLogger::Eprintln,
        };
    }

    macrokid_core::attr::trace::expand_trace(func, cfg).into()
}

// =====================
// NOTE: Derive macros like Display are now demonstrated in the 
// examples/custom_derive crate to show how to use macrokid_core
// =====================

// =====================
// Function-like macro: make_enum!(Name: Foo, Bar, Baz)
// Generates an enum and basic Display + FromStr impls.
// =====================
#[proc_macro]
pub fn make_enum(input: TokenStream) -> TokenStream {
    let parsed_input: macrokid_core::function::make_enum::MakeEnumInput =
        parse_macro_input!(input as macrokid_core::function::make_enum::MakeEnumInput);

    macrokid_core::function::make_enum::expand_make_enum(parsed_input).into()
}

// --- Parsing for #[trace(...)] options ---
struct TraceArgs {
    prefix: Option<LitStr>,
    release: Option<LitBool>,
    logger: Option<LitStr>,
}

impl Parse for TraceArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut prefix = None;
        let mut release = None;
        let mut logger = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "prefix" => { prefix = Some(input.parse::<LitStr>()?); },
                "release" => { release = Some(input.parse::<LitBool>()?); },
                "logger" => { logger = Some(input.parse::<LitStr>()?); },
                _ => return Err(syn::Error::new_spanned(key, "unknown trace option")),
            }
            let _ = input.parse::<Token![,]>();
        }

        Ok(TraceArgs { prefix, release, logger })
    }
}

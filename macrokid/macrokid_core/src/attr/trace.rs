use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_quote, ItemFn};

/// Logger selection for trace output
#[derive(Clone, Copy, Debug)]
pub enum TraceLogger {
    Eprintln,
    Log,
}

/// Trace configuration
#[derive(Clone, Debug)]
pub struct TraceConfig {
    pub prefix: String,
    pub release: bool,
    pub logger: TraceLogger,
}

impl Default for TraceConfig {
    fn default() -> Self {
        Self {
            prefix: "[macrokid::trace]".to_string(),
            release: true,
            logger: TraceLogger::Eprintln,
        }
    }
}

/// Generate a tracing wrapper for a function using the provided config
pub fn expand_trace(mut func: ItemFn, cfg: TraceConfig) -> TokenStream2 {
    let name = func.sig.ident.to_string();
    let prefix = cfg.prefix;
    let orig_block = func.block;

    // Create unique variable names to avoid conflicts
    let start_var = format_ident!("__macrokid_trace_start_{}", func.sig.ident);
    let ret_var = format_ident!("__macrokid_trace_ret_{}", func.sig.ident);

    // Select logger
    let log_stmt = match cfg.logger {
        TraceLogger::Eprintln => quote! { eprintln!("{} {} took {:?}", #prefix, #name, #start_var.elapsed()); },
        TraceLogger::Log => quote! {
            #[cfg(feature = "log")]
            log::trace!("{} {} took {:?}", #prefix, #name, #start_var.elapsed());
            #[cfg(not(feature = "log"))]
            eprintln!("{} {} took {:?}", #prefix, #name, #start_var.elapsed());
        },
    };

    // Optionally gate in release builds
    let emit_stmt = if cfg.release {
        log_stmt
    } else {
        quote! { if cfg!(debug_assertions) { #log_stmt } }
    };

    // Replace the function body with a timed wrapper
    func.block = parse_quote!({
        let #start_var = ::std::time::Instant::now();
        let #ret_var = (|| #orig_block)();
        #emit_stmt
        #ret_var
    });

    quote!(#func)
}

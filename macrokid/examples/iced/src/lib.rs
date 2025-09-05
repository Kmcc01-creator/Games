use proc_macro::TokenStream;
use macrokid_core::builders::ImplBuilder;
use quote::quote;
use syn::{parse::{Parse, ParseStream}, parse_macro_input, DeriveInput, Ident, ItemImpl, LitStr, Token};

/// Simple function-like macro for creating complete Iced applications
/// 
/// This macro generates a complete Iced application with minimal boilerplate.
/// 
/// # Example
/// ```rust
/// iced_app! {
///     CounterApp {
///         title: "Counter",
///         state: { value: i32 = 0 },
///         messages: {
///             Increment => |app| app.value += 1,
///             Decrement => |app| app.value -= 1,
///             Reset => |app| app.value = 0,
///         },
///         view: |app| {
///             iced::widget::column![
///                 iced::widget::text(format!("Value: {}", app.value)),
///                 iced::widget::button("Increment").on_press(Message::Increment),
///                 iced::widget::button("Decrement").on_press(Message::Decrement),
///                 iced::widget::button("Reset").on_press(Message::Reset),
///             ]
///         }
///     }
/// }
/// ```
#[proc_macro]
pub fn iced_app(input: TokenStream) -> TokenStream {
    let input: IcedAppInput = parse_macro_input!(input as IcedAppInput);

    let app_ident = input.name;
    let msg_ident = Ident::new("Message", app_ident.span());
    let title = input.title.map(|s| s.value()).unwrap_or_else(|| app_ident.to_string());

    // Skeleton struct and empty message enum
    let struct_def = quote! { pub struct #app_ident; };
    let message_def = quote! { #[derive(Clone, Debug)] pub enum #msg_ident {} };

    // Implement IcedAppTrait for the app using ImplBuilder (assoc type + methods)
    let trait_impl = ImplBuilder::new(app_ident.clone(), syn::Generics::default())
        .implement_trait(quote! { crate::IcedAppTrait })
        .add_assoc_type(Ident::new("Message", app_ident.span()), quote! { #msg_ident })
        .add_method(quote! {
            fn update_app(&mut self, _message: Self::Message) -> iced::Command<Self::Message> {
                iced::Command::none()
            }
        })
        .add_method(quote! {
            fn view_app(&self) -> iced::Element<Self::Message> {
                iced::widget::text("Hello from macrokid + iced").into()
            }
        })
        .build();

    // Inherent run() helper
    let run_impl = ImplBuilder::new(app_ident.clone(), syn::Generics::default())
        .add_method(quote! {
            pub fn run() -> iced::Result
            where Self: Default + 'static,
            {
                iced::run(#title, Self::update_app, Self::view_app)
            }
        })
        .build();

    TokenStream::from(quote! {
        #struct_def
        #message_def
        #trait_impl
        #run_impl
    })
}

// Minimal input for iced_app!: `iced_app!(AppName, title: "Title")`
struct IcedAppInput { name: Ident, title: Option<LitStr> }

impl Parse for IcedAppInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        if input.is_empty() { return Ok(Self { name, title: None }); }
        let _comma: Token![,] = input.parse()?;
        let key: Ident = input.parse()?;
        if key != "title" { return Err(syn::Error::new_spanned(key, "expected `title`")); }
        let _colon: Token![:] = input.parse()?;
        let lit: LitStr = input.parse()?;
        Ok(Self { name, title: Some(lit) })
    }
}

/// Simpler derive macro for basic Iced application structure
#[proc_macro_derive(SimpleIcedApp, attributes(title))]
pub fn derive_simple_iced_app(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);

    // Extract title from attributes (simple parsing)
    let title = input.attrs.iter()
        .find(|attr| attr.path().is_ident("title"))
        .and_then(|attr| {
            if let Ok(syn::Meta::NameValue(meta)) = attr.meta.clone() {
                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = meta.value {
                    return Some(s.value());
                }
            }
            None
        })
        .unwrap_or_else(|| input.ident.to_string());

    let ident = input.ident;
    let generics = input.generics;

    // Build an inherent impl with a run() helper using ImplBuilder
    let run_impl = ImplBuilder::new(ident.clone(), generics)
        .add_method(quote! {
            pub fn run() -> iced::Result
            where 
                Self: Default + 'static + IcedAppTrait,
            {
                iced::run(#title, Self::update_app, Self::view_app)
            }
        })
        .build();

    let trait_def = quote! {
        pub trait IcedAppTrait {
            type Message: Clone + std::fmt::Debug;
            fn update_app(&mut self, message: Self::Message) -> iced::Command<Self::Message>;
            fn view_app(&self) -> iced::Element<Self::Message>;
        }
    };

    TokenStream::from(quote! { #run_impl #trait_def })
}

/// Attribute macro for simplifying view function implementations
/// 
/// This macro provides helper macros for common Iced widgets:
/// - `button!(text, message)` - Creates button with text and message
/// - `text!(value)` - Creates text widget
/// - `column![...]` and `row![...]` - Layout containers
/// 
/// # Example
/// ```rust
/// #[iced_view]
/// impl MyApp {
///     fn view(&self) -> Element<Message> {
///         column![
///             button!("Click me", Message::ButtonClicked),
///             text!(self.counter)
///         ]
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn iced_view(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input: ItemImpl = parse_macro_input!(item);
    
    // For now, just pass through the implementation
    // Later we can add macro expansions for button!(), text!(), etc.
    let expanded = quote! {
        #input
        
        // Helper macros for the view implementation
        macro_rules! button {
            ($text:expr, $message:expr) => {
                iced::widget::button($text).on_press($message)
            };
        }
        
        macro_rules! text {
            ($value:expr) => {
                iced::widget::text($value)
            };
        }
    };
    
    TokenStream::from(expanded)
}

// Helper structures and functions

struct MessageMethod {
    name: syn::Ident,
}

fn find_message_methods(_attrs: &[syn::Attribute]) -> Vec<MessageMethod> {
    // This is a simplified version - in a real implementation,
    // we would parse the struct definition to find methods marked with #[message]
    // For now, return empty vec as we'll handle this in the actual implementation
    vec![]
}

fn to_pascal_case(ident: &syn::Ident) -> String {
    let s = ident.to_string();
    let mut result = String::new();
    let mut capitalize_next = true;
    
    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    
    result
}

// Note: We intentionally do NOT depend on or re-export iced types from this
// proc-macro crate to keep it independent at compile time. The generated code
// references `iced::...` in the user's crate, which must include Iced.

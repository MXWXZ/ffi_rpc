use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream, Parser},
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    Expr, ExprClosure, Fields, FnArg, Ident, ImplItem, ItemImpl, ItemStruct, ItemTrait, Pat, Token,
    TraitItem, TraitItemFn, Type,
};

/// Expand to `plugin_api_struct` + `plugin_api_trait`
/// ```ignore
/// #[plugin_api(Client)]
/// pub trait ClientApi {
///     async fn add(a: i32, b: i32) -> i32;
/// }
/// ```
/// is equal to
/// ```ignore
/// #[plugin_api_struct]
/// pub struct Client;  // visibility is the same as the trait.
///
/// #[plugin_api_trait(Client)]
/// pub trait ClientApi {
///     async fn add(a: i32, b: i32) -> i32;
/// }
/// ```
#[proc_macro_attribute]
pub fn plugin_api(attr: TokenStream, item: TokenStream) -> TokenStream {
    let struct_name = parse_macro_input!(attr as Ident);
    let input = parse_macro_input!(item as ItemTrait);
    let vis = &input.vis;

    let expanded = quote! {
        #[ffi_rpc_macro::plugin_api_struct]
        #vis struct #struct_name;

        #[ffi_rpc_macro::plugin_api_trait(#struct_name)]
        #input
    };

    expanded.into()
}

/// Define several useful functions for API struct.
///
/// Note that the struct field should be named field and implement `Default`.
/// ```ignore
/// use std::env::consts::{DLL_PREFIX, DLL_SUFFIX};
///
/// #[plugin_api_struct]
/// pub struct Client {
///     field: i32,
/// }
/// // These are not allowed.
/// // pub struct Client(String);
/// // pub struct Client {
/// //     no_default: CustomType,
/// // }
///
/// let mut r = Registry::default();
/// let lib = Client::new(
///     format!("./target/debug/{}client{}", DLL_PREFIX, DLL_SUFFIX).as_ref(),
///     &mut r,
///     "client",
/// ).unwrap();
/// let client = Client::from(r.get("client").unwrap());
/// ```
#[proc_macro_attribute]
pub fn plugin_api_struct(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemStruct);
    let ident = &input.ident;
    let vis = &input.vis;
    let fields: Vec<_> = if let Fields::Named(field) = &mut input.fields {
        let ret = field
            .named
            .iter()
            .map(|x| x.ident.to_owned().unwrap())
            .collect();
        field
            .named
            .push(parse_quote!(_ffi_ref: ffi_rpc::plugin::PluginApiRef));
        ret
    } else if let Fields::Unit = &input.fields {
        input.fields = Fields::Named(parse_quote!({_ffi_ref: ffi_rpc::plugin::PluginApiRef}));
        Vec::new()
    } else {
        panic!("Expected named fields in struct");
    };

    let expanded = quote! {
        #input

        impl #ident {
            #vis fn new<S: Into<String>>(path: &std::path::Path,
                reg: &mut ffi_rpc::registry::Registry,
                id: S) -> Result<Self, abi_stable::library::LibraryError> {
                let api = ffi_rpc::plugin::load_plugin(path)?;
                reg.item.insert(id.into().into(), api);
                Ok(Self{
                    _ffi_ref: api,
                    #(#fields: Default::default()),*
                })
            }
        }

        impl From<ffi_rpc::plugin::PluginApiRef> for #ident {
            fn from(v: ffi_rpc::plugin::PluginApiRef) -> Self {
                Self {
                    _ffi_ref: v,
                    #(#fields: Default::default()),*
                }
            }
        }
    };

    expanded.into()
}

/// Define ffi call for each method in API struct.
///
/// Method arguments and return type should be:
/// - no self (prepend automatically)
/// - always async
/// - serializable
/// - no reference and mut
///
/// The implemetation will always use value, while the caller will always use reference.
/// ```ignore
/// pub struct Client;
///
/// #[plugin_api_trait(Client)]
/// pub trait ClientApi {
///     async fn add1(a: i32, b: i32) -> i32;
/// }
/// ```
#[proc_macro_attribute]
pub fn plugin_api_trait(attr: TokenStream, item: TokenStream) -> TokenStream {
    let struct_name = parse_macro_input!(attr as Ident);
    let mut input = parse_macro_input!(item as ItemTrait);
    let trait_name = &input.ident;
    let vis = &input.vis;

    let methods: Vec<_> = input
        .items
        .iter_mut()
        .filter_map(|item| {
            if let TraitItem::Fn(TraitItemFn { attrs, sig, .. }) = item {
                let mut method_sig = sig.clone();
                let param: Vec<Ident> = method_sig
                    .inputs
                    .iter_mut()
                    .map(|x| match x {
                        FnArg::Typed(x) => {
                            let ret = if let Pat::Ident(ident) = x.pat.as_ref() {
                                if ident.by_ref.is_some() || ident.mutability.is_some() {
                                    panic!("ref and mut is not supported")
                                }
                                if ident.subpat.is_some() {
                                    panic!("subpat is not supported")
                                }
                                ident.ident.clone()
                            } else {
                                panic!("unsupported argument type")
                            };
                            let ty = &x.ty;
                            x.ty = if let Type::Path(x) = ty.as_ref() {
                                if x.path.get_ident().is_some_and(|x| *x == "String") {
                                    parse_quote!(&str)
                                } else {
                                    parse_quote!(&#ty)
                                }
                            } else {
                                parse_quote!(&#ty)
                            };
                            ret
                        }
                        _ => panic!("unsupported `self` argument"),
                    })
                    .collect();
                sig.inputs
                    .insert(0, parse_quote!(_ffi_reg: &ffi_rpc::registry::Registry));
                sig.inputs.insert(0, parse_quote!(&self));
                method_sig
                    .inputs
                    .insert(0, parse_quote!(_ffi_reg: &ffi_rpc::registry::Registry));
                method_sig.inputs.insert(0, parse_quote!(&self));
                let method_name = &sig.ident;
                let api_name = format!("{}::{}", trait_name, method_name);
                Some(quote! {
                    #(#attrs)*
                    #vis #method_sig {
                        let param = (#(#param),*);
                        let ret = self._ffi_ref.call()(
                            abi_stable::std_types::RString::from(#api_name),
                            _ffi_reg,
                            bincode::serialize(&param).unwrap().into(),
                        ).await;
                        bincode::deserialize(&ret).unwrap()
                    }
                })
            } else {
                None
            }
        })
        .collect();

    let expanded = quote! {
        #[async_trait::async_trait]
        #input

        impl #struct_name {
            #(#methods)*
        }
    };

    expanded.into()
}

/// Mock a implementation without defining a real library.
///
/// ```ignore
/// #[plugin_impl_instance(||Server{})]
/// #[plugin_impl_call(ServerApi)]
/// #[plugin_impl_mock]
/// struct Server;
///
/// #[plugin_impl_trait]
/// impl ServerApi for Server {}
///
/// let mut r = Registry::default();
/// Server::register_mock(&mut r, "server");    // register the mock plugin.
/// ```
#[proc_macro_attribute]
pub fn plugin_impl_mock(_: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let vis = &input.vis;
    let ident = &input.ident;
    let expanded = quote! {
        #input

        impl #ident {
            abi_stable::staticref!(const _FFI_API: ffi_rpc::plugin::PluginApiRef = ffi_rpc::plugin::PluginApiRef(unsafe {
                abi_stable::prefix_type::WithMetadata::new(ffi_rpc::plugin::PluginApi { call: _ffi_call })
                    .as_prefix()
            }));

            #vis fn register_mock<S: Into<String>>(reg: &mut ffi_rpc::registry::Registry, id: S)  {
                reg.item.insert(id.into().into(), *#ident::_FFI_API);
            }
        }
    };
    expanded.into()
}

/// Create a new implementation instance with `LazyLock`.
///
/// The instance is named `"{struct_name.to_uppercase()}_INSTANCE"`.
///
/// Note that if you need to init the instance at runtime,
/// please create the static instance manually (e.g, using `OnceLock`).
/// ```ignore
/// // static SERVER_INSTANCE: std::sync::LazyLock<Server> = std::sync::LazyLock::new(||Server{});
/// #[plugin_impl_instance(|| Server{})] // pass the init closure.
/// struct Server;
/// ```
#[proc_macro_attribute]
pub fn plugin_impl_instance(attr: TokenStream, item: TokenStream) -> TokenStream {
    let init = parse_macro_input!(attr as ExprClosure);
    let input = parse_macro_input!(item as ItemStruct);
    let ident = &input.ident;
    let vis = &input.vis;
    let instance = format_ident!("{}_INSTANCE", input.ident.to_string().to_uppercase());

    let expanded = quote! {
        #vis static #instance: std::sync::LazyLock<#ident> = std::sync::LazyLock::new(#init);

        #input
    };
    expanded.into()
}

/// Define the root module in the plugin, `_ffi_call` must be defined in the same file.
///
/// Note that each plugin MUST have ONLY one root module.
/// You might need to customize `_ffi_call` if multiple instances in one plugin is needed (not common).
/// ```ignore
/// #[plugin_impl_root]
/// struct Api;
/// ```
#[proc_macro_attribute]
pub fn plugin_impl_root(_: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let expanded = quote! {
        #input

        #[abi_stable::export_root_module]
        pub fn _ffi_root_module() -> ffi_rpc::plugin::PluginApiRef {
            ffi_rpc::plugin::PluginApi { call: _ffi_call }.leak_into_prefix()
        }
    };
    expanded.into()
}

struct TraitList {
    traits: Punctuated<Ident, Token![,]>,
}

impl Parse for TraitList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let traits = Punctuated::parse_terminated(input)?;
        Ok(TraitList { traits })
    }
}

/// Define the `_ffi_call` function.
/// All implemented traits should be passed, seperated by a comma.
///
/// Note that each plugin MUST have ONLY one `_ffi_call` function.
/// You might need to customize it if multiple instances in one plugin is needed (not common).
/// ```ignore
/// #[plugin_impl_call(ClientApi1, ClientApi2)]
/// struct Api;
///
/// #[plugin_impl_trait]
/// impl ClientApi1 for Api {}
///
/// #[plugin_impl_trait]
/// impl ClientApi2 for Api {}
/// ```
#[proc_macro_attribute]
pub fn plugin_impl_call(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as TraitList);
    let input = parse_macro_input!(item as ItemStruct);
    let ident = &input.ident;
    let cases: Vec<_> = attr
        .traits
        .into_iter()
        .map(|x| {
            let prefix = format!("{}::", x);
            let func = format_ident!("parse_{}", x.to_string().to_lowercase());
            quote! {
                if (func.as_str().starts_with(#prefix)){
                    return #ident::#func(func, reg, param).await;
                }
            }
        })
        .collect();
    let expanded = quote! {
        #input

        #[abi_stable::sabi_extern_fn]
        pub fn _ffi_call<'fut>(func: abi_stable::std_types::RString,
            reg: &'fut ffi_rpc::registry::Registry,
            param: abi_stable::std_types::RVec<u8>) -> async_ffi::BorrowingFfiFuture<'fut, abi_stable::std_types::RVec<u8>> {
            async_ffi::BorrowingFfiFuture::new(async move {
                #(#cases)*
                panic!("{}", format!("Function `{func}` is not defined in the library"));
            })
        }
    };
    expanded.into()
}

/// Define how to invoke the methods.
///
/// A function named `"parse_{trait_name.to_lowercase()}"` is created to invoke each method from `_ffi_call`.
/// By default, it uses `"{struct_name.to_uppercase()}_INSTANCE"`.
/// You can pass an expression to guide how to get the actual instance if you define the instance manually.
/// ```ignore
/// #[plugin_impl_instance(|| Api{})]
/// struct Api;
///
/// #[plugin_impl_trait]    // will use `API_INSTANCE` by default.
/// impl ClientApi for Api {}
/// ```
/// Manually created instance:
/// ```ignore
/// static XX_INSTANCE: OnceLock<Api> = OnceLock::new();
/// struct Api;
///
/// #[plugin_impl_trait(XX_INSTANCE.get().unwrap())]
/// impl ClientApi for Api {}
/// ```
///
/// For each method, you need to prepend two arguments: `&self` and `reg: &Registry`.
/// ```ignore
/// // Interface
/// pub trait ClientApi {
///     async fn add(a: i32, b: i32) -> i32;
/// }
/// // Implementation
/// #[plugin_impl_trait]
/// impl ClientApi for Api {
///     async fn add(&self, _: &Registry, a: i32, b: i32) -> i32 {
///         a + b
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn plugin_impl_trait(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);
    let attr = Expr::parse.parse(attr).ok();
    let ty = &input.self_ty;
    let trait_name = input.trait_.as_ref().unwrap().1.get_ident().unwrap();
    let (instance, ty) = if let Type::Path(type_path) = ty.as_ref() {
        let last = type_path.path.segments.last().unwrap();
        let inst = format_ident!("{}_INSTANCE", &last.ident.to_string().to_uppercase());
        (attr.unwrap_or(parse_quote!(&*#inst)), &last.ident)
    } else {
        panic!("unknown type path");
    };

    let cases: Vec<_> = input
        .items
        .iter()
        .map(|x| {
            if let ImplItem::Fn(item) = x {
                let ident = &item.sig.ident;
                let param: Vec<Ident> = item
                    .sig
                    .inputs
                    .iter()
                    .skip(2)
                    .map(|x| match x {
                        FnArg::Typed(x) => {
                            if let Pat::Ident(ident) = x.pat.as_ref() {
                                ident.ident.clone()
                            } else {
                                panic!("unknown argument name")
                            }
                        }
                        _ => panic!("unsupported argument"),
                    })
                    .collect();
                let api_name = format!("{}::{}", trait_name, ident);
                quote! {
                    #api_name => {
                        let (#(#param),*) = bincode::deserialize(&param).unwrap();
                        bincode::serialize(&#trait_name::#ident(#instance, reg, #(#param),*).await)
                            .unwrap()
                            .into()
                    }
                }
            } else {
                panic!("unsupported implement function");
            }
        })
        .collect();
    let func = format_ident!("parse_{}", trait_name.to_string().to_lowercase());
    let expanded = quote! {
        #[async_trait::async_trait]
        #input

        impl #ty {
            pub async fn #func(func: abi_stable::std_types::RString,
                reg: &ffi_rpc::registry::Registry,
                param: abi_stable::std_types::RVec<u8>) -> abi_stable::std_types::RVec<u8> {
                match func.as_str() {
                    #(#cases)*
                    _ => panic!("Function is not defined in the library"),
                }
            }
        }
    };
    expanded.into()
}

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream, Parser},
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    Expr, ExprClosure, Fields, FnArg, Ident, ImplItem, ItemImpl, ItemStruct, ItemTrait, Pat, Token,
    TraitItem, TraitItemFn, Type,
};

#[proc_macro_attribute]
pub fn plugin_api(attr: TokenStream, item: TokenStream) -> TokenStream {
    let struct_name = parse_macro_input!(attr as Ident);
    let input = parse_macro_input!(item as ItemTrait);

    let expanded = quote! {
        #[ffi_rpc_macro::plugin_api_struct]
        pub struct #struct_name;

        #[ffi_rpc_macro::plugin_api_trait(#struct_name)]
        #input
    };

    expanded.into()
}

#[proc_macro_attribute]
pub fn plugin_api_struct(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemStruct);
    let ident = &input.ident;
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
            pub fn new<S: Into<String>>(path: &std::path::Path,
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

#[proc_macro_attribute]
pub fn plugin_api_trait(attr: TokenStream, item: TokenStream) -> TokenStream {
    let struct_name = parse_macro_input!(attr as Ident);
    let mut input = parse_macro_input!(item as ItemTrait);
    let trait_name = &input.ident;

    let methods: Vec<_> = input
        .items
        .iter_mut()
        .filter_map(|item| {
            if let TraitItem::Fn(TraitItemFn { attrs, sig, .. }) = item {
                let param: Vec<Ident> = sig
                    .inputs
                    .iter()
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
                sig.inputs
                    .insert(0, parse_quote!(_ffi_reg: &ffi_rpc::registry::Registry));
                sig.inputs.insert(0, parse_quote!(&self));
                let method_name = &sig.ident;
                let api_name = format!("{}::{}", trait_name, method_name);
                Some(quote! {
                    #(#attrs)*
                    #sig {
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
        #[async_trait::async_trait(?Send)]
        #input

        #[async_trait::async_trait(?Send)]
        impl #trait_name for #struct_name {
            #(#methods)*
        }
    };

    expanded.into()
}

#[proc_macro_attribute]
pub fn plugin_impl_mock(_: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let ident = &input.ident;
    let expanded = quote! {
        #input

        impl #ident {
            abi_stable::staticref!(const _FFI_API: ffi_rpc::plugin::PluginApiRef = ffi_rpc::plugin::PluginApiRef(unsafe {
                abi_stable::prefix_type::WithMetadata::new(ffi_rpc::plugin::PluginApi { call: _ffi_call })
                    .as_prefix()
            }));

            pub fn register_mock<S: Into<String>>(reg: &mut ffi_rpc::registry::Registry, id: S)  {
                reg.item.insert(id.into().into(), *#ident::_FFI_API);
            }
        }
    };
    expanded.into()
}

#[proc_macro_attribute]
pub fn plugin_impl_instance(attr: TokenStream, item: TokenStream) -> TokenStream {
    let init = parse_macro_input!(attr as ExprClosure);
    let input = parse_macro_input!(item as ItemStruct);
    let ident = &input.ident;
    let instance = format_ident!("{}_INSTANCE", input.ident.to_string().to_uppercase());

    let expanded = quote! {
        pub static #instance: std::sync::LazyLock<#ident> = std::sync::LazyLock::new(#init);

        #input
    };
    expanded.into()
}

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
            param: abi_stable::std_types::RVec<u8>) -> async_ffi::LocalBorrowingFfiFuture<'fut, abi_stable::std_types::RVec<u8>> {
            async_ffi::LocalBorrowingFfiFuture::new(async move {
                #(#cases)*
                panic!("Function is not defined in the library");
            })
        }
    };
    expanded.into()
}

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
                let param_ref: Vec<Expr> = item
                    .sig
                    .inputs
                    .iter()
                    .skip(2)
                    .map(|x| match x {
                        FnArg::Typed(x) => {
                            let ident = if let Pat::Ident(ident) = x.pat.as_ref() {
                                &ident.ident
                            } else {
                                panic!("unknown argument name")
                            };
                            if let Type::Reference(reference) = x.ty.as_ref() {
                                let and = &reference.and_token;
                                let mutability = &reference.mutability;
                                parse_quote!(#and #mutability #ident)
                            } else {
                                parse_quote!(#ident)
                            }
                        }
                        _ => panic!("unsupported argument"),
                    })
                    .collect();
                let api_name = format!("{}::{}", trait_name, ident);
                quote! {
                    #api_name => {
                        let (#(mut #param),*) = bincode::deserialize(&param).unwrap();
                        bincode::serialize(&#trait_name::#ident(#instance, reg, #(#param_ref),*).await)
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
        #[async_trait::async_trait(?Send)]
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

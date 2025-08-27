//#![feature(proc_macro_span)]
//#![feature(proc_macro_def_site)]
extern crate proc_macro;

//use file_names::find_file_index;

use proc_macro::TokenStream;
use proc_macro2::Group;
use quote::quote;
use syn::parse::{Parse, Parser};
use syn::{
    parse_macro_input, Block, Data, DataStruct, DeriveInput, Expr, Field, Fields, FieldsNamed,
    ImplItem, ImplItemFn, ItemImpl, ItemStruct, Lit, Local, LocalInit, Meta, Signature, Stmt,
};

#[proc_macro]
pub fn err_code(_input: TokenStream) -> TokenStream {
    todo!()
    //err_code_inner(input.into())
    //    .unwrap_or_else(Error::into_compile_error)
    //    .into()
}

//fn err_code_inner(input: proc_macro2::TokenStream) -> Result<proc_macro2::TokenStream> {
//    let caller = proc_macro2::Span::call_site();
//    let filename = caller.source_file().path();
//    let filename = filename.to_str().unwrap();
//    let line = caller.start().line();
//
//    println!("input: {input}, filename: {filename:?}, line: {line:?}");
//    // Create a hash or unique identifier based on filename and line
//    let err_code = encode_location(None, filename, line as u16);
//
//    Ok(quote_spanned!(Span::mixed_site() => {
//        #err_code
//    }))
//}
//
//fn encode_location(pkg_name: Option<&str>, file: &str, line: u16) -> f64 {
//    let file_hash = find_file_index(pkg_name, file).unwrap_or_default();
//
//    (((file_hash as u32) << 16) | (line as u32)) as f64
//}

#[proc_macro_attribute]
pub fn add_class_list(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemStruct);
    let class_list_field = Field::parse_named
        .parse2(quote! {
            pub(crate) class_list: ClassList<'static>
        })
        .unwrap();
    let Fields::Named(fields) = &mut input.fields else {
        panic!("Incorrect field type on struct");
    };

    fields.named.push(class_list_field);

    TokenStream::from(quote! { #input })
}

struct ClassListArgs(Vec<String>);

impl Parse for ClassListArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        use syn::{Expr, ExprArray, ExprLit};

        if input.is_empty() {
            return Ok(ClassListArgs(Vec::new()));
        }

        let array: ExprArray = input.parse()?;

        let classes: Vec<String> = array
            .elems
            .iter()
            .filter_map(|elem| match elem {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(lit_str),
                    ..
                }) => Some(lit_str.value()),
                _ => None,
            })
            .collect();

        Ok(ClassListArgs(classes))
    }
}

#[proc_macro_attribute]
pub fn with_class_list(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemImpl);

    let attr = attr.to_string();
    if attr != "in_builder" {
        input.update_render();
    }
    //if attr == "in_builder" {}
    input.impl_class_list();

    TokenStream::from(quote! { #input })
}

///The attribute is a default class for the builder.
#[proc_macro_attribute]
pub fn builder(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ClassListArgs(classes) = parse_macro_input!(attr as ClassListArgs);
    let mut item = parse_macro_input!(item as ImplItemFn);
    let ImplItemFn {
        block: Block { stmts, .. },
        ..
    } = &mut item;
    use syn::{ExprStruct, FieldValue, LitStr};

    let class_list_init = match classes.is_empty() {
        true => quote! { ClassList::new() },
        false => {
            let class_strings: Vec<_> = classes
                .iter()
                .map(|s| LitStr::new(s, proc_macro2::Span::call_site()))
                .collect();
            quote! { ClassList::from(vec![#(#class_strings),*]) }
        }
    };
    let class_list_field = syn::parse2::<FieldValue>(quote! {
        class_list: #class_list_init
    })
    .unwrap();

    let fields = stmts
        .iter_mut()
        .find_map(|stmt| match stmt {
            Stmt::Expr(Expr::Struct(ExprStruct { fields, .. }), _) => Some(fields),
            _ => None,
        })
        .unwrap();

    fields.push(class_list_field);

    TokenStream::from(quote! {#item})
}

trait ClassListImpl {
    fn impl_class_list(&mut self);
    fn update_render(&mut self);
}

impl ClassListImpl for ItemImpl {
    fn impl_class_list(&mut self) {
        let items = &mut self.items;
        let class_list_method: ImplItem = syn::parse_quote! {
            #[inline]
            pub(crate) fn class_list(self, class_list: &'static str) -> Self {
                let mut class_list_lock = self.class_list.lock_mut();
                class_list_lock.extend(class_list.split_whitespace());
                drop(class_list_lock);

                self
            }
        };

        items.push(class_list_method);
    }

    fn update_render(&mut self) {
        use syn::{ExprMacro, Macro};

        let items = &mut self.items;
        let statements = items
            .iter_mut()
            .find_map(|item| match item {
                ImplItem::Fn(ImplItemFn {
                    sig: Signature { ident, .. },
                    block: Block { stmts, .. },
                    ..
                }) if ident == "render" || ident == "build" => Some(stmts),
                _ => None,
            })
            .unwrap();
        let macro_tokens = statements
            .iter_mut()
            .find_map(|stmt| match stmt {
                Stmt::Expr(
                    Expr::Macro(ExprMacro {
                        mac: Macro { tokens, .. },
                        ..
                    }),
                    _,
                ) => Some(tokens),
                Stmt::Local(Local { init, .. }) => {
                    init.as_mut()
                        .and_then(|LocalInit { ref mut expr, .. }| match expr.as_mut() {
                            Expr::Macro(ExprMacro {
                                mac: Macro { tokens, .. },
                                ..
                            }) => Some(tokens),
                            _ => None,
                        })
                }
                _ => None,
            })
            .unwrap();

        *macro_tokens = macro_tokens
            .clone()
            .into_iter()
            .map(|mut tt| {
                if let proc_macro2::TokenTree::Group(group) = &mut tt {
                    let mut dom_builder_stream = group.stream();
                    dom_builder_stream.extend(quote! {
                        .class_list(&self.class_list)
                    });
                    *group = Group::new(group.delimiter(), dom_builder_stream);
                }

                tt
            })
            .collect();

        //TODO: Otherwise render is just self.inner.into_dom()
    }
}

///Each struct deriving `Settings` has to also derive `Default`
#[proc_macro_derive(Settings, attributes(setting))]
pub fn derive_settings(item: TokenStream) -> TokenStream {
    //dbg!(&item);
    let item = parse_macro_input!(item as DeriveInput);
    let Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { named, .. }),
        ..
    }) = item.data
    else {
        panic!("Settings can only be derived on structs!");
    };
    let name = &item.ident;

    let settings = named.iter().filter_map(|f| {
        let should_skip = f.attrs.iter().any(|attr| {
            let Meta::List(list) = &attr.meta else {
                return false;
            };
            let path = list.path.get_ident().unwrap();

            path == "setting" && list.tokens.to_string() == "skip"
        });
        if should_skip {
            return None;
        }

        let field_name = f
            .ident
            .as_ref()
            .unwrap_or_else(|| panic!("no field name for field"));
        let field_str = field_name.to_string();

        let res = quote! {
            if let Some(setting_value) = settings.get(#field_str) {
                this.#field_name.update(setting_value.clone());
            }

            let future = this
                .#field_name
                .as_setting_signal(|change| json!({ #field_str: change }))
                .for_each(move |json_change| async move {
                    //debug_log!(&serde_json::to_string_pretty(&json_change).unwrap());
                    todo!()
                    // crate::globals::port::Port::__internal_send_settings_change(&addon_name, json_change).await
                });
            wasm_bindgen_futures::spawn_local(future);
        };

        Some(res)
    });

    let expanded = quote! {
        impl #name {
            const WINDOW_TYPE: crate::globals::addons::WindowType = crate::globals::addons::WindowType::SettingsWindow;

            fn new(addon_name: crate::globals::addons::AddonName) -> &'static Self {
                use serde_json::json;
                use futures_signals::signal::SignalExt;
                use futures::stream::StreamExt;
                //use common::debug_log;

                use crate::utils::{Setting, SettingFromValue};

                let settings = crate::globals::addons::Addons::__internal_get_settings(addon_name);
                let this: &'static Self = Box::leak(Box::new(Self::default()));

                #(#settings)*

                this
            }
        }
    };
    //println!("{}", &expanded);

    expanded.into()
}

///Each struct deriving `ActiveSettings` has to also derive `Default`
#[proc_macro_derive(ActiveSettings, attributes(setting))]
pub fn derive_active_settings(item: TokenStream) -> TokenStream {
    //dbg!(&item);
    let item = parse_macro_input!(item as DeriveInput);
    let Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { named, .. }),
        ..
    }) = item.data
    else {
        panic!("ActiveSettings can only be derived on structs!");
    };
    let name = &item.ident;

    let settings = named.iter().filter_map(|f| {
        let should_skip = f.attrs.iter().any(|attr| {
            let Meta::List(list) = &attr.meta else {
                return false;
            };
            let path = list.path.get_ident().unwrap();

            path == "setting" && list.tokens.to_string() == "skip"
        });
        if should_skip {
            return None;
        }

        let field_name = f
            .ident
            .as_ref()
            .unwrap_or_else(|| panic!("no field name for field"));
        let field_str = field_name.to_string();

        let res = quote! {
            if let Some(setting_value) = settings.get(#field_str) {
                this.#field_name.update(setting_value.clone());
            }

            let future = this
                .#field_name
                .as_setting_signal(|change| json!({ #field_str: change }))
                .for_each(move |json_change| {
                    //debug_log!(&serde_json::to_string_pretty(&json_change).unwrap());
                    async move {
                        todo!()
                        // crate::globals::port::Port::__internal_send_active_settings_change(&addon_name, json_change).await
                    }
                });
            wasm_bindgen_futures::spawn_local(future);
        };

        Some(res)
    });

    let expanded = quote! {
        impl #name {
            const WINDOW_TYPE: crate::globals::addons::WindowType = crate::globals::addons::WindowType::AddonWindow;

            fn new(addon_name: crate::globals::addons::AddonName) -> &'static Self {
                use serde_json::json;
                use futures_signals::signal::SignalExt;
                use futures::stream::StreamExt;
                use common::debug_log;

                use crate::utils::{Setting, SettingFromValue};

                let settings = Addons::__internal_get_active_settings(addon_name);
                let this: &'static Self = Box::leak(Box::new(Self::default()));

                #(#settings)*

                this
            }
        }
    };
    //println!("{}", &expanded);

    expanded.into()
}

//trait ToSnakeCase: AsRef<str> {
//    fn to_snake_case(&self) -> String;
//}
//
//impl<T> ToSnakeCase for T
//where
//    T: AsRef<str>,
//{
//    fn to_snake_case(&self) -> String {
//        let text = self.as_ref();
//
//        let mut buffer = String::with_capacity(text.len() + text.len() / 2);
//
//        let mut text = text.chars();
//
//        if let Some(first) = text.next() {
//            let mut n2: Option<(bool, char)> = None;
//            let mut n1: (bool, char) = (first.is_lowercase(), first);
//
//            for c in text {
//                let prev_n1 = n1;
//
//                let n3 = n2;
//                n2 = Some(n1);
//                n1 = (c.is_lowercase(), c);
//
//                // insert underscore if acronym at beginning
//                // ABc -> a_bc
//                if let Some((false, c3)) = n3 {
//                    if let Some((false, c2)) = n2 {
//                        if n1.0 && c3.is_uppercase() && c2.is_uppercase() {
//                            buffer.push('_');
//                        }
//                    }
//                }
//
//                buffer.push_str(&prev_n1.1.to_lowercase().to_string());
//
//                // insert underscore before next word
//                // abC -> ab_c
//                if let Some((true, _)) = n2 {
//                    if n1.1.is_uppercase() {
//                        buffer.push('_');
//                    }
//                }
//            }
//
//            buffer.push_str(&n1.1.to_lowercase().to_string());
//        }
//
//        buffer
//    }
//}

#[proc_macro_derive(Setting, attributes(setting))]
pub fn derive_setting(item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as DeriveInput);
    let Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { named, .. }),
        ..
    }) = item.data
    else {
        panic!("Setting can only be derived on structs!");
    };
    let name = &item.ident;

    let settings_names = named
        .iter()
        .filter_map(|f| {
            let should_skip = f.attrs.iter().any(|attr| {
                let Meta::List(list) = &attr.meta else {
                    return false;
                };
                let path = list.path.get_ident().unwrap();

                path == "setting" && list.tokens.to_string() == "skip"
            });
            if should_skip {
                return None;
            }
            Some(
                f.ident
                    .as_ref()
                    .unwrap_or_else(|| panic!("no field name for field")),
            )
        })
        .collect::<Vec<_>>();
    let settings_streams = settings_names.clone().into_iter().map(|name| {
        let name_str = name.to_string();
        quote! {
            let #name = self.#name.as_setting_signal(|change| serde_json::json!({ #name_str: change})).map(f);
        }
    });
    let setting_idents = settings_names.clone().into_iter().map(|name| {
        quote! {
            #name
        }
    });
    let output_stream = match settings_names.len() {
        0 => panic!("Too few fields on struct"),
        1 => {
            let stream = setting_idents
                .collect::<Vec<_>>()
                .into_iter()
                .next()
                .unwrap();
            quote! { #stream }
        }
        _ => quote! { futures::stream_select!(#(#setting_idents),*) },
    };
    let setting_impl = quote! {
        impl crate::utils::Setting for #name {
            fn as_setting_signal(
                &self,
                f: fn(serde_json::Value) -> serde_json::Value,
            ) -> impl futures::Stream<Item = serde_json::Value> {
                use futures::StreamExt;

                #(#settings_streams)*
                #output_stream
            }
        }
    };

    let settings_from_values = settings_names.into_iter().map(|name| {
        let name_str = name.to_string();

        quote! {
            if let Some(setting) = value.get(#name_str) {
                self.#name.update(setting.clone());
            }
        }
    });

    //let name_str = name.to_string();
    let setting_from_value_impl = quote! {
        impl crate::utils::SettingFromValue for #name {
            fn update(&self, value: serde_json::Value) {
                //common::debug_log!("value for ", #name_str, &format!("{value}"));
                #(#settings_from_values)*
            }
        }
    };

    let expanded = quote! {
        #setting_impl

        #setting_from_value_impl
    };
    //println!("{}", &expanded);

    expanded.into()
}

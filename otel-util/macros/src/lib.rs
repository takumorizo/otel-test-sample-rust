use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, ItemFn, Lit, Meta, NestedMeta};

struct UseOtelTestArgs {
    pub endpoint: String,
    pub others: Vec<NestedMeta>,
}

impl UseOtelTestArgs {
    fn new(args: AttributeArgs) -> Self {
        let mut endpoint = "grpc://localhost:4317".to_string();
        let mut other_args = Vec::<NestedMeta>::new();
        for arg in args {
            match arg {
                NestedMeta::Meta(Meta::NameValue(nv)) if nv.path.is_ident("endpoint") => {
                    if let Lit::Str(s) = &nv.lit {
                        endpoint = s.value();
                    }
                }
                _ => {
                    other_args.push(arg);
                }
            }
        }
        UseOtelTestArgs {
            endpoint,
            others: other_args,
        }
    }
}

#[proc_macro_attribute]
pub fn use_otel_at_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let attrs = &input.attrs;
    let return_type = &input.sig.output;
    let is_default_type = *return_type == syn::ReturnType::Default;
    let block = &input.block;

    let args = parse_macro_input!(_attr as AttributeArgs);
    let my_args = UseOtelTestArgs::new(args);
    let (endpoint, other_args) = (my_args.endpoint, my_args.others);

    let tokio_test_attrs = match other_args.len() {
        0 => quote! { #[tokio::test] },
        _ => {
            quote! { #[tokio::test(#(#other_args),*)] }
        }
    };

    let await_block = if is_default_type {
        quote! { execute_async_block.await; }
    } else {
        quote! { execute_async_block.await.unwrap(); }
    };

    let expanded = quote! {
        #(#attrs)*
        #tokio_test_attrs
        async fn #fn_name() {
            // otel の初期化処理
            let __otel_guard_for_otel_test;
            {
                use otel_util::DefaultSimpleOtelGuardFactory;
                __otel_guard_for_otel_test = DefaultSimpleOtelGuardFactory::new(#endpoint, stringify!(#fn_name), "non-deployment").build();
            }

            // 関数 block の async 定義
            use otel_util::tracing::Instrument;
            let execute_async_block = async {
                #block
            }.instrument(tracing::info_span!(stringify!(#fn_name)));

            // 関数 block の async 実行と、panic-catch 部分
            use otel_util::tokio::time::{sleep, Duration};
            use std::panic::{self, AssertUnwindSafe};
            let result = panic::catch_unwind(AssertUnwindSafe(|| {
                tokio::spawn(async move {
                    #await_block
                })
            }));

            let join_result = result.unwrap().await;
            sleep(Duration::from_secs(1)).await; // trace の送信の前に、待機しないと、trace が送信されない。

            if join_result.is_err() {
                panic!("panic occurred");
            }
        }
    };
    TokenStream::from(expanded)
}

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, ItemFn, Lit, Meta, NestedMeta};

#[proc_macro_attribute]
pub fn use_otel_at_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let block = &input.block;

    let args = parse_macro_input!(_attr as AttributeArgs);
    let endpoint = match args.first() {
        Some(NestedMeta::Meta(Meta::NameValue(nv))) if nv.path.is_ident("endpoint") => {
            if let Lit::Str(s) = &nv.lit {
                s.value()
            } else {
                "http://localhost:4317".to_string() // デフォルトのエンドポイント
            }
        }
        _ => "http://localhost:4317".to_string(), // 引数がない場合のデフォルトのエンドポイント
    };

    let expanded = quote! {
        #[tokio::test]
        async fn #fn_name() {
            // otel の初期化処理
            let __otel_guard_for_otel_test;
            {
                use otel_test::opentelemetry::{global, KeyValue};
                use otel_test::opentelemetry_otlp::WithExportConfig;
                use otel_test::opentelemetry_sdk::{
                    runtime,
                    trace::{BatchConfig, RandomIdGenerator, Sampler, Tracer},
                    Resource,
                };
                use otel_test::opentelemetry_semantic_conventions::{
                    resource::{DEPLOYMENT_ENVIRONMENT, SERVICE_NAME, SERVICE_VERSION},
                    SCHEMA_URL,
                };
                use otel_test::tokio::runtime::Handle;
                use otel_test::tracing_core::Level;
                use otel_test::tracing_opentelemetry::OpenTelemetryLayer;
                use otel_test::tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

                fn my_resource() -> Resource {
                    Resource::from_schema_url(
                        [
                            // fn_name を文字列として、service_name に入れたい
                            KeyValue::new(SERVICE_NAME, stringify!(#fn_name)),
                            KeyValue::new(DEPLOYMENT_ENVIRONMENT, "non-deployment"),
                        ],
                        SCHEMA_URL,
                    )
                }

                fn my_init_tracer(collector_endpoint: &str) -> Tracer {
                    global::set_text_map_propagator(opentelemetry_sdk::propagation::TraceContextPropagator::new());

                    opentelemetry_otlp::new_pipeline()
                        .tracing()
                        .with_trace_config(
                            opentelemetry_sdk::trace::Config::default()
                                // Customize sampling strategy
                                .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
                                    1.0,
                                ))))
                                // If export trace to AWS X-Ray, you can use XrayIdGenerator
                                .with_id_generator(RandomIdGenerator::default())
                                .with_resource(my_resource()),
                        )
                        .with_exporter(
                            opentelemetry_otlp::new_exporter()
                                .tonic()
                                .with_endpoint(collector_endpoint),
                        )
                        .install_simple()
                        .unwrap()
                }

                fn my_init_telemetry(collector_endpoint: &str) -> MyOtelGuard {
                    let tracer = my_init_tracer(collector_endpoint);

                    tracing_subscriber::registry()
                        .with(tracing_subscriber::filter::LevelFilter::from_level(
                            Level::INFO,
                        ))
                        // .with(tracing_subscriber::fmt::layer())// trace のログ出力はオフにする。
                        .with(OpenTelemetryLayer::new(tracer.clone()))
                        .init();

                    std::panic::set_hook(Box::new(|panic_info| {
                        eprintln!("panic occurred: {}", panic_info);
                        tracing::error!("panic occurred: {}", panic_info);
                        // std::thread::sleep(std::time::Duration::from_secs(2));
                        // let _ = Handle::current().spawn(async {
                        //     opentelemetry::global::shutdown_tracer_provider();
                        // });
                    }));

                    MyOtelGuard { tracer }
                }

                struct MyOtelGuard {
                    tracer: Tracer,
                }

                impl Drop for MyOtelGuard {
                    fn drop(&mut self) {
                        let _ = Handle::current().spawn(async {
                            opentelemetry::global::shutdown_tracer_provider();
                        });
                    }
                }
                __otel_guard_for_otel_test = my_init_telemetry(#endpoint);
            }

            // テスト対象の関数を実行
            let result = std::panic::catch_unwind(|| {
                #block
            });
            if result.is_err() {
                use otel_test::tokio::runtime::Handle;
                tracing::error!("panic occurred @ {}", stringify!(#fn_name));
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                let _ = Handle::current().spawn(async {
                    opentelemetry::global::shutdown_tracer_provider();
                });
                panic!("panic occurred @ {}", stringify!(#fn_name));
            } else {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    };

    TokenStream::from(expanded)
}

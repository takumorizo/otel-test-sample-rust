const CONTAINER_RESULT_PATH: &str = "/result.json";

// use super::trace_equivalency::TraceContent;
use opentelemetry_proto::tonic::{
    resource::{self, v1::Resource},
    trace::v1::{ResourceSpans, TracesData},
};
use otel_util::opentelemetry::StringValue;
use std::{
    fmt::Debug,
    io::{self, BufRead},
    os::unix::fs::PermissionsExt,
    vec,
};
use testcontainers::{
    core::{AccessMode, IntoContainerPort, Mount},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt, TestcontainersError,
};

struct CollectorContainerFactory {
    base_image: GenericImage,
    host_config_path: String,
    host_result_path: String,
}

impl CollectorContainerFactory {
    fn new(base_image: GenericImage, host_config_path: &str, host_result_path: &str) -> Self {
        CollectorContainerFactory {
            base_image,
            host_config_path: host_config_path.to_string(),
            host_result_path: host_result_path.to_string(),
        }
    }

    pub async fn build(self) -> Result<ContainerAsync<GenericImage>, TestcontainersError> {
        self.base_image
            .with_mapped_port(4317, 4317.tcp())
            .with_mapped_port(4318, 4318.tcp())
            .with_mapped_port(13133, 13133.tcp())
            .with_mapped_port(8889, 8889.tcp())
            .with_mount(Mount::bind_mount(
                self.host_config_path.clone(),
                "/etc/opentelemetry-collector.yaml",
            ))
            .with_mount(
                Mount::bind_mount(self.host_result_path.clone(), CONTAINER_RESULT_PATH)
                    .with_access_mode(AccessMode::ReadWrite),
            )
            .with_cmd(vec!["--config=/etc/opentelemetry-collector.yaml"])
            .start()
            .await
    }
}

struct OriginalTestExecutor {
    test_name: String,
}

impl OriginalTestExecutor {
    fn new(test_name: &str) -> Self {
        OriginalTestExecutor {
            test_name: test_name.to_string(),
        }
    }

    async fn execute(&self) -> String {
        let crate_path = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let config_path = format!("{crate_path}/otel-collector-config.yaml");
        let result_path = format!("{crate_path}/result/{}.json", self.test_name);
        // let expected_path = format!("{crate_path}/expected/{test_name}.json");
        let file = std::fs::File::create(result_path.clone()).unwrap();
        file.set_permissions(std::fs::Permissions::from_mode(0o666))
            .unwrap();

        let collector_factory = CollectorContainerFactory::new(
            GenericImage::new("otel/opentelemetry-collector-contrib", "0.103.1"),
            &config_path,
            &result_path,
        );
        let _container = collector_factory
            .build()
            .await
            .expect("Failed to start opentelemetry-collector");

        // when
        tokio::process::Command::new("cargo")
            .arg("test")
            .arg(format!("tests::original_test_case::{}", self.test_name))
            .output()
            .await
            .expect("Failed to execute cargo test");

        result_path
    }
}

trait TraceInfoExtractor {
    fn get_service_name(&self) -> String;
    fn get_span_names(&self) -> Vec<String>;
}
impl TraceInfoExtractor for ResourceSpans {
    fn get_service_name(&self) -> String {
        self.resource
            .clone()
            .unwrap_or_default()
            .attributes
            .iter()
            .find(|attr| attr.key == "service.name")
            .map_or("".to_string(), |attr| {
                if let Some(service_value) = &attr.value {
                    match service_value.value {
                        Some(ref v) => {
                            format!("{:?}", v)
                        }
                        None => "".to_string(),
                    }
                } else {
                    "".to_string()
                }
            })
    }

    fn get_span_names(&self) -> Vec<String> {
        let mut ans = vec![];
        for scope_span in self.scope_spans.clone() {
            let span_names: Vec<String> = scope_span
                .spans
                .iter()
                .map(|span| span.name.clone())
                .collect();
            ans.extend(span_names);
        }
        ans
    }
}

trait SpanInfoExtractor {
    fn get_span_name(&self) -> String;
    fn get_event_names(&self) -> Vec<String>;
    fn get_event_exception_messages(&self) -> Vec<String>;
}

impl SpanInfoExtractor for opentelemetry_proto::tonic::trace::v1::Span {
    fn get_span_name(&self) -> String {
        self.name.clone()
    }

    fn get_event_names(&self) -> Vec<String> {
        let mut ans: Vec<String> = self.events.iter().map(|event| event.name.clone()).collect();
        ans.sort();
        ans
    }

    fn get_event_exception_messages(&self) -> Vec<String> {
        let mut ans :Vec<String> = self.events
            .iter()
            .map(|event| {
                event.attributes.iter().find_map(|attr| {
                    if attr.key == "exception.message" {
                        if let Some(opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(message)) = attr.value.as_ref().and_then(|v| v.value.as_ref()) {
                            Some(message.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }).unwrap_or_else(|| "".to_string())
            })
            .collect();
        ans.sort();
        ans
    }
}

// 複数のResouceSpan を持って、意味を取得、一致をみる構造体。
struct TraceContent {
    pub trace: Vec<ResourceSpans>,
}

impl TraceContent {
    pub fn new(trace: Vec<ResourceSpans>) -> Self {
        TraceContent { trace }
    }

    pub fn get_span_names(&self) -> Vec<String> {
        let mut ans: Vec<String> = self
            .trace
            .iter()
            .flat_map(|resource_span| resource_span.get_span_names())
            .collect();
        ans.sort();
        ans
    }

    pub fn span_count(&self) -> usize {
        self.trace
            .iter()
            .map(|resource_span| resource_span.scope_spans.len())
            .sum()
    }

    pub fn status_count(&self, status: i32) -> usize {
        self.trace
            .iter()
            .map(|resource_span| {
                resource_span
                    .scope_spans
                    .iter()
                    .map(|scope_span| {
                        scope_span
                            .spans
                            .iter()
                            .filter(|span| match &span.status {
                                Some(s) => s.code == status,
                                None => false,
                            })
                            .count()
                    })
                    .sum::<usize>()
            })
            .sum()
    }

    pub fn get_span_event_names(&self) -> std::collections::HashMap<String, Vec<String>> {
        let mut span_event_names = std::collections::HashMap::new();
        for resource_span in &self.trace {
            for scope_span in &resource_span.scope_spans {
                for span in &scope_span.spans {
                    span_event_names.insert(span.get_span_name(), span.get_event_names());
                }
            }
        }
        span_event_names
    }

    pub fn get_span_event_exceptions(&self) -> std::collections::HashMap<String, Vec<String>> {
        let mut span_event_names = std::collections::HashMap::new();
        for resource_span in &self.trace {
            for scope_span in &resource_span.scope_spans {
                for span in &scope_span.spans {
                    span_event_names
                        .insert(span.get_span_name(), span.get_event_exception_messages());
                }
            }
        }
        span_event_names
    }
}

fn build_trace_content(path: &str) -> TraceContent {
    let lines = io::BufReader::new(std::fs::File::open(path).unwrap()).lines();
    let traces_data: Vec<TracesData> = lines
        .map_while(Result::ok)
        .map(|line| {
            let trace_data: TracesData =
                serde_json::from_str(&line).expect("Failed to read json file");
            trace_data
        })
        .collect();
    let mut resource_spans: Vec<ResourceSpans> = traces_data
        .into_iter()
        .flat_map(|trace_data| trace_data.resource_spans)
        .collect();

    TraceContent::new(resource_spans)
}

#[tokio::test]
async fn check_otlp_output_failed_otel_test() {
    // given
    let test_name = "failed_otel_test";
    // when
    let original_executor = OriginalTestExecutor::new(test_name);
    let result_path = original_executor.execute().await;

    // then
    let result = build_trace_content(&result_path);
    let expected_path = format!("./expected/{}.json", test_name);
    let expected = build_trace_content(&expected_path);

    println!("==============================");
    println!("result.trace: {:?}", result.trace);
    println!("==============================");
    println!("==============================");
    println!("expected.trace: {:?}", expected.trace);
    println!("==============================");

    assert_eq!(result.get_span_names(), expected.get_span_names());

    println!(
        "result.get_span_names(): {:?}, expected.get_span_names(): {:?}",
        result.get_span_names(),
        expected.get_span_names()
    );

    assert_eq!(result.span_count(), expected.span_count());

    println!(
        "result.span_count(): {:?}, expected.span_count(): {:?}",
        result.span_count(),
        expected.span_count()
    );

    assert_eq!(result.status_count(2), expected.status_count(2));

    println!(
        "result.status_count(2): {:?}, expected.status_count(2): {:?}",
        result.status_count(2),
        expected.status_count(2)
    );

    assert_eq!(
        result.get_span_event_names(),
        expected.get_span_event_names(),
    );

    println!(
        "result.get_span_event_names(): {:?}, expected.get_span_event_names(): {:?}",
        result.get_span_event_names(),
        expected.get_span_event_names()
    );

    assert_eq!(
        result.get_span_event_exceptions(),
        expected.get_span_event_exceptions(),
    );

    println!(
        "result.get_span_event_exceptions(): {:?}, expected.get_span_event_exceptions(): {:?}",
        result.get_span_event_exceptions(),
        expected.get_span_event_exceptions()
    );
}

#[tokio::test]
async fn check_otlp_output_error_otel_test() {
    // given
    let test_name = "error_otel_test";
    // when
    let original_executor = OriginalTestExecutor::new(test_name);
    let result_path = original_executor.execute().await;

    // then
    let result = build_trace_content(&result_path);
    let expected_path = format!("./expected/{}.json", test_name);
    let expected = build_trace_content(&expected_path);

    println!("==============================");
    println!("result.trace: {:?}", result.trace);
    println!("==============================");
    println!("==============================");
    println!("expected.trace: {:?}", expected.trace);
    println!("==============================");

    assert_eq!(result.get_span_names(), expected.get_span_names());

    println!(
        "result.get_span_names(): {:?}, expected.get_span_names(): {:?}",
        result.get_span_names(),
        expected.get_span_names()
    );

    assert_eq!(result.span_count(), expected.span_count());

    println!(
        "result.span_count(): {:?}, expected.span_count(): {:?}",
        result.span_count(),
        expected.span_count()
    );

    assert_eq!(result.status_count(2), expected.status_count(2));

    println!(
        "result.status_count(2): {:?}, expected.status_count(2): {:?}",
        result.status_count(2),
        expected.status_count(2)
    );
    assert_eq!(
        result.get_span_event_names(),
        expected.get_span_event_names(),
    );

    println!(
        "result.get_span_event_names(): {:?}, expected.get_span_event_names(): {:?}",
        result.get_span_event_names(),
        expected.get_span_event_names()
    );

    assert_eq!(
        result.get_span_event_exceptions(),
        expected.get_span_event_exceptions(),
    );

    println!(
        "result.get_span_event_exceptions(): {:?}, expected.get_span_event_exceptions(): {:?}",
        result.get_span_event_exceptions(),
        expected.get_span_event_exceptions()
    );
}

#[tokio::test]
async fn check_otlp_output_panic_otel_test() {
    // given
    let test_name = "panic_otel_test";
    // when
    let original_executor = OriginalTestExecutor::new(test_name);
    let result_path = original_executor.execute().await;

    // then
    let result = build_trace_content(&result_path);
    let expected_path = format!("./expected/{}.json", test_name);
    let expected = build_trace_content(&expected_path);

    println!("==============================");
    println!("result.trace: {:?}", result.trace);
    println!("==============================");
    println!("expected.trace: {:?}", expected.trace);
    println!("==============================");

    assert_eq!(result.get_span_names(), expected.get_span_names());

    println!(
        "result.get_span_names(): {:?}, expected.get_span_names(): {:?}",
        result.get_span_names(),
        expected.get_span_names()
    );

    assert_eq!(result.span_count(), expected.span_count());

    println!(
        "result.span_count(): {:?}, expected.span_count(): {:?}",
        result.span_count(),
        expected.span_count()
    );

    assert_eq!(result.status_count(2), expected.status_count(2));

    println!(
        "result.status_count(2): {:?}, expected.status_count(2): {:?}",
        result.status_count(2),
        expected.status_count(2)
    );
    assert_eq!(
        result.get_span_event_names(),
        expected.get_span_event_names(),
    );

    println!(
        "result.get_span_event_names(): {:?}, expected.get_span_event_names(): {:?}",
        result.get_span_event_names(),
        expected.get_span_event_names()
    );

    assert_eq!(
        result.get_span_event_exceptions(),
        expected.get_span_event_exceptions(),
    );

    println!(
        "result.get_span_event_exceptions(): {:?}, expected.get_span_event_exceptions(): {:?}",
        result.get_span_event_exceptions(),
        expected.get_span_event_exceptions()
    );
}

#[tokio::test]
async fn check_otlp_output_succeed_otel_test() {
    // given
    let test_name = "succeed_otel_test";
    // when
    let original_executor = OriginalTestExecutor::new(test_name);
    let result_path = original_executor.execute().await;

    // then
    let result = build_trace_content(&result_path);
    let expected_path = format!("./expected/{}.json", test_name);
    let expected = build_trace_content(&expected_path);

    // assert_eq!(result, expected);
    println!("==============================");
    println!("result.trace: {:?}", result.trace);
    println!("==============================");
    println!("expected.trace: {:?}", expected.trace);
    println!("==============================");

    assert_eq!(result.get_span_names(), expected.get_span_names());

    println!(
        "result.get_span_names(): {:?}, expected.get_span_names(): {:?}",
        result.get_span_names(),
        expected.get_span_names()
    );

    assert_eq!(result.span_count(), expected.span_count());

    println!(
        "result.span_count(): {:?}, expected.span_count(): {:?}",
        result.span_count(),
        expected.span_count()
    );

    assert_eq!(result.status_count(2), expected.status_count(2));

    println!(
        "result.status_count(2): {:?}, expected.status_count(2): {:?}",
        result.status_count(2),
        expected.status_count(2)
    );
}

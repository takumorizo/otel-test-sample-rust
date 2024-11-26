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
            .unwrap_or(Resource::default())
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
    let result_trace_content = build_trace_content(&result_path);
    let expected_path = format!("./expected/{}.json", test_name);
    let expected_trace_content = build_trace_content(&expected_path);

    println!("==============================");
    println!(
        "result_trace_content.trace: {:?}",
        result_trace_content.trace
    );
    println!("==============================");
    println!("==============================");
    println!(
        "expected_trace_content.trace: {:?}",
        expected_trace_content.trace
    );
    println!("==============================");

    assert_eq!(
        result_trace_content.get_span_names(),
        expected_trace_content.get_span_names()
    );

    println!(
        "result_trace_content.get_span_names(): {:?}, expected_trace_content.get_span_names(): {:?}",
        result_trace_content.get_span_names(), expected_trace_content.get_span_names()
    );

    assert_eq!(
        result_trace_content.span_count(),
        expected_trace_content.span_count()
    );

    println!(
        "result_trace_content.span_count(): {:?}, expected_trace_content.span_count(): {:?}",
        result_trace_content.span_count(),
        expected_trace_content.span_count()
    );

    assert_eq!(
        result_trace_content.status_count(2),
        expected_trace_content.status_count(2)
    );

    println!(
        "result_trace_content.status_count(2): {:?}, expected_trace_content.status_count(2): {:?}",
        result_trace_content.status_count(2),
        expected_trace_content.status_count(2)
    );

    unimplemented!("event name, event exception message の比較がまだなので、 todo");
}

#[tokio::test]
async fn check_otlp_output_error_otel_test() {
    // given
    let test_name = "error_otel_test";
    // when
    let original_executor = OriginalTestExecutor::new(test_name);
    let result_path = original_executor.execute().await;

    // then
    let result_trace_content = build_trace_content(&result_path);
    let expected_path = format!("./expected/{}.json", test_name);
    let expected_trace_content = build_trace_content(&expected_path);

    println!("==============================");
    println!(
        "result_trace_content.trace: {:?}",
        result_trace_content.trace
    );
    println!("==============================");
    println!("==============================");
    println!(
        "expected_trace_content.trace: {:?}",
        expected_trace_content.trace
    );
    println!("==============================");

    assert_eq!(
        result_trace_content.get_span_names(),
        expected_trace_content.get_span_names()
    );

    println!(
        "result_trace_content.get_span_names(): {:?}, expected_trace_content.get_span_names(): {:?}",
        result_trace_content.get_span_names(), expected_trace_content.get_span_names()
    );

    assert_eq!(
        result_trace_content.span_count(),
        expected_trace_content.span_count()
    );

    println!(
        "result_trace_content.span_count(): {:?}, expected_trace_content.span_count(): {:?}",
        result_trace_content.span_count(),
        expected_trace_content.span_count()
    );

    assert_eq!(
        result_trace_content.status_count(2),
        expected_trace_content.status_count(2)
    );

    println!(
        "result_trace_content.status_count(2): {:?}, expected_trace_content.status_count(2): {:?}",
        result_trace_content.status_count(2),
        expected_trace_content.status_count(2)
    );
    unimplemented!("event name, event exception message の比較がまだなので、 todo");
}

#[tokio::test]
async fn check_otlp_output_panic_otel_test() {
    // given
    let test_name = "panic_otel_test";
    // when
    let original_executor = OriginalTestExecutor::new(test_name);
    let result_path = original_executor.execute().await;

    // then
    let result_trace_content = build_trace_content(&result_path);
    let expected_path = format!("./expected/{}.json", test_name);
    let expected_trace_content = build_trace_content(&expected_path);

    println!("==============================");
    println!(
        "result_trace_content.trace: {:?}",
        result_trace_content.trace
    );
    println!("==============================");
    println!("==============================");
    println!(
        "expected_trace_content.trace: {:?}",
        expected_trace_content.trace
    );
    println!("==============================");

    assert_eq!(
        result_trace_content.get_span_names(),
        expected_trace_content.get_span_names()
    );

    println!(
        "result_trace_content.get_span_names(): {:?}, expected_trace_content.get_span_names(): {:?}",
        result_trace_content.get_span_names(), expected_trace_content.get_span_names()
    );

    assert_eq!(
        result_trace_content.span_count(),
        expected_trace_content.span_count()
    );

    println!(
        "result_trace_content.span_count(): {:?}, expected_trace_content.span_count(): {:?}",
        result_trace_content.span_count(),
        expected_trace_content.span_count()
    );

    assert_eq!(
        result_trace_content.status_count(2),
        expected_trace_content.status_count(2)
    );

    println!(
        "result_trace_content.status_count(2): {:?}, expected_trace_content.status_count(2): {:?}",
        result_trace_content.status_count(2),
        expected_trace_content.status_count(2)
    );
    // unimplemented!("event name, event exception message の比較がまだなので、 todo");
}

#[tokio::test]
async fn check_otlp_output_succeed_otel_test() {
    // given
    let test_name = "succeed_otel_test";
    // when
    let original_executor = OriginalTestExecutor::new(test_name);
    let result_path = original_executor.execute().await;

    // then
    let result_trace_content = build_trace_content(&result_path);
    let expected_path = format!("./expected/{}.json", test_name);
    let expected_trace_content = build_trace_content(&expected_path);

    // assert_eq!(result_trace_content, expected_trace_content);
    println!("==============================");
    println!(
        "result_trace_content.trace: {:?}",
        result_trace_content.trace
    );
    println!("==============================");
    println!("==============================");
    println!(
        "expected_trace_content.trace: {:?}",
        expected_trace_content.trace
    );
    println!("==============================");

    assert_eq!(
        result_trace_content.get_span_names(),
        expected_trace_content.get_span_names()
    );

    println!(
        "result_trace_content.get_span_names(): {:?}, expected_trace_content.get_span_names(): {:?}",
        result_trace_content.get_span_names(), expected_trace_content.get_span_names()
    );

    assert_eq!(
        result_trace_content.span_count(),
        expected_trace_content.span_count()
    );

    println!(
        "result_trace_content.span_count(): {:?}, expected_trace_content.span_count(): {:?}",
        result_trace_content.span_count(),
        expected_trace_content.span_count()
    );

    assert_eq!(
        result_trace_content.status_count(2),
        expected_trace_content.status_count(2)
    );

    println!(
        "result_trace_content.status_count(2): {:?}, expected_trace_content.status_count(2): {:?}",
        result_trace_content.status_count(2),
        expected_trace_content.status_count(2)
    );
}

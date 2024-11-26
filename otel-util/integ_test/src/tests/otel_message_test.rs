const CONTAINER_RESULT_PATH: &str = "/result.json";

use super::trace_equivalency::TraceContent;
use opentelemetry_proto::tonic::{
    resource,
    trace::v1::{ResourceSpans, TracesData},
};
use std::{
    io::{self, BufRead},
    os::unix::fs::PermissionsExt,
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
    // println!("traces_data: {:?}", traces_data);
    let resource_spans: Vec<ResourceSpans> = traces_data
        .into_iter()
        .flat_map(|trace_data| trace_data.resource_spans)
        .collect();
    // println!("resource_spans: {:?}", resource_spans);

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

    assert_eq!(result_trace_content, expected_trace_content);
    unimplemented!("expected と、result の比較が厳しすぎるので、todo:");
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

    assert_eq!(result_trace_content, expected_trace_content);
    unimplemented!("expected と、result の比較が厳しすぎるので、todo:");
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

    assert_eq!(result_trace_content, expected_trace_content);
    unimplemented!("expected と、result の比較が厳しすぎるので、todo:");
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

    assert_eq!(result_trace_content, expected_trace_content);
    unimplemented!("expected と、result の比較が厳しすぎるので、todo:");
}

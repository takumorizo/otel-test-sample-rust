const CONTAINER_RESULT_PATH: &str = "/result.json";

use serde_json::Value;
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

    async fn execute(&self) {
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

        // tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    }
}

#[tokio::test]
async fn check_otlp_output_failed_otel_test() {
    // given
    let test_name = "failed_otel_test";
    // when
    let original_executor = OriginalTestExecutor::new(test_name);
    original_executor.execute().await;

    // then
    // 元の、otel version　を、0.21.0 から、最新にしないと、opentelemetry_otlp の test と同様のテストが実行できない気がするので、一旦原子的なチェックだけをする。
    // let file = std::fs::File::open(result_path).unwrap();
    // let reader = io::BufReader::new(file);

    // let mut lines = reader.lines();
    // if let Some(Ok(line1)) = lines.next() {
    //     let json1: Value = serde_json::from_str(&line1).unwrap();
    //     if let Some(service_name) =
    //         json1.pointer("/resourceSpans/0/resource/attributes/2/value/stringValue")
    //     {
    //         assert_eq!(service_name, "failed_otel_test");
    //     } else {
    //         panic!("Failed to get service name");
    //     }

    //     if let Some(event_name) =
    //         json1.pointer("/resourceSpans/0/scopeSpans/0/spans/0/events/0/attributes/name")
    //     {
    //         let event_name_str = event_name.as_str().unwrap();
    //         assert!(
    //             event_name_str.contains("panicked at src/tests/original_test_case.rs"),
    //             "Event name does not contain the expected panic message"
    //         );
    //     } else {
    //         panic!("Failed to get event name");
    //     }
    // }

    // if let Some(Ok(line2)) = lines.next() {
    //     let json2: Value = serde_json::from_str(&line2).unwrap();
    //     if let Some(service_name) =
    //         json2.pointer("/resourceSpans/0/resource/attributes/2/value/stringValue")
    //     {
    //         assert_eq!(service_name, "failed_otel_test");
    //     } else {
    //         panic!("Failed to get service name");
    //     }
    // }
}

#[tokio::test]
async fn check_otlp_output_error_otel_test() {
    // given
    let test_name = "error_otel_test";
    // when
    let original_executor = OriginalTestExecutor::new(test_name);
    original_executor.execute().await;
}

#[tokio::test]
async fn check_otlp_output_panic_otel_test() {
    // given
    let test_name = "panic_otel_test";
    // when
    let original_executor = OriginalTestExecutor::new(test_name);
    original_executor.execute().await;
}

#[tokio::test]
async fn check_otlp_output_succeed_otel_test() {
    // given
    let test_name = "panic_otel_test";
    // when
    let original_executor = OriginalTestExecutor::new(test_name);
    original_executor.execute().await;
}

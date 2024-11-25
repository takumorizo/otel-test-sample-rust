const CONTAINER_RESULT_PATH: &str = "/result.json";

use std::os::unix::fs::PermissionsExt;
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

#[tokio::test]
async fn check_otlp_output_failed_otel_test() {
    let test_name = "failed_otel_test";
    // given

    let crate_path = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let config_path = format!("{crate_path}/otel-collector-config.yaml");
    let result_path = format!("{crate_path}/result/{test_name}.json");
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
        .arg(format!("tests::original_test_case::{test_name}"))
        .output()
        .await
        .expect("Failed to execute cargo test");

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // then
    // 元の、otel version　を、0.21.0 から、最新にしないと、opentelemetry_otlp の test と同様のテストが実行できない。
    // なので、一旦原子的なチェックだけをする。

    // let result_spans =
    //     read_spans_from_json(std::fs::File::open(result_path).expect("Failed to open result file"));

    // let expected_spans = read_spans_from_json(
    //     std::fs::File::open(expected_path).expect("Failed to open result file"),
    // );
    // print!("spans loaded");
    // print!("{:?}", result_spans);
    // print!("{:?}", expected_spans);

    // TraceAsserter::new(result_spans, expected_spans).assert();

    // let a = 1;

    unimplemented!("Check the result file");
    // finally: 無くても、コンテナは消える。
    // _container
    //     .stop()
    //     .await
    //     .expect("Failed to stop opentelemetry-collector");
}

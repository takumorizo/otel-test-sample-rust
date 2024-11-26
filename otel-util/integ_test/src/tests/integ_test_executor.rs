const CONTAINER_RESULT_PATH: &str = "/result.json";

use std::{os::unix::fs::PermissionsExt, vec};

use testcontainers::{
    core::{AccessMode, IntoContainerPort, Mount},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt, TestcontainersError,
};

pub struct CollectorContainerFactory {
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

pub struct OriginalTestExecutor {
    test_name: String,
}

impl OriginalTestExecutor {
    pub fn new(test_name: &str) -> Self {
        OriginalTestExecutor {
            test_name: test_name.to_string(),
        }
    }

    pub async fn execute(&self) -> String {
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

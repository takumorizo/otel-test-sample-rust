// use std::collections::HashMap;
// use testcontainers::clients::Cli;
// use testcontainers::core::Port;
// use testcontainers::core::WaitFor;
// use testcontainers::images::generic::GenericImage;
// use testcontainers::Image;
// use testcontainers::RunnableImage;

// const COLLECTOR_CONTAINER_NAME: &str = "otel-collector";

// pub struct Collector {
//     volumes: HashMap<String, String>,
// }

// impl Image for Collector {
//     type Args = ();

//     fn name(&self) -> String {
//         "otel/opentelemetry-collector".to_string()
//     }

//     fn tag(&self) -> String {
//         "latest".to_string()
//     }

//     fn ready_conditions(&self) -> Vec<WaitFor> {
//         vec![WaitFor::Nothing]
//     }

//     fn volumes(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
//         Box::new(self.volumes.iter())
//     }

//     fn expose_ports(&self) -> Vec<u16> {
//         vec![
//             // 4317,  // gRPC port, defined in Dockerfile
//             // 4318,  // HTTP port, defined in Dockerfile
//         ]
//     }
// }

// impl Default for Collector {
//     fn default() -> Self {
//         Collector {
//             volumes: HashMap::from([(
//                 "./otel-collector-config.yaml".into(),
//                 "/etc/otelcol/config.yaml".into(),
//             )]),
//         }
//     }
// }

// impl Collector {
//     pub fn with_volume(mut self, src: &str, dst: &str) -> Self {
//         self.volumes.insert(src.into(), dst.into());
//         self
//     }
// }

// #[tokio::test]
// async fn check_otel_trace_failed_otel_test() {
//     // given
//     let mut collector_image = Collector::default();
//     let docker = Cli::default();
//     let mut image =
//         RunnableImage::from(collector_image).with_container_name(COLLECTOR_CONTAINER_NAME);

//     for port in [
//         4317, // gRPC port
//         4318, // HTTP port
//     ] {
//         image = image.with_mapped_port(Port {
//             local: port,
//             internal: port,
//         })
//     }

//     let collector_container = docker.run(image);

//     // when
//     tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

//     // then
//     collector_container.stop();
// }

const CONTAINER_RESULT_PATH: &str = "/result.json";
use std::os::unix::fs::PermissionsExt;

use testcontainers::{
    core::{AccessMode, IntoContainerPort, Mount},
    runners::AsyncRunner,
    GenericImage, ImageExt,
};

#[tokio::test]
async fn check_otlp_output_failed_otel_test() {
    let test_name = "failed_otel_test";
    // given
    let crate_path = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .into_owned();

    let result_path = format!("{crate_path}/result/{test_name}.json");
    let file = std::fs::File::create(result_path.clone()).unwrap();
    file.set_permissions(std::fs::Permissions::from_mode(0o666))
        .unwrap();

    println!("Current crate path: {:?}", crate_path);
    let container = GenericImage::new("otel/opentelemetry-collector-contrib", "0.103.1")
        .with_mapped_port(4317, 4317.tcp())
        .with_mapped_port(4318, 4318.tcp())
        .with_mapped_port(13133, 13133.tcp())
        .with_mapped_port(8889, 8889.tcp())
        .with_mount(Mount::bind_mount(
            format!("{crate_path}/otel-collector-config.yaml"),
            "/etc/opentelemetry-collector.yaml",
        ))
        .with_mount(
            Mount::bind_mount(result_path, CONTAINER_RESULT_PATH)
                .with_access_mode(AccessMode::ReadWrite),
        )
        .with_cmd(vec!["--config=/etc/opentelemetry-collector.yaml"])
        .start()
        .await
        .expect("Failed to start opentelemetry-collector");

    // when
    let output = tokio::process::Command::new("cargo")
        .arg("test")
        .arg(format!("tests::original_test_case::{test_name}"))
        .output()
        .await
        .expect("Failed to execute cargo test");

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // then

    unimplemented!("Check the result file");

    // finally
    container
        .stop()
        .await
        .expect("Failed to stop opentelemetry-collector");
}

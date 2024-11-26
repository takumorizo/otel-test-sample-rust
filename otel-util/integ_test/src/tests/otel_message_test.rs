use super::integ_test_executor::OriginalTestExecutor;
use super::trace_contents::TraceContent;
use opentelemetry_proto::tonic::trace::v1::{ResourceSpans, TracesData};
use std::io::{self, BufRead};

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
    let resource_spans: Vec<ResourceSpans> = traces_data
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

use opentelemetry_proto::tonic::trace::v1::ResourceSpans;
use std::vec;

pub trait TraceInfoExtractor {
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

pub trait SpanInfoExtractor {
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
pub struct TraceContent {
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

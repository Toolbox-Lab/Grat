use crate::error::PrismResult;
use crate::types::report::{DiagnosticReport, RootCause, SuggestedFix};
use crate::xdr::codec::XdrCodec;
use stellar_xdr::curr::{ContractEventBody, DiagnosticEvent, ScVal};

pub fn enrich_report(
    report: &mut DiagnosticReport,
    tx_data: &serde_json::Value,
) -> PrismResult<()> {
    if let Some(events_b64) = tx_data
        .get("diagnosticEventsXdr")
        .and_then(|e| e.as_array())
    {
        let mut decoded_events = Vec::new();

        for event_b64 in events_b64 {
            if let Some(b64_str) = event_b64.as_str() {
                if let Ok(event) = DiagnosticEvent::from_xdr_base64(b64_str) {
                    analyze_diagnostic_event(report, &event);
                    decoded_events.push(event);
                }
            }
        }

        if let Some(error_event) = deepest_error_event(&decoded_events) {
            add_deepest_error_root_cause(report, &error_event);
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeepestErrorEvent {
    depth: usize,
    topics: Vec<String>,
    payload: String,
}

fn scval_to_string(val: &ScVal) -> Option<String> {
    match val {
        ScVal::Symbol(sym) => Some(sym.to_string()),
        ScVal::String(s) => Some(s.to_string()),
        ScVal::U32(u) => Some(u.to_string()),
        ScVal::I32(i) => Some(i.to_string()),
        ScVal::U64(u) => Some(u.to_string()),
        ScVal::I64(i) => Some(i.to_string()),
        _ => None,
    }
}

fn scval_to_payload_string(val: &ScVal) -> Option<String> {
    scval_to_string(val).or_else(|| match val {
        ScVal::Void => None,
        _ => Some(format!("{val:?}")),
    })
}

fn normalized_topic(topic: &str) -> String {
    topic.trim().to_ascii_lowercase()
}

fn is_call_topic(topic: &str) -> bool {
    matches!(
        normalized_topic(topic).as_str(),
        "fn_call" | "function_call" | "call"
    )
}

fn is_return_topic(topic: &str) -> bool {
    matches!(
        normalized_topic(topic).as_str(),
        "fn_return" | "function_return" | "return"
    )
}

fn contains_error_signal(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("error")
        || lower == "err"
        || lower.contains("failed")
        || lower.contains("failure")
        || lower.contains("panic")
        || lower.contains("trap")
}

fn error_payload(topics: &[String], data: &ScVal) -> Option<String> {
    let data_payload = scval_to_payload_string(data);

    if let Some(payload) = &data_payload {
        if contains_error_signal(payload) {
            return Some(payload.clone());
        }
    }

    if topics.iter().any(|topic| contains_error_signal(topic)) {
        return data_payload.or_else(|| Some(topics.join(" > ")));
    }

    None
}

#[allow(irrefutable_let_patterns)]
fn deepest_error_event(events: &[DiagnosticEvent]) -> Option<DeepestErrorEvent> {
    let mut depth = 0usize;
    let mut deepest: Option<DeepestErrorEvent> = None;

    for event in events {
        if let ContractEventBody::V0(v0) = &event.event.body {
            let topics: Vec<String> = v0.topics.iter().filter_map(scval_to_string).collect();

            if topics.iter().any(|topic| is_call_topic(topic)) {
                depth += 1;
            }

            if let Some(payload) = error_payload(&topics, &v0.data) {
                let candidate = DeepestErrorEvent {
                    depth,
                    topics: topics.clone(),
                    payload,
                };

                let should_replace = match &deepest {
                    Some(current) => candidate.depth >= current.depth,
                    None => true,
                };

                if should_replace {
                    deepest = Some(candidate);
                }
            }

            if topics.iter().any(|topic| is_return_topic(topic)) {
                depth = depth.saturating_sub(1);
            }
        }
    }

    deepest
}

fn add_deepest_error_root_cause(report: &mut DiagnosticReport, error_event: &DeepestErrorEvent) {
    let topics = if error_event.topics.is_empty() {
        "untagged diagnostic event".to_string()
    } else {
        error_event.topics.join(" > ")
    };
    let description = format!(
        "Deepest diagnostic error occurred at call depth {} in [{}] with payload: {}.",
        error_event.depth, topics, error_event.payload
    );

    if !report
        .root_causes
        .iter()
        .any(|cause| cause.description.contains("Deepest diagnostic error"))
    {
        report.root_causes.push(RootCause {
            description: description.clone(),
            likelihood: "high".to_string(),
        });
    }

    let detail = format!("- Deepest error event: {description}");
    if !report.detailed_explanation.contains(&detail) {
        if report.detailed_explanation.is_empty() {
            report.detailed_explanation = format!("Diagnostic events trace:\n{detail}");
        } else {
            report.detailed_explanation.push('\n');
            report.detailed_explanation.push_str(&detail);
        }
    }
}

#[allow(irrefutable_let_patterns)]
fn analyze_diagnostic_event(report: &mut DiagnosticReport, event: &DiagnosticEvent) {
    if let ContractEventBody::V0(v0) = &event.event.body {
        let topics: Vec<String> = v0.topics.iter().filter_map(scval_to_string).collect();
        if topics.is_empty() {
            return;
        }

        if topics
            .iter()
            .any(|t| t.to_lowercase().contains("budget") || t.to_lowercase().contains("limit"))
        {
            if !report
                .root_causes
                .iter()
                .any(|c| c.description.contains("Resource limit"))
            {
                report.root_causes.push(RootCause {
                    description: "Resource limit was exceeded during contract execution."
                        .to_string(),
                    likelihood: "high".to_string(),
                });
            }
            if !report
                .suggested_fixes
                .iter()
                .any(|f| f.id == "increase_limits")
            {
                report.suggested_fixes.push(SuggestedFix {
                    description:
                        "Increase the resource limits when simulating/submitting the transaction."
                            .to_string(),
                    difficulty: "easy".to_string(),
                    requires_upgrade: false,
                    example: None,
                    id: "increase_limits".to_string(),
                    remedy_code: None,
                });
            }
        }

        if topics
            .iter()
            .any(|t| t.to_lowercase().contains("storage") || t.to_lowercase().contains("footprint"))
        {
            if !report
                .root_causes
                .iter()
                .any(|c| c.description.contains("footprint"))
            {
                report.root_causes.push(RootCause {
                    description: "The contract accessed or requested a storage key that was not declared in the footprint.".to_string(),
                    likelihood: "high".to_string(),
                });
            }
            if !report
                .suggested_fixes
                .iter()
                .any(|f| f.id == "resimulate_footprint")
            {
                report.suggested_fixes.push(SuggestedFix {
                    description: "Re-simulate the transaction to capture the correct footprint keys and footprint declaration.".to_string(),
                    difficulty: "easy".to_string(),
                    requires_upgrade: false,
                    example: None,
                    id: "resimulate_footprint".to_string(),
                    remedy_code: None,
                });
            }
        }

        if topics
            .iter()
            .any(|t| t.to_lowercase().contains("auth") || t.to_lowercase().contains("signature"))
        {
            if !report
                .root_causes
                .iter()
                .any(|c| c.description.contains("authorization"))
            {
                report.root_causes.push(RootCause {
                    description: "Transaction verification or authorization check failed in __check_auth or signature check.".to_string(),
                    likelihood: "high".to_string(),
                });
            }
            if !report
                .suggested_fixes
                .iter()
                .any(|f| f.id == "check_auth_signatures")
            {
                report.suggested_fixes.push(SuggestedFix {
                    description: "Check that the transaction signatures match the required signers and the nonce is correct.".to_string(),
                    difficulty: "medium".to_string(),
                    requires_upgrade: false,
                    example: None,
                    id: "check_auth_signatures".to_string(),
                    remedy_code: None,
                });
            }
        }

        let topics_str = topics.join(" > ");
        if !report.detailed_explanation.contains(&topics_str) {
            if report.detailed_explanation.is_empty() {
                report.detailed_explanation =
                    format!("Diagnostic events trace:\n- [{}]", topics_str);
            } else {
                report
                    .detailed_explanation
                    .push_str(&format!("\n- [{}]", topics_str));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use stellar_xdr::curr::{
        ContractEvent, ContractEventBody, ContractEventType, ContractEventV0, DiagnosticEvent,
        ExtensionPoint, ScVal,
    };

    fn diagnostic_event(topic: &str, data: ScVal) -> DiagnosticEvent {
        DiagnosticEvent {
            in_successful_contract_call: false,
            event: ContractEvent {
                ext: ExtensionPoint::V0,
                contract_id: None,
                type_: ContractEventType::Diagnostic,
                body: ContractEventBody::V0(ContractEventV0 {
                    topics: vec![ScVal::Symbol(topic.try_into().unwrap())]
                        .try_into()
                        .unwrap(),
                    data,
                }),
            },
        }
    }

    fn event_b64(topic: &str, data: ScVal) -> String {
        let event = diagnostic_event(topic, data);
        XdrCodec::to_xdr_base64(&event).expect("encode diagnostic event")
    }

    #[test]
    fn deepest_error_event_prefers_nested_payload() {
        let events = vec![
            diagnostic_event("fn_call", ScVal::Void),
            diagnostic_event("error", ScVal::Symbol("outer".try_into().unwrap())),
            diagnostic_event("fn_call", ScVal::Void),
            diagnostic_event("error", ScVal::Symbol("inner".try_into().unwrap())),
            diagnostic_event("fn_return", ScVal::Void),
            diagnostic_event("fn_return", ScVal::Void),
        ];

        let deepest = deepest_error_event(&events).expect("deepest error event");

        assert_eq!(deepest.depth, 2);
        assert_eq!(deepest.payload, "inner");
        assert_eq!(deepest.topics, vec!["error".to_string()]);
    }

    #[test]
    fn enrich_report_adds_deepest_error_root_cause() {
        let tx_data = json!({
            "diagnosticEventsXdr": [
                event_b64("fn_call", ScVal::Void),
                event_b64("error", ScVal::Symbol("outer".try_into().unwrap())),
                event_b64("fn_call", ScVal::Void),
                event_b64("error", ScVal::Symbol("inner".try_into().unwrap())),
                event_b64("fn_return", ScVal::Void),
                event_b64("fn_return", ScVal::Void),
            ]
        });
        let mut report = DiagnosticReport::new("contract", 1, "HostError", "failure");

        enrich_report(&mut report, &tx_data).expect("enrich report");

        assert!(report.root_causes.iter().any(|cause| {
            cause.description.contains("call depth 2") && cause.description.contains("inner")
        }));
        assert!(report.detailed_explanation.contains("Deepest error event"));
        assert!(report.detailed_explanation.contains("inner"));
    }
}

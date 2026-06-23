

use crate::error::PrismResult;
use crate::types::report::{DiagnosticReport, RootCause, SuggestedFix};
use crate::xdr::codec::XdrCodec;
use stellar_xdr::curr::{DiagnosticEvent, ContractEventBody, ScVal};

pub fn enrich_report(
    report: &mut DiagnosticReport,
    tx_data: &serde_json::Value,
) -> PrismResult<()> {
    if let Some(events_b64) = tx_data.get("diagnosticEventsXdr").and_then(|e| e.as_array()) {
        for event_b64 in events_b64 {
            if let Some(b64_str) = event_b64.as_str() {
                if let Ok(event) = DiagnosticEvent::from_xdr_base64(b64_str) {
                    analyze_diagnostic_event(report, &event);
                }
            }
        }
    }

    Ok(())
}

fn scval_to_string(val: &ScVal) -> Option<String> {
    match val {
        ScVal::Symbol(sym) => Some(sym.to_string()),
        ScVal::String(s) => Some(s.to_string()),
        ScVal::U32(u) => Some(u.to_string()),
        ScVal::I32(i) => Some(i.to_string()),
        ScVal::U64(u) => Some(u.to_string()),
        ScVal::I64(i) => Some(i.to_string()),
        ScVal::Void => Some("Void".to_string()),
        ScVal::Bool(b) => Some(b.to_string()),
        ScVal::U128(u) => {
            let num = ((u.hi as u128) << 64) | (u.lo as u128);
            Some(num.to_string())
        }
        ScVal::I128(i) => {
            let num = ((i.hi as i128) << 64) | (i.lo as u128 as i128);
            Some(num.to_string())
        }
        ScVal::Vec(Some(v)) => {
            let items: Vec<String> = v.iter().map(|item| {
                scval_to_string(item).unwrap_or_else(|| "?".to_string())
            }).collect();
            Some(format!("[{}]", items.join(", ")))
        }
        ScVal::Map(Some(m)) => {
            let items: Vec<String> = m.iter().map(|entry| {
                let k = scval_to_string(&entry.key).unwrap_or_else(|| "?".to_string());
                let v = scval_to_string(&entry.val).unwrap_or_else(|| "?".to_string());
                format!("{}: {}", k, v)
            }).collect();
            Some(format!("{{{}}}", items.join(", ")))
        }
        _ => None,
    }
}

#[allow(irrefutable_let_patterns)]
fn analyze_diagnostic_event(report: &mut DiagnosticReport, event: &DiagnosticEvent) {
    if let ContractEventBody::V0(v0) = &event.event.body {
        let topics: Vec<String> = v0.topics.iter().filter_map(scval_to_string).collect();
        if topics.is_empty() {
            return;
        }

        if topics.iter().any(|t| t.to_lowercase().contains("budget") || t.to_lowercase().contains("limit")) {
            if !report.root_causes.iter().any(|c| c.description.contains("Resource limit")) {
                report.root_causes.push(RootCause {
                    description: "Resource limit was exceeded during contract execution.".to_string(),
                    likelihood: "high".to_string(),
                });
            }
            if !report.suggested_fixes.iter().any(|f| f.id == "increase_limits") {
                report.suggested_fixes.push(SuggestedFix {
                    description: "Increase the resource limits when simulating/submitting the transaction.".to_string(),
                    difficulty: "easy".to_string(),
                    requires_upgrade: false,
                    example: None,
                    id: "increase_limits".to_string(),
                    remedy_code: None,
                });
            }
        }

        if topics.iter().any(|t| t.to_lowercase().contains("storage") || t.to_lowercase().contains("footprint")) {
            if !report.root_causes.iter().any(|c| c.description.contains("footprint")) {
                report.root_causes.push(RootCause {
                    description: "The contract accessed or requested a storage key that was not declared in the footprint.".to_string(),
                    likelihood: "high".to_string(),
                });
            }
            if !report.suggested_fixes.iter().any(|f| f.id == "resimulate_footprint") {
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

        if topics.iter().any(|t| t.to_lowercase().contains("auth") || t.to_lowercase().contains("signature")) {
            if !report.root_causes.iter().any(|c| c.description.contains("authorization")) {
                report.root_causes.push(RootCause {
                    description: "Transaction verification or authorization check failed in __check_auth or signature check.".to_string(),
                    likelihood: "high".to_string(),
                });
            }
            if !report.suggested_fixes.iter().any(|f| f.id == "check_auth_signatures") {
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
                report.detailed_explanation = format!("Diagnostic events trace:\n- [{}]", topics_str);
            } else {
                report.detailed_explanation.push_str(&format!("\n- [{}]", topics_str));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::curr::{ScVal, ScVec, ScMap, ScMapEntry, Int128Parts};

    #[test]
    fn test_scval_to_string_supported_variants() {
        assert_eq!(scval_to_string(&ScVal::String("hello".try_into().unwrap())), Some("hello".to_string()));
        assert_eq!(scval_to_string(&ScVal::Bool(true)), Some("true".to_string()));
        assert_eq!(scval_to_string(&ScVal::Bool(false)), Some("false".to_string()));
        assert_eq!(scval_to_string(&ScVal::I32(-2147483648)), Some("-2147483648".to_string()));
        assert_eq!(scval_to_string(&ScVal::I32(2147483647)), Some("2147483647".to_string()));
        assert_eq!(scval_to_string(&ScVal::U64(18446744073709551615)), Some("18446744073709551615".to_string()));
        assert_eq!(scval_to_string(&ScVal::I64(-9223372036854775808)), Some("-9223372036854775808".to_string()));
        assert_eq!(scval_to_string(&ScVal::I64(9223372036854775807)), Some("9223372036854775807".to_string()));
    }

    #[test]
    fn test_scval_to_string_empty_args() {
        let empty_vec = ScVal::Vec(Some(ScVec(vec![].try_into().unwrap())));
        assert_eq!(scval_to_string(&empty_vec), Some("[]".to_string()));
        let void_val = ScVal::Void;
        assert_eq!(scval_to_string(&void_val), Some("Void".to_string()));
    }

    #[test]
    fn test_scval_to_string_large_integers() {
        // U128 standard
        let u128_val = ScVal::U128(Int128Parts { hi: 1, lo: 0 });
        assert_eq!(scval_to_string(&u128_val), Some("18446744073709551616".to_string()));

        // I128 standard
        let i128_val = ScVal::I128(Int128Parts { hi: -1i64, lo: 0 });
        assert_eq!(scval_to_string(&i128_val), Some("-18446744073709551616".to_string()));

        // U128 Max: hi is -1 (all 1s), lo is u64::MAX
        let u128_max = ScVal::U128(Int128Parts { hi: -1i64, lo: u64::MAX });
        assert_eq!(scval_to_string(&u128_max), Some("340282366920938463463374607431768211455".to_string()));

        // U128 Min: 0
        let u128_min = ScVal::U128(Int128Parts { hi: 0, lo: 0 });
        assert_eq!(scval_to_string(&u128_min), Some("0".to_string()));

        // I128 Max: hi is i64::MAX, lo is u64::MAX
        let i128_max = ScVal::I128(Int128Parts { hi: i64::MAX, lo: u64::MAX });
        assert_eq!(scval_to_string(&i128_max), Some("170141183460469231731687303715884105727".to_string()));

        // I128 Min: hi is i64::MIN, lo is 0
        let i128_min = ScVal::I128(Int128Parts { hi: i64::MIN, lo: 0 });
        assert_eq!(scval_to_string(&i128_min), Some("-170141183460469231731687303715884105728".to_string()));
    }

    #[test]
    fn test_scval_to_string_nested_map() {
        let map_entry = ScMapEntry {
            key: ScVal::Symbol("key".try_into().unwrap()),
            val: ScVal::U32(42),
        };
        let scmap = ScMap(vec![map_entry].try_into().unwrap());
        let nested_map = ScVal::Map(Some(scmap));
        let vec_val = ScVal::Vec(Some(ScVec(vec![nested_map].try_into().unwrap())));
        assert_eq!(scval_to_string(&vec_val), Some("[{key: 42}]".to_string()));
    }
}

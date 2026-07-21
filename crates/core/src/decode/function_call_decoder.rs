//! `FunctionCallDecoder`
//!
//! Reconstructs the exact function call a Soroban transaction initiated.
//!
//! Every Soroban transaction begins with a Host Function Invocation: the raw
//! XDR command telling the VM which function to run on which contract with
//! which arguments. In the raw XDR, the arguments arrive as an untyped
//! `SCVec` — just a bag of bytes with no field names attached. This module
//! turns that untyped bag back into a labeled, human-readable call, e.g.
//! `transfer(from: "G...", to: "G...", amount: 1000)`.

use crate::decode::scval_to_json::scval_to_json;
use crate::error::GratError;
use crate::spec::ContractSpec;

use serde::Serialize;
use serde_json::Value as JsonValue;
use stellar_xdr::curr::{HostFunction, InvokeContractArgs, ScVal};

/// One decoded, labeled argument: the parameter name from the contract's
/// spec, paired with its JSON-decoded value.
#[derive(Debug, Clone, Serialize)]
pub struct DecodedArgument {
    pub name: String,
    pub value: JsonValue,
}

/// The fully reconstructed function call: which function was invoked, and
/// its labeled arguments in declaration order.
#[derive(Debug, Clone, Serialize)]
pub struct DecodedFunctionCall {
    pub function_name: String,
    pub arguments: Vec<DecodedArgument>,
}

impl DecodedFunctionCall {
    /// Renders e.g. `transfer(from: "G...", to: "G...", amount: 1000)`
    pub fn pretty_print(&self) -> String {
        let args = self
            .arguments
            .iter()
            .map(|a| format!("{}: {}", a.name, a.value))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}({})", self.function_name, args)
    }
}

pub struct FunctionCallDecoder;

impl FunctionCallDecoder {
    /// Decode a raw `HostFunction` invocation into a labeled function-call
    /// summary, using the contract's parsed `ContractSpec` to resolve the
    /// target function's signature.
    ///
    /// # Errors
    /// - `GratError::ArityMismatch` if the number of raw arguments doesn't
    ///   match the number of parameters declared for this function.
    /// - `GratError::SpecError` if the function name can't be found in the
    ///   contract's spec.
    /// - `GratError::UnsupportedHostFunction` if the host function isn't an
    ///   `InvokeContract` invocation.
    pub fn decode(
        host_function: &HostFunction,
        spec: &ContractSpec,
    ) -> Result<DecodedFunctionCall, GratError> {
        let invoke_args: &InvokeContractArgs = match host_function {
            HostFunction::InvokeContract(args) => args,
            other => {
                return Err(GratError::UnsupportedHostFunction(format!(
                    "FunctionCallDecoder only supports InvokeContract host functions, got: {other:?}"
                )));
            }
        };

        let function_name =
            String::from_utf8_lossy(invoke_args.function_name.as_ref()).into_owned();
        let raw_args: &[ScVal] = invoke_args.args.as_slice();

        // Look up this function's declared signature in the contract's spec.
        let contract_function = spec
            .functions
            .iter()
            .find(|f| f.name == function_name)
            .ok_or_else(|| {
                GratError::SpecError(format!(
                    "function '{function_name}' not found in contract spec"
                ))
            })?;

        let expected_params = &contract_function.params;

        // Arity check first, before we try to zip/decode anything — a
        // mismatched count means the zip below would silently truncate,
        // which is exactly the failure mode this decoder exists to catch.
        if expected_params.len() != raw_args.len() {
            return Err(GratError::ArityMismatch {
                function_name: function_name.clone(),
                expected: expected_params.len(),
                actual: raw_args.len(),
            });
        }

        let arguments = expected_params
            .iter()
            .zip(raw_args.iter())
            .map(|((param_name, _param_type), raw_val)| DecodedArgument {
                name: param_name.clone(),
                value: scval_to_json(raw_val),
            })
            .collect();

        Ok(DecodedFunctionCall {
            function_name,
            arguments,
        })
    }
}
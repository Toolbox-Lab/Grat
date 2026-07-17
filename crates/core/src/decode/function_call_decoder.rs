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
//!
//! Pipeline:
//!   1. Extract `function_name` (an `ScSymbol`) and the raw `args` (`ScVec`)
//!      from the `HostFunction` invocation.
//!   2. Ask `SpecParser` for the contract's `ContractFunction` metadata that
//!      matches `function_name`.
//!   3. Zip the function's expected parameter list against the raw argument
//!      array, position by position.
//!   4. For each pair, run the raw `ScVal` through the SCVal-to-JSON
//!      converter and attach the expected parameter's name to the result.
//!   5. If the argument count doesn't match the expected parameter count,
//!      stop and return `GratError::ArityMismatch` with enough detail to
//!      pinpoint the mismatch.

use crate::error::GratError;
use crate::spec::SpecParser;
use crate::decode::scval_json::scval_to_json; // adjust path if the converter lives elsewhere

use soroban_env_host::xdr::{HostFunction, InvokeContractArgs, ScVal};
use serde::Serialize;
use serde_json::Value as JsonValue;

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

                pub struct FunctionCallDecoder;

                impl FunctionCallDecoder {
                    /// Decode a raw `HostFunction` invocation into a labeled function-call
                        /// summary, using `spec_parser` to resolve the target contract's
                            /// function signature.
                                ///
                                    /// # Errors
                                        /// - `GratError::ArityMismatch` if the number of raw arguments doesn't
                                            ///   match the number of parameters declared for this function.
                                                /// - Propagates whatever `GratError` variant `SpecParser` returns if the
                                                    ///   function name can't be resolved against the contract's spec.
                                                        pub fn decode(
                                                                host_function: &HostFunction,
                                                                        spec_parser: &SpecParser,
                                                                            ) -> Result<DecodedFunctionCall, GratError> {
                                                                                    let invoke_args: &InvokeContractArgs = match host_function {
                                                                                                HostFunction::InvokeContract(args) => args,
                                                                                                            other => {
                                                                                                                            return Err(GratError::UnsupportedHostFunction(format!(
                                                                                                                                                "FunctionCallDecoder only supports InvokeContract host functions, got: {other:?}"
                                                                                                                                                                )));
                                                                                                                                                                            }
                                                                                                                                                                                    };

                                                                                                                                                                                            let function_name = invoke_args.function_name.to_string();
                                                                                                                                                                                                    let raw_args: &[ScVal] = invoke_args.args.as_slice();

                                                                                                                                                                                                            // Ask the spec for this function's expected signature.
                                                                                                                                                                                                                    let contract_function = spec_parser.get_function(&function_name)?;
                                                                                                                                                                                                                            let expected_params = &contract_function.parameters;

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
                                                                                                                                                                                                                                                                                                                                                                                        .map(|(param, raw_val)| DecodedArgument {
                                                                                                                                                                                                                                                                                                                                                                                                        name: param.name.clone(),
                                                                                                                                                                                                                                                                                                                                                                                                                        value: scval_to_json(raw_val),
                                                                                                                                                                                                                                                                                                                                                                                                                                    })
                                                                                                                                                                                                                                                                                                                                                                                                                                                .collect();

                                                                                                                                                                                                                                                                                                                                                                                                                                                        Ok(DecodedFunctionCall {
                                                                                                                                                                                                                                                                                                                                                                                                                                                                    function_name,
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                arguments,
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        })
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            }
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            }
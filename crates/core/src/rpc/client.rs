//! Soroban RPC client.
//!
//! Communicates with Soroban RPC endpoints: `getTransaction`, `simulateTransaction`,
//! `getLedgerEntries`, `getEvents`, `getLatestLedger`. Handles retries and
//! basic rate-limit backoff.

use crate::error::{PrismError, PrismResult};
use crate::network::NetworkConfig;
use crate::rpc::jsonrpc::{JsonRpcRequest, JsonRpcResponse};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

// ── simulateTransaction response types ──────────────────────────────────────

/// Ledger footprint returned by `simulateTransaction`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulateFootprint {
    #[serde(rename = "readOnly", default)]
    pub read_only: Vec<String>,
    #[serde(rename = "readWrite", default)]
    pub read_write: Vec<String>,
}

/// Authorization entry returned by `simulateTransaction`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulateAuthEntry {
    pub xdr: String,
}

/// Resource cost estimates returned by `simulateTransaction`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulateCost {
    #[serde(rename = "cpuInsns", default)]
    pub cpu_insns: String,
    #[serde(rename = "memBytes", default)]
    pub mem_bytes: String,
}

/// Soroban resource limits and fees returned by `simulateTransaction`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulateSorobanData {
    pub data: String,
    #[serde(rename = "minResourceFee")]
    pub min_resource_fee: String,
}

/// Typed response from the `simulateTransaction` RPC method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulateTransactionResponse {
    #[serde(rename = "latestLedger")]
    pub latest_ledger: u32,
    #[serde(rename = "transactionData", default)]
    pub soroban_data: Option<String>,
    #[serde(rename = "minResourceFee", default)]
    pub min_resource_fee: Option<String>,
    #[serde(default)]
    pub auth: Vec<String>,
    #[serde(default)]
    pub results: Vec<SimulateResult>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub cost: Option<SimulateCost>,
}

/// Invocations result in a `simulateTransaction` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulateResult {
    #[serde(default)]
    pub xdr: String,
    #[serde(default)]
    pub auth: Vec<String>,
}

impl SimulateTransactionResponse {
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    pub fn return_value_xdr(&self) -> Option<&str> {
        self.results.first().map(|r| r.xdr.as_str())
    }
}

/// Primary entry point for Soroban network communication.
#[derive(Debug, Clone)]
pub struct SorobanRpcClient {
    /// HTTP client instance.
    client: reqwest::Client,
    /// Soroban RPC endpoint URL.
    rpc_url: String,
}

/// Transaction status in Soroban.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionStatus {
    Success,
    NotFound,
    Failed,
}

/// Response for the `getTransaction` RPC method.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTransactionResponse {
    pub status: TransactionStatus,
    pub latest_ledger: u32,
    pub latest_ledger_close_time: Option<u64>,
    pub oldest_ledger: Option<u32>,
    pub oldest_ledger_close_time: Option<u64>,
    pub ledger: Option<u32>,
    pub created_at: Option<String>,
    pub application_order: Option<u32>,
    pub fee_bump: Option<String>,
    pub envelope_xdr: Option<String>,
    pub result_xdr: Option<String>,
    pub result_meta_xdr: Option<String>,
}

impl SorobanRpcClient {
    /// Create a new `SorobanRpcClient` from a [`NetworkConfig`].
    ///
    /// Initialises a [`reqwest::Client`] matching the config's timeout or defaults to 30s.
    pub fn new(config: &NetworkConfig) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .default_headers(headers)
            .build()
            .expect("Failed to build reqwest client");

        Self {
            client,
            rpc_url: config.rpc_url.clone(),
        }
    }

    /// Update the timeout for the client in seconds.
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        self.client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .default_headers(headers)
            .build()
            .expect("Failed to build reqwest client");
        self
    }

    /// Fetch a transaction by hash.
    pub async fn get_transaction(&self, tx_hash: &str) -> PrismResult<GetTransactionResponse> {
        let params = serde_json::json!([tx_hash]);
        self.call("getTransaction", params).await
    }

    /// Simulate a transaction against the current ledger state.
    ///
    /// Fires the `simulateTransaction` JSON-RPC method and returns a typed
    /// [`SimulateTransactionResponse`] containing:
    /// - `soroban_data` — the `SorobanTransactionData` XDR to stamp onto the
    ///   transaction before submission (footprint + resource limits).
    /// - `min_resource_fee` — the minimum fee in stroops required.
    /// - `auth` — authorization entries that must be signed by the relevant
    ///   parties before the transaction is submitted.
    /// - `results` — per-invocation return values.
    ///
    /// If the node returns an `error` field the method returns
    /// [`PrismError::RpcError`] so callers can surface the simulation failure
    /// without having to inspect the raw JSON.
    ///
    /// # Arguments
    /// * `tx_xdr` — base64-encoded XDR of the unsigned `TransactionEnvelope`.
    pub async fn simulate_transaction(
        &self,
        tx_xdr: &str,
    ) -> PrismResult<SimulateTransactionResponse> {
        let params = serde_json::json!({ "transaction": tx_xdr });
        let raw = self
            .call::<serde_json::Value>("simulateTransaction", params)
            .await?;

        let response: SimulateTransactionResponse = serde_json::from_value(raw).map_err(|e| {
            PrismError::RpcError(format!("Failed to parse simulateTransaction response: {e}"))
        })?;

        // Surface simulation-level errors as a proper Rust error so callers
        // don't need to inspect the struct themselves.
        if let Some(ref err) = response.error {
            return Err(PrismError::RpcError(format!(
                "simulateTransaction failed: {err}"
            )));
        }

        Ok(response)
    }

    /// Fetch ledger entries by their XDR keys.
    pub async fn get_ledger_entries(&self, keys: &[String]) -> PrismResult<serde_json::Value> {
        let params = serde_json::json!({ "keys": keys });
        self.call::<serde_json::Value>("getLedgerEntries", params)
            .await
    }

    /// Query events starting from `start_ledger` with the given filters.
    pub async fn get_events(
        &self,
        start_ledger: u32,
        filters: serde_json::Value,
    ) -> PrismResult<serde_json::Value> {
        let params = serde_json::json!({
            "startLedger": start_ledger,
            "filters": filters,
        });
        self.call("getEvents", params).await
    }

    /// Return the latest ledger info from the RPC node.
    pub async fn get_latest_ledger(&self) -> PrismResult<serde_json::Value> {
        self.call("getLatestLedger", serde_json::json!({})).await
    }

    /// Internal JSON-RPC call with retry and rate-limit backoff.
    async fn call<T: for<'de> Deserialize<'de>>(
        &self,
        method: &'static str,
        params: serde_json::Value,
    ) -> PrismResult<T> {
        let request = JsonRpcRequest::new(1, method, params);

        const MAX_RETRIES: u32 = 3;
        let mut last_error: Option<PrismError> = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let backoff = Duration::from_millis(100 * 2u64.pow(attempt));
                tokio::time::sleep(backoff).await;
                tracing::debug!(attempt, method, "Retrying RPC request");
            }

            let started = Instant::now();
            tracing::debug!(method, endpoint = %self.rpc_url, attempt, "Sending RPC request");

            match self.client.post(&self.rpc_url).json(&request).send().await {
                Ok(response) => {
                    let status = response.status();
                    let elapsed_ms = started.elapsed().as_millis();
                    tracing::info!(
                        method,
                        endpoint = %self.rpc_url,
                        attempt,
                        %status,
                        elapsed_ms,
                        "RPC request latency"
                    );

                    let body = response.text().await.map_err(|e| {
                        PrismError::RpcError(format!("Failed to read response body: {e}"))
                    })?;

                    tracing::debug!(
                        method,
                        endpoint = %self.rpc_url,
                        attempt,
                        %status,
                        elapsed_ms,
                        "RPC response received"
                    );

                    if status == 429 {
                        tracing::warn!(method, "Rate limited by RPC node, backing off");
                        last_error =
                            Some(PrismError::RpcError("Rate limited (HTTP 429)".to_string()));
                        continue;
                    }

                    if !status.is_success() {
                        return Err(PrismError::RpcError(format!(
                            "RPC request failed with HTTP {}: {}",
                            status, body
                        )));
                    }
                    let rpc_response: JsonRpcResponse<T> = serde_json::from_str(&body)
                        .map_err(|e| PrismError::RpcError(format!("Response parse error: {e}")))?;

                    if let Some(err) = rpc_response.error {
                        tracing::debug!(
                            method,
                            endpoint = %self.rpc_url,
                            attempt,
                            error = %err.message,
                            code = err.code,
                            "RPC returned an error response"
                        );
                        return Err(PrismError::JsonRpc(err));
                    }

                    return rpc_response.result.ok_or_else(|| {
                        PrismError::RpcError("Empty result in RPC response".into())
                    });
                }
                Err(e) => {
                    let elapsed_ms = started.elapsed().as_millis();
                    tracing::info!(
                        method,
                        endpoint = %self.rpc_url,
                        attempt,
                        elapsed_ms,
                        error = %e,
                        "RPC request latency"
                    );

                    tracing::debug!(
                        method,
                        endpoint = %self.rpc_url,
                        attempt,
                        elapsed_ms,
                        error = %e,
                        "RPC request failed"
                    );
                    last_error = Some(PrismError::RpcError(format!("HTTP request failed: {e}")));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| PrismError::RpcError("Unknown RPC error".into())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_transaction_response_deserializes() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "status": "SUCCESS",
                "latestLedger": 123,
                "latestLedgerCloseTime": 1711620000,
                "ledger": 120,
                "createdAt": "2024-03-28T10:00:00Z",
                "applicationOrder": 1,
                "envelopeXdr": "AAAAAg...",
                "resultXdr": "AAAAAw...",
                "resultMetaXdr": "AAAABA..."
            }
        }"#;

        let resp: JsonRpcResponse<GetTransactionResponse> = serde_json::from_str(json).unwrap();
        let result = resp.result.unwrap();

        assert_eq!(result.status, TransactionStatus::Success);
        assert_eq!(result.latest_ledger, 123);
        assert_eq!(result.ledger, Some(120));
    }

    #[test]
    fn transaction_status_variants_deserialize() {
        let cases = [
            ("\"SUCCESS\"", TransactionStatus::Success),
            ("\"NOT_FOUND\"", TransactionStatus::NotFound),
            ("\"FAILED\"", TransactionStatus::Failed),
        ];

        for (raw, expected) in cases {
            let got: TransactionStatus = serde_json::from_str(raw).unwrap();
            assert_eq!(got, expected);
        }
    }

    #[test]
    fn test_simulate_response_is_success() {
        let ok = SimulateTransactionResponse {
            latest_ledger: 100,
            soroban_data: Some("AAAA".to_string()),
            min_resource_fee: Some("1000".to_string()),
            auth: vec![],
            results: vec![],
            error: None,
            events: vec![],
            cost: None,
        };
        assert!(ok.is_success());

        let err = SimulateTransactionResponse {
            error: Some("contract trap".to_string()),
            ..ok
        };
        assert!(!err.is_success());
    }

    #[test]
    fn test_simulate_response_deserialization() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "latestLedger": 200,
                "transactionData": "AAAAXDR=",
                "minResourceFee": "5000",
                "auth": ["AUTHXDR="],
                "results": [{"xdr": "RETVAL=", "auth": []}],
                "events": []
            }
        }"#;

        let resp: JsonRpcResponse<SimulateTransactionResponse> =
            serde_json::from_str(json).unwrap();
        let result = resp.result.unwrap();

        assert_eq!(result.latest_ledger, 200);
        assert_eq!(result.soroban_data.as_deref(), Some("AAAAXDR="));
        assert_eq!(result.min_resource_fee.as_deref(), Some("5000"));
        assert_eq!(result.auth, vec!["AUTHXDR="]);
        assert_eq!(result.return_value_xdr(), Some("RETVAL="));
        assert!(result.is_success());
    }

    #[test]
    fn test_get_transaction_success_status() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "status": "SUCCESS",
                "latestLedger": 500,
                "latestLedgerCloseTime": 1711620000,
                "oldestLedger": 100,
                "oldestLedgerCloseTime": 1711610000,
                "ledger": 450,
                "createdAt": "2024-03-28T10:00:00Z",
                "applicationOrder": 2,
                "envelopeXdr": "AAAAAgAAAABqYWNrQGV4YW1wbGUuY29tAAABkA==",
                "resultXdr": "AAAAAAAAAGQAAAAAAAAAAQAAAAAAAAABAAAAAAAAAAA=",
                "resultMetaXdr": "AAAAAwAAAAAAAAACAAAAAwAAAcQAAAAAAAAAAA=="
            }
        }"#;

        let resp: JsonRpcResponse<GetTransactionResponse> = serde_json::from_str(json).unwrap();
        let result = resp.result.unwrap();

        assert_eq!(result.status, TransactionStatus::Success);
        assert_eq!(result.latest_ledger, 500);
        assert_eq!(result.latest_ledger_close_time, Some(1711620000));
        assert_eq!(result.oldest_ledger, Some(100));
        assert_eq!(result.oldest_ledger_close_time, Some(1711610000));
        assert_eq!(result.ledger, Some(450));
        assert_eq!(result.created_at, Some("2024-03-28T10:00:00Z".to_string()));
        assert_eq!(result.application_order, Some(2));
        assert_eq!(result.envelope_xdr, Some("AAAAAgAAAABqYWNrQGV4YW1wbGUuY29tAAABkA==".to_string()));
        assert_eq!(result.result_xdr, Some("AAAAAAAAAGQAAAAAAAAAAQAAAAAAAAABAAAAAAAAAAA=".to_string()));
        assert_eq!(result.result_meta_xdr, Some("AAAAAwAAAAAAAAACAAAAAwAAAcQAAAAAAAAAAA==".to_string()));
    }

    #[test]
    fn test_get_transaction_not_found_status() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "status": "NOT_FOUND",
                "latestLedger": 600,
                "latestLedgerCloseTime": 1711625000,
                "oldestLedger": 200,
                "oldestLedgerCloseTime": 1711615000
            }
        }"#;

        let resp: JsonRpcResponse<GetTransactionResponse> = serde_json::from_str(json).unwrap();
        let result = resp.result.unwrap();

        assert_eq!(result.status, TransactionStatus::NotFound);
        assert_eq!(result.latest_ledger, 600);
        assert_eq!(result.latest_ledger_close_time, Some(1711625000));
        assert_eq!(result.oldest_ledger, Some(200));
        assert_eq!(result.oldest_ledger_close_time, Some(1711615000));
        assert_eq!(result.ledger, None);
        assert_eq!(result.created_at, None);
        assert_eq!(result.application_order, None);
        assert_eq!(result.envelope_xdr, None);
        assert_eq!(result.result_xdr, None);
        assert_eq!(result.result_meta_xdr, None);
    }

    #[test]
    fn test_get_transaction_failed_status() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "status": "FAILED",
                "latestLedger": 700,
                "latestLedgerCloseTime": 1711630000,
                "oldestLedger": 300,
                "oldestLedgerCloseTime": 1711620000,
                "ledger": 650,
                "createdAt": "2024-03-28T11:00:00Z",
                "applicationOrder": 5,
                "envelopeXdr": "AAAAAgAAAABmYWlsZWRAdHguY29tAAABkA==",
                "resultXdr": "AAAAAAAAAGT////7AAAAAA==",
                "resultMetaXdr": "AAAAAwAAAAAAAAACAAAAAwAAAoYAAAAAAAAAAA=="
            }
        }"#;

        let resp: JsonRpcResponse<GetTransactionResponse> = serde_json::from_str(json).unwrap();
        let result = resp.result.unwrap();

        assert_eq!(result.status, TransactionStatus::Failed);
        assert_eq!(result.latest_ledger, 700);
        assert_eq!(result.latest_ledger_close_time, Some(1711630000));
        assert_eq!(result.oldest_ledger, Some(300));
        assert_eq!(result.oldest_ledger_close_time, Some(1711620000));
        assert_eq!(result.ledger, Some(650));
        assert_eq!(result.created_at, Some("2024-03-28T11:00:00Z".to_string()));
        assert_eq!(result.application_order, Some(5));
        assert_eq!(result.envelope_xdr, Some("AAAAAgAAAABmYWlsZWRAdHguY29tAAABkA==".to_string()));
        assert_eq!(result.result_xdr, Some("AAAAAAAAAGT////7AAAAAA==".to_string()));
        assert_eq!(result.result_meta_xdr, Some("AAAAAwAAAAAAAAACAAAAAwAAAoYAAAAAAAAAAA==".to_string()));
    }

    #[tokio::test]
    async fn test_get_ledger_entries_empty_response() {
        use tokio::io::AsyncWriteExt;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let rpc_url = format!("http://{}", addr);

        let config = NetworkConfig {
            network: crate::network::Network::Testnet,
            rpc_url,
            network_passphrase: "test".to_string(),
            archive_urls: vec![],
            api_key: None,
            request_timeout_secs: 30,
        };
        let client = SorobanRpcClient::new(&config);

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let body = r#"{"jsonrpc":"2.0","id":1,"result":{"latestLedger":123,"entries":[]}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });

        let result = client.get_ledger_entries(&["key1".to_string()]).await.unwrap();
        assert_eq!(result["entries"].as_array().unwrap().len(), 0);
        assert_eq!(result["latestLedger"], 123);
    }

    #[tokio::test]
    async fn test_client_respects_timeout() {
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let rpc_url = format!("http://{}", addr);

        // Set a 1s timeout in config
        let config = NetworkConfig {
            network: crate::network::Network::Testnet,
            rpc_url,
            network_passphrase: "test".to_string(),
            archive_urls: vec![],
            api_key: None,
            request_timeout_secs: 1,
        };
        let client = SorobanRpcClient::new(&config);

        tokio::spawn(async move {
            while let Ok((_socket, _)) = listener.accept().await {
                // Sleep for 2s to trigger timeout
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });

        let result = client.get_latest_ledger().await;
        
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        println!("Error message: {}", err_msg);
        // reqwest timeout error message usually contains "timeout"
        assert!(
            err_msg.to_lowercase().contains("timeout") || err_msg.to_lowercase().contains("error sending request"),
            "Actual error: {}", err_msg
        );
    }
}

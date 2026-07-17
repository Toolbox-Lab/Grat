#!/usr/bin/env node

/**
 * Mock Replay Submitter
 * 
 * Interacts with the grat-server API to submit a transaction hash for simulation/replay
 * and polls the API for the job's completion status.
 */

const txHash = process.argv[2];
if (!txHash) {
  console.error("Error: Please provide a transaction hash.");
  console.error("Usage: npm start <transaction_hash>");
  process.exit(1);
}

const API_BASE = process.env.API_URL || "http://localhost:3001";
const TIMEOUT_MS = Number(process.env.TIMEOUT_MS) || 60000; // absolute timeout (default 60s)
const MAX_ITERATIONS = Number(process.env.MAX_ITERATIONS) || 50; // max iterations (default 50)

async function run() {
  console.log(`Submitting transaction hash for replay: ${txHash}`);

  try {
    const submitResponse = await fetch(`${API_BASE}/api/replay`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ txHash, network: "mainnet" }),
    });

    if (!submitResponse.ok) {
      throw new Error(`Failed to submit replay job: HTTP ${submitResponse.status} ${submitResponse.statusText}`);
    }

    const submitData = await submitResponse.json();
    console.log("Job submission response:", submitData);

    const jobId = submitData.jobId || submitData.id || txHash;
    console.log(`Starting polling loop for Job ID: ${jobId}`);

    const startTime = Date.now();
    let delay = 1000;
    let iteration = 0;

    while (true) {
      // Overarching absolute timeout check
      if (Date.now() - startTime > TIMEOUT_MS) {
        throw new Error(`Absolute timeout of ${TIMEOUT_MS / 1000} seconds exceeded while polling for job status.`);
      }

      // Maximum iteration counter check
      iteration++;
      if (iteration > MAX_ITERATIONS) {
        throw new Error(`Maximum iteration limit of ${MAX_ITERATIONS} reached while polling for job status.`);
      }

      try {
        const pollResponse = await fetch(`${API_BASE}/api/replay/${jobId}`);
        if (!pollResponse.ok) {
          throw new Error(`HTTP error ${pollResponse.status} ${pollResponse.statusText}`);
        }

        const jobResult = await pollResponse.json();
        const status = jobResult.status || jobResult.state;

        switch (status) {
          case "queued":
          case "running":
            console.log(`Job is ${status}. Retrying in ${delay}ms...`);
            await new Promise((resolve) => setTimeout(resolve, delay));
            // Backoff logic: exponential backoff capped at 5 seconds
            delay = Math.min(delay * 1.5, 5000);
            break;

          case "failed": {
            const errorReason = jobResult.error_reason || "No error reason provided";
            throw new Error(`Job failed: ${errorReason}`);
          }

          case "completed": {
            const results = jobResult.results || jobResult.simulation_results || {};
            console.log("Job completed successfully!");
            console.log("Simulation Results:");
            console.log(JSON.stringify(results, null, 2));
            process.exit(0);
          }

          default:
            // Handle unexpected status values by treating them like queued/running (with backoff)
            console.log(`Unknown job status received: "${status}". Retrying in ${delay}ms...`);
            await new Promise((resolve) => setTimeout(resolve, delay));
            delay = Math.min(delay * 1.5, 5000);
            break;
        }
      } catch (pollError) {
        // Log the polling error and retry with backoff, unless it's a fatal exception thrown inside the switch-case
        if (pollError.message.includes("Job failed:")) {
          throw pollError;
        }
        console.warn(`Error during status polling iteration ${iteration}: ${pollError.message}. Retrying in ${delay}ms...`);
        await new Promise((resolve) => setTimeout(resolve, delay));
        delay = Math.min(delay * 1.5, 5000);
      }
    }
  } catch (error) {
    console.error("Fatal Exception:", error.message);
    process.exit(1);
  }
}

run();

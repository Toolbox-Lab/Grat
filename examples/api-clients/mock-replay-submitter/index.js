#!/usr/bin/env node

const API_URL = process.env.API_URL || 'http://localhost:3001';
const REPLAY_ENDPOINT = `${API_URL}/api/replay`;

/**
 * Submits a transaction hash to the grat-server replay API and returns the
 * job token used to track simulation progress.
 *
 * Resilient against:
 * - Fastify 415 Unsupported Media Type (explicit Content-Type/Accept headers)
 * - Network-level failures — DNS errors, connection refused, server downtime
 *   (fetch rejections are caught here instead of propagating as unhandled
 *   Promise rejections that would crash the process)
 * - Non-2xx responses (4xx/5xx), surfaced as descriptive errors
 * - Malformed or unexpected JSON response bodies
 */
async function submitReplayJob(txHash) {
  let response;
  try {
    response = await fetch(REPLAY_ENDPOINT, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json',
      },
      body: JSON.stringify({ tx_hash: txHash }),
    });
  } catch (err) {
    throw new Error(
      `Could not reach grat-server at ${REPLAY_ENDPOINT}: ${err.message}. ` +
        'Is the server running? (pnpm --filter grat-server dev)'
    );
  }

  let payload;
  try {
    payload = await response.json();
  } catch (err) {
    throw new Error(
      `Server returned a non-JSON response (HTTP ${response.status} ${response.statusText}): ${err.message}`
    );
  }

  if (!response.ok) {
    const detail = payload && (payload.error || payload.message);
    throw new Error(
      `Replay submission failed with HTTP ${response.status} ${response.statusText}` +
        (detail ? `: ${detail}` : '')
    );
  }

  if (!payload || !payload.jobId) {
    throw new Error(
      `Replay submission succeeded (HTTP ${response.status}) but the response did not include a "jobId": ${JSON.stringify(
        payload
      )}`
    );
  }

  return payload.jobId;
}

/**
 * Polls the server using exponential backoff to check the status of a replay job.
 * 
 * @param {string} jobId The ID of the job to poll.
 * @param {number} currentDelay The delay before the next poll request in milliseconds.
 * @returns {Promise<object>} The final job status payload.
 */
function pollJobStatus(jobId, currentDelay = 500) {
  return new Promise((resolve, reject) => {
    const executePoll = async () => {
      try {
        const response = await fetch(`${REPLAY_ENDPOINT}/${jobId}`, {
          headers: { Accept: 'application/json' },
        });

        if (!response.ok) {
          console.warn(`[Poll Warning] Replay status fetch failed with HTTP ${response.status}`);
          scheduleNext();
          return;
        }

        let payload;
        try {
          payload = await response.json();
        } catch (err) {
          console.warn(`[Poll Warning] Non-JSON response: ${err.message}`);
          scheduleNext();
          return;
        }

        const status = payload.status;
        const pendingStatuses = ['queued', 'pending', 'running', 'waiting', 'active'];

        if (pendingStatuses.includes(status)) {
          scheduleNext();
        } else {
          // Job reached terminal state (e.g. completed, failed, error)
          resolve(payload);
        }
      } catch (err) {
        // Handle network/request errors gracefully without crashing
        console.warn(`[Poll Warning] Network/request error during poll: ${err.message}`);
        scheduleNext();
      }
    };

    const scheduleNext = () => {
      // Double the delay for the next poll, capped at 5000ms
      const nextDelay = Math.min(currentDelay * 2, 5000);
      pollJobStatus(jobId, nextDelay).then(resolve).catch(reject);
    };

    // Wait currentDelay before executing the poll
    setTimeout(executePoll, currentDelay);
  });
}

async function main() {
  const txHash = process.argv[2];

  if (!txHash) {
    console.error('Usage: node index.js <tx-hash>');
    process.exitCode = 1;
    return;
  }

  console.log(`Submitting replay job for ${txHash} to ${REPLAY_ENDPOINT}...`);

  try {
    const jobId = await submitReplayJob(txHash);
    console.log(`✓ Replay job accepted. jobId: ${jobId}`);

    console.log(`Polling for job status...`);
    const finalResult = await pollJobStatus(jobId);
    console.log(`✓ Job finished with status: ${finalResult.status}`);
  } catch (err) {
    console.error(`✗ ${err.message}`);
    process.exitCode = 1;
  }
}

if (require.main === module) {
  main();
}

module.exports = { submitReplayJob, pollJobStatus };

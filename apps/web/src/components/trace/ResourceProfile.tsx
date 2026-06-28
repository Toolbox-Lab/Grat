interface ResourceProfileProps {
  profile: {
    cpu_used: number;
    memory_used: number;
    cpu_limit: number;
    memory_limit: number;
    total_read_bytes?: number;
    read_bytes_limit?: number;
  };
}

export function ResourceProfile({ profile }: ResourceProfileProps) {
  const cpuPercentage = (profile.cpu_used / profile.cpu_limit) * 100;
  const memoryPercentage = (profile.memory_used / profile.memory_limit) * 100;
  const readBytes = profile.total_read_bytes ?? 0;
  const readLimit = profile.read_bytes_limit ?? 0;
  const readPercentage = readLimit > 0 ? (readBytes / readLimit) * 100 : 0;

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <h2 className="text-xl font-semibold mb-4">Resource Profile</h2>

      <div className="space-y-6">
        <div>
          <div className="flex justify-between mb-2">
            <span className="text-sm font-medium">CPU Instructions</span>
            <span className="text-sm text-gray-600">
              {profile.cpu_used.toLocaleString()} /{" "}
              {profile.cpu_limit.toLocaleString()} ({cpuPercentage.toFixed(1)}%)
            </span>
          </div>
          <div className="w-full bg-gray-200 rounded-full h-4">
            <div
              className={`h-4 rounded-full transition-all duration-300 ${
                cpuPercentage > 90
                  ? "bg-red-500"
                  : cpuPercentage > 70
                    ? "bg-yellow-500"
                    : "bg-green-500"
              }`}
              style={{ width: `${Math.min(cpuPercentage, 100)}%` }}
            />
          </div>
        </div>

        <div>
          <div className="flex justify-between mb-2">
            <span className="text-sm font-medium">Memory</span>
            <span className="text-sm text-gray-600">
              {(profile.memory_used / 1024 / 1024).toFixed(2)} MB /{" "}
              {(profile.memory_limit / 1024 / 1024).toFixed(2)} MB (
              {memoryPercentage.toFixed(1)}%)
            </span>
          </div>
          <div className="w-full bg-gray-200 rounded-full h-4">
            <div
              className={`h-4 rounded-full transition-all duration-300 ${
                memoryPercentage > 90
                  ? "bg-red-500"
                  : memoryPercentage > 70
                    ? "bg-yellow-500"
                    : "bg-green-500"
              }`}
              style={{ width: `${Math.min(memoryPercentage, 100)}%` }}
            />
          </div>
        </div>

        <div>
          <div className="flex justify-between mb-2">
            <span className="text-sm font-medium">Ledger Read Bytes</span>
            <span className="text-sm text-gray-600">
              {readBytes.toLocaleString()} / {readLimit.toLocaleString()} bytes
              {readLimit > 0 && ` (${readPercentage.toFixed(1)}%)`}
            </span>
          </div>
          <div className="w-full bg-gray-200 rounded-full h-4">
            <div
              className={`h-4 rounded-full transition-all duration-300 ${
                readPercentage > 90
                  ? "bg-red-500"
                  : readPercentage > 70
                    ? "bg-yellow-500"
                    : "bg-green-500"
              }`}
              style={{ width: `${Math.min(readPercentage, 100)}%` }}
            />
          </div>
        </div>
      </div>

      {(cpuPercentage > 90 || memoryPercentage > 90 || readPercentage > 90) && (
        <div className="mt-4 p-3 bg-yellow-50 border border-yellow-200 rounded">
          <p className="text-sm text-yellow-800">
            ⚠ Resource usage is approaching limits. Consider optimizing contract
            code.
          </p>
        </div>
      )}
    </div>
  );
}

export default ResourceProfile;

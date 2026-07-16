import { execFile } from "child_process";

export function callGrat(args: string[]): Promise<unknown> {
  return new Promise((resolve, reject) => {
    execFile("grat", [...args, "--output", "json"], (error, stdout) => {
      if (error) return reject(error);
      try {
        resolve(JSON.parse(stdout));
      } catch {
        reject(new Error("Failed to parse grat output"));
      }
    });
  });
}

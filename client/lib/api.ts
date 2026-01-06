import type { JobListResponse, JobResponse } from "./types";

const defaultBase = "http://localhost:8080";

export const apiBase = () =>
  process.env.NEXT_PUBLIC_API_BASE?.replace(/\/$/, "") ?? defaultBase;

export async function listJobs(): Promise<JobListResponse> {
  const response = await fetch(`${apiBase()}/jobs`, {
    cache: "no-store",
  });
  if (!response.ok) {
    throw new Error("Failed to load jobs.");
  }
  return response.json();
}

export async function getJob(id: string): Promise<JobResponse> {
  const response = await fetch(`${apiBase()}/jobs/${id}`, {
    cache: "no-store",
  });
  if (!response.ok) {
    throw new Error("Failed to load job.");
  }
  return response.json();
}

export async function createJob(jobType: string, payload: unknown): Promise<JobResponse> {
  const response = await fetch(`${apiBase()}/jobs`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ job_type: jobType, payload }),
  });
  if (!response.ok) {
    const body = await response.json().catch(() => ({ error: "Request failed" }));
    throw new Error(body.error ?? "Request failed");
  }
  return response.json();
}

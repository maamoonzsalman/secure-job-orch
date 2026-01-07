"use client";

import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import { createJob, listJobs } from "@/lib/api";
import type { JobRecord } from "@/lib/types";

const defaultPayload = `{
  "task": "sum",
  "values": [1, 2, 3, 4]
}`;

const badgeStyles: Record<string, string> = {
  SUBMITTED: "bg-slate-900/80 text-white",
  VALIDATED: "bg-amber-200 text-amber-950",
  PROCESSING: "bg-sky-200 text-sky-900",
  VERIFIED: "bg-emerald-200 text-emerald-950",
  FAILED: "bg-rose-200 text-rose-950",
  UNVERIFIED: "bg-slate-200 text-slate-900",
  VALID: "bg-emerald-200 text-emerald-950",
  INVALID: "bg-rose-200 text-rose-950",
};

function Badge({ label }: { label: string }) {
  const style = badgeStyles[label] ?? "bg-slate-200 text-slate-900";
  return (
    <span className={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-wide ${style}`}>
      {label}
    </span>
  );
}

function formatTime(value: string) {
  return new Date(value).toLocaleString();
}

export default function Home() {
  const [jobType, setJobType] = useState("deterministic_sample");
  const [payloadText, setPayloadText] = useState(defaultPayload);
  const [jobs, setJobs] = useState<JobRecord[]>([]);
  const [publicKey, setPublicKey] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const parsedPayload = useMemo(() => {
    try {
      return JSON.parse(payloadText);
    } catch {
      return null;
    }
  }, [payloadText]);

  const refresh = async () => {
    const response = await listJobs();
    setJobs(response.jobs);
    setPublicKey(response.public_key);
  };

  useEffect(() => {
    refresh().catch((err) => setError(err.message));
  }, []);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(null);
    setStatus(null);

    if (!parsedPayload) {
      setError("Payload must be valid JSON.");
      return;
    }

    setLoading(true);
    try {
      const response = await createJob(jobType, parsedPayload);
      setStatus(`Job ${response.job.id} verified as ${response.job.verification_status}.`);
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Submission failed.");
    } finally {
      setLoading(false);
    }
  };

  return (
    <main className="min-h-screen px-6 py-10">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-10">
        <header
          className="intro flex flex-col gap-6 rounded-3xl border border-slate-200/60 bg-white/70 p-8 shadow-[0_20px_60px_-45px_rgba(15,23,42,0.5)] backdrop-blur"
          style={{ animationDelay: "40ms" }}
        >
          <div className="flex flex-wrap items-center justify-between gap-6">
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.4em] text-slate-500">
                Secure Job Orchestrator
              </p>
              <h1 className="mt-3 text-4xl font-semibold text-slate-900 md:text-5xl">
                Verifiable jobs with clear trust boundaries.
              </h1>
            </div>
            <div className="rounded-2xl border border-slate-200 bg-slate-950 px-5 py-4 text-xs text-slate-200">
              <p className="font-semibold text-slate-300">Control Plane</p>
              <p className="mt-1 text-sm text-slate-100">Rust + RustCrypto</p>
              <p className="mt-3 font-semibold text-slate-300">Compute Module</p>
              <p className="mt-1 text-sm text-slate-100">C++ deterministic runner</p>
            </div>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <span className="rounded-full border border-slate-200 bg-white px-4 py-2 text-xs font-semibold text-slate-700">
              Supabase persistence only
            </span>
            <span className="rounded-full border border-slate-200 bg-white px-4 py-2 text-xs font-semibold text-slate-700">
              Ed25519 signatures
            </span>
            <span className="rounded-full border border-slate-200 bg-white px-4 py-2 text-xs font-semibold text-slate-700">
              SHA-256 payload hashing
            </span>
            <span className="rounded-full border border-slate-200 bg-white px-4 py-2 text-xs font-semibold text-slate-700">
              Deterministic output verification
            </span>
          </div>
        </header>

        <section
          className="intro grid gap-6 lg:grid-cols-[1.1fr_0.9fr]"
          style={{ animationDelay: "120ms" }}
        >
          <form
            onSubmit={handleSubmit}
            className="flex h-full flex-col gap-5 rounded-3xl border border-slate-200/60 bg-white/80 p-8 shadow-[0_20px_60px_-45px_rgba(15,23,42,0.35)]"
          >
            <div className="flex items-center justify-between">
              <h2 className="text-xl font-semibold text-slate-900">Submit a job</h2>
              <Badge label={parsedPayload ? "VALID" : "INVALID"} />
            </div>
            <label className="text-sm font-semibold text-slate-700">Job type</label>
            <input
              value={jobType}
              onChange={(event) => setJobType(event.target.value)}
              className="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm font-medium text-slate-900 outline-none transition focus:border-slate-400"
              placeholder="deterministic_sample"
            />
            <label className="text-sm font-semibold text-slate-700">Payload (JSON)</label>
            <textarea
              value={payloadText}
              onChange={(event) => setPayloadText(event.target.value)}
              className="min-h-[220px] resize-none rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-900 outline-none focus:border-slate-400"
            />
            <div className="flex flex-wrap items-center gap-3">
              <button
                type="submit"
                disabled={loading}
                className="rounded-2xl bg-slate-900 px-5 py-3 text-sm font-semibold text-white transition hover:bg-slate-800 disabled:cursor-not-allowed disabled:bg-slate-400"
              >
                {loading ? "Submitting..." : "Submit & verify"}
              </button>
              <button
                type="button"
                onClick={() => refresh().catch((err) => setError(err.message))}
                className="rounded-2xl border border-slate-200 px-5 py-3 text-sm font-semibold text-slate-700 transition hover:border-slate-300"
              >
                Refresh list
              </button>
              {status ? <span className="text-sm font-medium text-emerald-700">{status}</span> : null}
              {error ? <span className="text-sm font-medium text-rose-700">{error}</span> : null}
            </div>
          </form>

          <div className="flex flex-col gap-4 rounded-3xl border border-slate-200/60 bg-white/80 p-8 shadow-[0_20px_60px_-45px_rgba(15,23,42,0.35)]">
            <h2 className="text-xl font-semibold text-slate-900">Integrity metadata</h2>
            <div className="rounded-2xl border border-slate-200 bg-slate-50 p-4">
              <p className="text-xs font-semibold uppercase tracking-[0.25em] text-slate-500">Backend public key</p>
              <p className="mt-3 break-all text-xs text-slate-800 mono">
                {publicKey ?? "Load jobs to fetch the public key."}
              </p>
            </div>
            <div className="rounded-2xl border border-slate-200 bg-slate-50 p-4">
              <p className="text-xs font-semibold uppercase tracking-[0.25em] text-slate-500">Verification rule</p>
              <p className="mt-3 text-sm text-slate-700">
                Rust recomputes the deterministic output and verifies the FNV-1a checksum
                from the C++ module before marking a job verified.
              </p>
            </div>
            <div className="rounded-2xl border border-slate-200 bg-slate-50 p-4">
              <p className="text-xs font-semibold uppercase tracking-[0.25em] text-slate-500">Trust boundary</p>
              <p className="mt-3 text-sm text-slate-700">
                C++ output is treated as untrusted until Rust validates signatures, hashes,
                and the deterministic checksum.
              </p>
            </div>
          </div>
        </section>

        <section
          className="intro rounded-3xl border border-slate-200/60 bg-white/85 p-8 shadow-[0_20px_60px_-45px_rgba(15,23,42,0.3)]"
          style={{ animationDelay: "200ms" }}
        >
          <div className="flex flex-wrap items-center justify-between gap-4">
            <h2 className="text-xl font-semibold text-slate-900">Jobs</h2>
            <span className="text-xs font-semibold uppercase tracking-[0.3em] text-slate-500">
              {jobs.length} total
            </span>
          </div>
          <div className="mt-6 grid gap-4">
            {jobs.length === 0 ? (
              <p className="text-sm text-slate-600">No jobs yet. Submit one to see the lifecycle.</p>
            ) : (
              jobs.map((job) => (
                <div
                  key={job.id}
                  className="flex flex-col gap-4 rounded-2xl border border-slate-200 bg-white p-5 md:flex-row md:items-center md:justify-between"
                >
                  <div>
                    <div className="flex flex-wrap items-center gap-2">
                      <p className="text-sm font-semibold text-slate-900">{job.job_type}</p>
                      <Badge label={job.state} />
                      <Badge label={job.verification_status} />
                    </div>
                    <p className="mt-2 text-xs text-slate-500">{job.id}</p>
                    <p className="mt-2 text-xs text-slate-500">Updated {formatTime(job.updated_at)}</p>
                  </div>
                  <div className="flex flex-wrap items-center gap-3">
                    <Link
                      href={`/jobs/${job.id}`}
                      className="rounded-full border border-slate-200 px-4 py-2 text-xs font-semibold text-slate-700 transition hover:border-slate-300"
                    >
                      View details
                    </Link>
                  </div>
                </div>
              ))
            )}
          </div>
        </section>
      </div>
    </main>
  );
}

"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { getJob } from "@/lib/api";
import type { JobRecord } from "@/lib/types";

const badgeStyles: Record<string, string> = {
  SUBMITTED: "bg-slate-900/80 text-white",
  VALIDATED: "bg-amber-200 text-amber-950",
  PROCESSING: "bg-sky-200 text-sky-900",
  VERIFIED: "bg-emerald-200 text-emerald-950",
  FAILED: "bg-rose-200 text-rose-950",
  UNVERIFIED: "bg-slate-200 text-slate-900",
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

export default function JobDetailClient({ id }: { id: string }) {
  const [job, setJob] = useState<JobRecord | null>(null);
  const [publicKey, setPublicKey] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let isActive = true;
    getJob(id)
      .then((response) => {
        if (!isActive) return;
        setJob(response.job);
        setPublicKey(response.public_key);
      })
      .catch(() => {
        if (!isActive) return;
        setError("Job not found.");
      });
    return () => {
      isActive = false;
    };
  }, [id]);

  if (error) {
    return (
      <main className="min-h-screen px-6 py-10">
        <div className="mx-auto flex w-full max-w-3xl flex-col gap-6">
          <h1 className="text-2xl font-semibold text-slate-900">Job detail</h1>
          <p className="text-sm text-rose-700">{error}</p>
          <Link
            href="/"
            className="w-fit rounded-full border border-slate-200 px-4 py-2 text-xs font-semibold text-slate-700 transition hover:border-slate-300"
          >
            Back to jobs
          </Link>
        </div>
      </main>
    );
  }

  if (!job) {
    return (
      <main className="min-h-screen px-6 py-10">
        <div className="mx-auto flex w-full max-w-3xl flex-col gap-6">
          <h1 className="text-2xl font-semibold text-slate-900">Job detail</h1>
          <p className="text-sm text-slate-600">Loading job details...</p>
        </div>
      </main>
    );
  }

  return (
    <main className="min-h-screen px-6 py-10">
      <div className="mx-auto flex w-full max-w-5xl flex-col gap-8">
        <div className="intro flex flex-wrap items-center justify-between gap-4">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.3em] text-slate-500">Job detail</p>
            <h1 className="mt-3 text-3xl font-semibold text-slate-900">{job.job_type}</h1>
            <p className="mt-2 text-xs text-slate-500">{job.id}</p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Badge label={job.state} />
            <Badge label={job.verification_status} />
          </div>
        </div>

        <section className="intro grid gap-6 lg:grid-cols-[1.1fr_0.9fr]" style={{ animationDelay: "80ms" }}>
          <div className="rounded-3xl border border-slate-200/60 bg-white/85 p-6 shadow-[0_20px_60px_-45px_rgba(15,23,42,0.3)]">
            <h2 className="text-lg font-semibold text-slate-900">Payload</h2>
            <pre className="mt-4 max-h-[320px] overflow-auto rounded-2xl border border-slate-200 bg-slate-50 p-4 text-xs text-slate-700">
              {JSON.stringify(job.payload, null, 2)}
            </pre>
          </div>
          <div className="flex flex-col gap-4 rounded-3xl border border-slate-200/60 bg-white/85 p-6 shadow-[0_20px_60px_-45px_rgba(15,23,42,0.3)]">
            <div className="rounded-2xl border border-slate-200 bg-slate-50 p-4">
              <p className="text-xs font-semibold uppercase tracking-[0.25em] text-slate-500">Payload hash</p>
              <p className="mt-3 break-all text-xs text-slate-800 mono">{job.payload_hash}</p>
            </div>
            <div className="rounded-2xl border border-slate-200 bg-slate-50 p-4">
              <p className="text-xs font-semibold uppercase tracking-[0.25em] text-slate-500">Backend signature</p>
              <p className="mt-3 break-all text-xs text-slate-800 mono">{job.backend_signature}</p>
            </div>
            <div className="rounded-2xl border border-slate-200 bg-slate-50 p-4">
              <p className="text-xs font-semibold uppercase tracking-[0.25em] text-slate-500">Public key</p>
              <p className="mt-3 break-all text-xs text-slate-800 mono">
                {publicKey ?? "Unavailable"}
              </p>
            </div>
            <div className="rounded-2xl border border-slate-200 bg-slate-50 p-4">
              <p className="text-xs font-semibold uppercase tracking-[0.25em] text-slate-500">Timestamps</p>
              <p className="mt-3 text-xs text-slate-700">Created {formatTime(job.created_at)}</p>
              <p className="mt-1 text-xs text-slate-700">Updated {formatTime(job.updated_at)}</p>
            </div>
          </div>
        </section>

        <Link
          href="/"
          className="w-fit rounded-full border border-slate-200 px-4 py-2 text-xs font-semibold text-slate-700 transition hover:border-slate-300"
        >
          Back to jobs
        </Link>
      </div>
    </main>
  );
}

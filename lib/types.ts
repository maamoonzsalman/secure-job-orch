export type JobState =
  | "SUBMITTED"
  | "VALIDATED"
  | "PROCESSING"
  | "VERIFIED"
  | "FAILED";

export type VerificationStatus =
  | "UNVERIFIED"
  | "VERIFIED"
  | "INVALID"
  | "FAILED";

export interface JobRecord {
  id: string;
  job_type: string;
  payload: unknown;
  state: JobState;
  payload_hash: string;
  backend_signature: string;
  verification_status: VerificationStatus;
  created_at: string;
  updated_at: string;
}

export interface JobResponse {
  job: JobRecord;
  public_key: string;
}

export interface JobListResponse {
  jobs: JobRecord[];
  public_key: string;
}

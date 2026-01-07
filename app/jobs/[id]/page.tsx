import JobDetailClient from "./job-detail-client";

export function generateStaticParams() {
  return [{ id: "sample" }];
}

export default function JobDetailPage({ params }: { params: { id: string } }) {
  return <JobDetailClient id={params.id} />;
}

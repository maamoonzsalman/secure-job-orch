import JobDetailClient from "./job-detail-client";

export function generateStaticParams() {
  return [{ id: "sample" }];
}

export default async function JobDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  return <JobDetailClient id={id} />;
}

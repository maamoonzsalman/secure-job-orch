import type { Metadata } from "next";
import { IBM_Plex_Mono, Space_Grotesk } from "next/font/google";
import "./globals.css";

export const metadata: Metadata = {
  title: "Secure Job Orchestrator",
  description: "A verifiable job control plane built with Rust, C++, and Supabase.",
};

const spaceGrotesk = Space_Grotesk({
  variable: "--font-sans",
  display: "swap",
  subsets: ["latin"],
});

const plexMono = IBM_Plex_Mono({
  variable: "--font-mono",
  display: "swap",
  weight: ["400", "600"],
  subsets: ["latin"],
});

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className={`${spaceGrotesk.variable} ${plexMono.variable}`}>
        {children}
      </body>
    </html>
  );
}

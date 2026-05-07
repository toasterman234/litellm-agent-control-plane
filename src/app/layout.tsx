import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";

import { TopNav } from "@/components/top-nav";

import "./globals.css";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "LiteLLM Agents",
  description: "Build, run, and observe agents on LiteLLM.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      className={`${geistSans.variable} ${geistMono.variable} h-full`}
    >
      <body className="flex h-full flex-col bg-background text-foreground antialiased">
        <TopNav />
        <main className="flex flex-1 min-h-0 flex-col overflow-y-auto">
          {children}
        </main>
      </body>
    </html>
  );
}

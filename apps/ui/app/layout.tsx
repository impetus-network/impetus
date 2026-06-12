import "./globals.css";
import type { Metadata } from "next";
import { headers } from "next/headers";
import { Web3Provider } from "~/components/providers/Web3Provider";
import { AppLayout } from "~/components/layout/AppLayout";
import { Inter, Geist_Mono } from "next/font/google";
import { cn } from "~/lib/utils";

const geistMono = Geist_Mono({subsets:['latin'],variable:'--font-mono'});

const interHeading = Inter({subsets:['latin'],variable:'--font-heading'});

const inter = Inter({subsets:['latin'],variable:'--font-sans'});

export const metadata: Metadata = {
  title: "Impetus Explorer",
  description: "Block explorer and contract debugger for Impetus chain",
};

export default async function RootLayout({ children }: { children: React.ReactNode }) {
  const cookie = (await headers()).get("cookie");

  return (
    <html lang="en" className={cn("font-mono", inter.variable, interHeading.variable, geistMono.variable)}>
      <body className="min-h-screen bg-background text-foreground">
        <Web3Provider cookie={cookie}>
          <AppLayout>{children}</AppLayout>
        </Web3Provider>
      </body>
    </html>
  );
}

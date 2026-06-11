import type { ReactNode } from "react";
import { Header } from "@/components/layout/header";
import { Footer } from "@/components/layout/footer";
import { TRPCProvider } from "@/trpc/client";
import "./globals.css";

export const dynamic = "force-dynamic";

export const metadata = {
  title: "Artemis Explorer",
  description: "Block explorer for the Artemis chain",
};

export default function RootLayout({
  children,
}: {
  children: ReactNode;
}) {
  return (
    <html lang="en">
      <body className="flex min-h-screen flex-col bg-white text-gray-900 antialiased">
        <TRPCProvider>
          <Header />
          <div className="flex-1">{children}</div>
          <Footer />
        </TRPCProvider>
      </body>
    </html>
  );
}

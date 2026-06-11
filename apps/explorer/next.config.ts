import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: "standalone",
  reactStrictMode: true,
  transpilePackages: ["@artemis/shared", "@artemis/coss-ui"],
  experimental: {
    useCache: true,
  },
};

export default nextConfig;

// Single indexer: the Subsquid (Squid SDK) GraphQL (OpenReader). Serves
// staking, pools, validators, transfers, and gasless rules. The hook appends
// `/graphql`. Default is the local squid graphql-server port (4350).
export const SQUID_URL = process.env.NEXT_PUBLIC_SQUID_URL || "http://localhost:4350";

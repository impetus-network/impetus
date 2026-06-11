import "server-only";
import { appRouter } from "./routers/_app";
import { createContext } from "./trpc";

export async function getServerCaller() {
  return appRouter.createCaller(createContext());
}

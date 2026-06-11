"use client";

export default function ErrorPage({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  return (
    <main className="mx-auto flex max-w-7xl flex-col items-center justify-center px-4 py-20">
      <h2 className="text-2xl font-bold">Something went wrong</h2>
      <p className="mt-2 text-gray-600">{error.message}</p>
      <button
        onClick={reset}
        className="mt-4 rounded-lg bg-gray-900 px-4 py-2 text-white hover:bg-gray-800"
      >
        Try again
      </button>
    </main>
  );
}

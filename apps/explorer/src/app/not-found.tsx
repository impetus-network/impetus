import Link from "next/link";

export default function NotFound() {
  return (
    <main className="mx-auto flex max-w-7xl flex-col items-center justify-center px-4 py-20">
      <h2 className="text-2xl font-bold">Not Found</h2>
      <p className="mt-2 text-gray-600">
        The block, transaction, or address you are looking for does not exist.
      </p>
      <Link
        href="/"
        className="mt-4 rounded-lg bg-gray-900 px-4 py-2 text-white hover:bg-gray-800"
      >
        Go home
      </Link>
    </main>
  );
}

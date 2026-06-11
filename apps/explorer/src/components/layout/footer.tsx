import { Separator } from "@artemis/coss-ui/ui/separator";

export function Footer() {
  return (
    <footer className="mt-auto">
      <Separator />
      <div className="mx-auto max-w-7xl px-4 py-6">
        <div className="flex items-center justify-between text-sm text-gray-500">
          <span>Artemis Explorer</span>
          <div className="flex items-center gap-4">
            <span>Chain ID: 322</span>
            <span>Token: ART</span>
          </div>
        </div>
      </div>
    </footer>
  );
}

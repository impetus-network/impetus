"use client";

interface BlockieAvatarProps {
  address: string;
  size?: number;
}

export function BlockieAvatar({ address, size = 24 }: BlockieAvatarProps) {
  const hue = parseInt(address.slice(2, 8), 16) % 360;
  return (
    <div
      className="rounded-full"
      style={{
        width: size,
        height: size,
        background: `linear-gradient(135deg, hsl(${hue}, 70%, 60%), hsl(${(hue + 60) % 360}, 70%, 40%))`,
      }}
    />
  );
}

import { useEffect, useState } from 'react';

const cache = new Map<string, string>();

async function loadIcon(name: string, color: string): Promise<string> {
  const key = `${name}:${color}`;
  const cached = cache.get(key);
  if (cached) return cached;
  const url = new URL(`../assets/icons/${name}.svg`, import.meta.url);
  const raw = await Deno.readTextFile(url);
  const themed = raw.replaceAll('currentColor', color);
  const dataUrl = `data:image/svg+xml;base64,${btoa(themed)}`;
  cache.set(key, dataUrl);
  return dataUrl;
}

export function Icon({
  name,
  color,
  size = 16,
}: {
  name: string;
  color: string;
  size?: number;
}) {
  const [src, setSrc] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    loadIcon(name, color).then((url) => {
      if (active) setSrc(url);
    });
    return () => {
      active = false;
    };
  }, [name, color]);

  if (!src) return <view w={size} h={size} />;
  return <image src={src} w={size} h={size} />;
}

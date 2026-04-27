const cache = new Map<string, string>();

function loadIcon(name: string, color: string): string {
  const key = `${name}:${color}`;
  const cached = cache.get(key);
  if (cached) return cached;
  // todo find a better way to load this
  const url = new URL(`../assets/icons/${name}.svg`, import.meta.url);
  const raw = Deno.readTextFileSync(url);
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
  const src = loadIcon(name, color);
  return <image src={src} w={size} h={size} />;
}

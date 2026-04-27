export function Icon({
  name,
  color,
  size = 16,
}: {
  name: string;
  color: string;
  size?: number;
}) {
  const src = new URL(`../assets/icons/${name}.svg`, import.meta.url).href;
  return <image src={src} w={size} h={size} color={color} />;
}

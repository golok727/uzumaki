export function Icon({
  name,
  color,
  size = 16,
}: {
  name: string;
  color: string;
  size?: number;
}) {
  const src = Uz.path.resource(`assets/icons/${name}.svg`);
  return <image src={src} w={size} h={size} color={color} />;
}

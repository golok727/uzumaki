import type { UzumakiMouseEvent } from 'uzumaki-ui';
import { ACTIVE_BG, HOVER_BG, NAV_ACTIVE, TEXT_COLOR } from './styles';

export function Button({
  onClick,
  children,
  ...props
}: {
  onClick?: (event: UzumakiMouseEvent) => void;
  children?: React.ReactNode;
  [key: string]: any;
}) {
  return (
    <button
      onClick={onClick}
      color={TEXT_COLOR}
      p="8"
      px="16"
      bg={NAV_ACTIVE}
      rounded="6"
      hover:bg={HOVER_BG}
      active:bg={ACTIVE_BG}
      {...props}
    >
      {children}
    </button>
  );
}

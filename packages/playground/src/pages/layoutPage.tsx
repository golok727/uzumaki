import { useState } from 'react';
import { C } from '../theme';
import { Divider } from '../components';

function AbsClickCounter({ color }: { color: string }) {
  const [count, setCount] = useState(0);
  return (
    <button
      onClick={() => setCount((c) => c + 1)}
      p={7}
      bg={color}
      hover:bg={C.surface4}
      rounded={4}
      cursor="pointer"
      display="flex"
      items="center"
      gap={6}
    >
      <text fontSize={11} fontWeight={600} color={C.text}>
        {count}
      </text>
    </button>
  );
}

function AbsolutePositioningDemo() {
  return (
    <view display="flex" flexDir="col" gap={12}>
      <view display="flex" flexDir="col" gap={4}>
        <text fontSize={14} fontWeight={700} color={C.text}>
          Absolute positioning
        </text>
        <text fontSize={12} color={C.textMuted}>
          position="absolute" with top / right / bottom / left insets
        </text>
      </view>

      <view display="flex" flexDir="row" gap={12}>
        {(
          [
            { label: 'top + left', pos: { top: 8, left: 8 }, c: C.accentDim },
            {
              label: 'top + right',
              pos: { top: 8, right: 8 },
              c: C.primaryDim,
            },
            {
              label: 'bottom + right',
              pos: { bottom: 8, right: 8 },
              c: C.successDim,
            },
            {
              label: 'bottom + left',
              pos: { bottom: 8, left: 8 },
              c: C.warningDim,
            },
          ] as const
        ).map(({ label, pos, c }) => (
          <view
            key={label}
            flex={1}
            minW={0}
            h={120}
            position="relative"
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
            display="flex"
            items="center"
            justify="center"
          >
            <text fontSize={10} color={C.textDim}>
              {label}
            </text>
            <view
              position="absolute"
              {...pos}
              bg={c}
              hover:bg={'#ff0000'}
              rounded={6}
              w={10}
              h={10}
              p={6}
              display="flex"
              flexDir="col"
              gap={4}
            ></view>
          </view>
        ))}
      </view>

      <view
        h={140}
        position="relative"
        bg={C.surface2}
        rounded={8}
        border={1}
        borderColor={C.border}
        display="flex"
        items="center"
        justify="center"
      >
        <text fontSize={12} color={C.textDim}>
          relative container bg
        </text>
        <view
          position="absolute"
          top={10}
          left={10}
          right={10}
          bottom={10}
          bg="#e2a52e10"
          rounded={8}
          display="flex"
          items="center"
          justify="center"
        >
          <AbsClickCounter color={C.accentDim} />
        </view>
      </view>
    </view>
  );
}

export function LayoutPage() {
  const [showVisibility, setShowVisibility] = useState(false);
  const [showDisplay, setShowDisplay] = useState(false);
  const [gap, setGap] = useState(8);
  const [padding, setPadding] = useState(12);

  return (
    <view display="flex" flexDir="col" gap={0} h="full" scroll>
      <view
        display="flex"
        flexDir="col"
        px={24}
        py={16}
        borderBottom={1}
        borderColor={C.border}
        gap={8}
      >
        <view fontSize={20} fontWeight={800} color={C.text}>
          Layout Lab
        </view>
        <view fontSize={12} color={C.textMuted}>
          Flex, nesting, borders, rounding, opacity, visibility
        </view>
      </view>

      <view display="flex" flexDir="col" gap={24} p={24}>
        <view display="flex" flexDir="col" gap={12}>
          <text fontSize={14} fontWeight={700} color={C.text}>
            Flexbox — justify variants
          </text>
          {(['center', 'between', 'around', 'evenly'] as const).map((j) => (
            <view key={j} display="flex" flexDir="col" gap={4}>
              <text fontSize={11} fontWeight={600} color={C.textMuted}>
                justify="{j}"
              </text>
              <view
                display="flex"
                flexDir="row"
                justify={j}
                bg={C.surface2}
                rounded={8}
                p={12}
                border={1}
                borderColor={C.border}
              >
                {[C.accentHi, C.primaryHi, C.successHi, C.warningHi].map(
                  (c, i) => (
                    <view key={i} w={36} h={36} bg={c} rounded={4} />
                  ),
                )}
              </view>
            </view>
          ))}
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={12}>
          <text fontSize={14} fontWeight={700} color={C.text}>
            Flexbox — items variants
          </text>
          <view display="flex" flexDir="row" gap={12}>
            {(['start', 'center', 'end', 'stretch'] as const).map((a) => (
              <view key={a} display="flex" flexDir="col" gap={4} flex={1}>
                <text fontSize={11} fontWeight={600} color={C.textMuted}>
                  items="{a}"
                </text>
                <view
                  display="flex"
                  flexDir="row"
                  items={a}
                  bg={C.surface2}
                  rounded={8}
                  p={10}
                  h={70}
                  border={1}
                  borderColor={C.border}
                  gap={4}
                >
                  {a === 'stretch' ? (
                    <>
                      <view w={24} bg={C.accentHi} rounded={4} />
                      <view w={24} bg={C.primaryHi} rounded={4} />
                      <view w={24} bg={C.successHi} rounded={4} />
                    </>
                  ) : (
                    <>
                      <view w={24} h={24} bg={C.accentHi} rounded={4} />
                      <view w={24} h={36} bg={C.primaryHi} rounded={4} />
                      <view w={24} h={16} bg={C.successHi} rounded={4} />
                    </>
                  )}
                </view>
              </view>
            ))}
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={12}>
          <text fontSize={14} fontWeight={700} color={C.text}>
            Per-corner border-radius
          </text>
          <view display="flex" flexDir="row" gap={12} items="center">
            <view display="flex" flexDir="col" items="center" gap={4}>
              <view w={60} h={60} bg={C.accent} roundedTL={24} />
              <text fontSize={10} color={C.textMuted}>
                TL
              </text>
            </view>
            <view display="flex" flexDir="col" items="center" gap={4}>
              <view w={60} h={60} bg={C.primary} roundedTR={24} />
              <text fontSize={10} color={C.textMuted}>
                TR
              </text>
            </view>
            <view display="flex" flexDir="col" items="center" gap={4}>
              <view w={60} h={60} bg={C.success} roundedBR={24} />
              <text fontSize={10} color={C.textMuted}>
                BR
              </text>
            </view>
            <view display="flex" flexDir="col" items="center" gap={4}>
              <view w={60} h={60} bg={C.warning} roundedBL={24} />
              <text fontSize={10} color={C.textMuted}>
                BL
              </text>
            </view>
            <view display="flex" flexDir="col" items="center" gap={4}>
              <view w={60} h={60} bg={C.accent} roundedTL={24} roundedBR={24} />
              <text fontSize={10} color={C.textMuted}>
                TL+BR
              </text>
            </view>
            <view display="flex" flexDir="col" items="center" gap={4}>
              <view w={60} h={60} bg={C.danger} roundedTR={24} roundedBL={24} />
              <text fontSize={10} color={C.textMuted}>
                TR+BL
              </text>
            </view>
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={12}>
          <text fontSize={14} fontWeight={700} color={C.text}>
            Per-side borders
          </text>
          <view display="flex" flexDir="row" gap={12} items="center">
            {[
              { side: 'Top', prop: { borderTop: 3 }, color: C.accentHi },
              { side: 'Right', prop: { borderRight: 3 }, color: C.primaryHi },
              { side: 'Bottom', prop: { borderBottom: 3 }, color: C.successHi },
              { side: 'Left', prop: { borderLeft: 3 }, color: C.warningHi },
              { side: 'All', prop: { border: 2 }, color: C.accentHi },
            ].map(({ side, prop, color }) => (
              <view
                key={side}
                display="flex"
                flexDir="col"
                items="center"
                gap={4}
              >
                <view
                  w={60}
                  h={60}
                  bg={C.surface2}
                  rounded={8}
                  borderColor={color}
                  {...prop}
                />
                <text fontSize={10} color={C.textMuted}>
                  {side}
                </text>
              </view>
            ))}
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={12}>
          <text fontSize={14} fontWeight={700} color={C.text}>
            Opacity scale
          </text>
          <view display="flex" flexDir="row" gap={8} items="center">
            {[1, 0.8, 0.6, 0.4, 0.2, 0.1].map((op) => (
              <view
                key={op}
                display="flex"
                flexDir="col"
                items="center"
                gap={4}
              >
                <view
                  w={52}
                  h={52}
                  bg={C.accent}
                  rounded={8}
                  opacity={op}
                  display="flex"
                  items="center"
                  justify="center"
                >
                  <text fontSize={11} fontWeight={700} color="#fff">
                    {op}
                  </text>
                </view>
              </view>
            ))}
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={12}>
          <view display="flex" flexDir="col" gap={4}>
            <text fontSize={14} fontWeight={700} color={C.text}>
              Transforms
            </text>
            <text fontSize={12} color={C.textMuted}>
              translate, rotate, scale, and hover:scale without changing layout
            </text>
          </view>
          <view
            display="flex"
            flexDir="row"
            gap={16}
            p={18}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
          >
            {[
              {
                label: 'hover scale',
                props: { scale: 1, 'hover:scale': 1.16 },
                color: C.accent,
              },
              {
                label: 'rotated',
                props: { rotate: -8, 'hover:rotate': 8 },
                color: C.primary,
              },
              {
                label: 'translated',
                props: { translate: [0, 0], 'hover:translate': [10, -8] },
                color: C.success,
              },
            ].map(({ label, props, color }) => (
              <view
                key={label}
                w={120}
                h={72}
                bg={color}
                rounded={8}
                cursor="pointer"
                display="flex"
                items="center"
                justify="center"
                {...(props as any)}
              >
                <text fontSize={12} fontWeight={800} color="#ffffff">
                  {label}
                </text>
              </view>
            ))}
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={12}>
          <view display="flex" flexDir="row" items="center" gap={20}>
            <text fontSize={14} fontWeight={700} color={C.text}>
              Dynamic gap / padding
            </text>
            <view display="flex" flexDir="row" items="center" gap={8}>
              <button
                onClick={() => setGap((g) => Math.max(2, g - 2))}
                px={10}
                py={4}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={4}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} color={C.text}>
                  gap−
                </text>
              </button>
              <text fontSize={12} color={C.accentHi}>
                gap={gap}
              </text>
              <button
                onClick={() => setGap((g) => Math.min(40, g + 2))}
                px={10}
                py={4}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={4}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} color={C.text}>
                  gap+
                </text>
              </button>
              <button
                onClick={() => setPadding((p) => Math.max(4, p - 4))}
                px={10}
                py={4}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={4}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} color={C.text}>
                  p−
                </text>
              </button>
              <text fontSize={12} color={C.primaryHi}>
                p={padding}
              </text>
              <button
                onClick={() => setPadding((p) => Math.min(40, p + 4))}
                px={10}
                py={4}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={4}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} color={C.text}>
                  p+
                </text>
              </button>
            </view>
          </view>
          <view
            display="flex"
            flexDir="row"
            gap={gap}
            p={padding}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
          >
            {['A', 'B', 'C', 'D', 'E'].map((l, i) => (
              <view
                key={l}
                flex={1}
                p={padding}
                bg={
                  [
                    C.accentDim,
                    C.primaryDim,
                    C.successDim,
                    '#422006',
                    C.dangerDim,
                  ][i]
                }
                rounded={8}
                display="flex"
                items="center"
                justify="center"
              >
                <text fontSize={16} fontWeight={800} color={C.text}>
                  {l}
                </text>
              </view>
            ))}
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={12}>
          <view display="flex" flexDir="col" gap={4}>
            <text fontSize={14} fontWeight={700} color={C.text}>
              Buttons
            </text>
            <text fontSize={12} color={C.textMuted}>
              Various button configurations and property combinations
            </text>
          </view>

          <view display="flex" flexDir="col" gap={12}>
            <view display="flex" flexDir="row" gap={12}>
              <view
                display="flex"
                flexDir="col"
                gap={8}
                flex={1}
                p={12}
                bg={C.surface2}
                rounded={12}
                border={1}
                borderColor={C.border}
              >
                <view display="flex" flexDir="col" gap={2}>
                  <text fontSize={13} fontWeight={600} color={C.accentHi}>
                    Default
                  </text>
                  <text fontSize={11} color={C.textMuted}>
                    No properties set
                  </text>
                </view>
                <view
                  display="flex"
                  items="center"
                  justify="center"
                  p={8}
                  bg={C.surface}
                  rounded={8}
                >
                  <button bg={C.accent} cursor="pointer" hover:bg={C.accentDim}>
                    button text
                  </button>
                </view>
              </view>

              <view
                display="flex"
                flexDir="col"
                gap={8}
                flex={1}
                p={12}
                bg={C.surface2}
                rounded={12}
                border={1}
                borderColor={C.border}
              >
                <view display="flex" flexDir="col" gap={2}>
                  <text fontSize={13} fontWeight={600} color={C.accentHi}>
                    With Padding
                  </text>
                  <text fontSize={11} color={C.textMuted}>
                    px: 12 | py: 6
                  </text>
                </view>
                <view
                  display="flex"
                  items="center"
                  justify="center"
                  p={8}
                  bg={C.surface}
                  rounded={8}
                >
                  <button
                    px={12}
                    py={6}
                    bg={C.accent}
                    cursor="pointer"
                    hover:bg={C.accentDim}
                  >
                    button text
                  </button>
                </view>
              </view>

              <view
                display="flex"
                flexDir="col"
                gap={8}
                flex={1}
                p={12}
                bg={C.surface2}
                rounded={12}
                border={1}
                borderColor={C.border}
              >
                <view display="flex" flexDir="col" gap={2}>
                  <text fontSize={13} fontWeight={600} color={C.accentHi}>
                    Uniform Padding
                  </text>
                  <text fontSize={11} color={C.textMuted}>
                    padding: 12
                  </text>
                </view>
                <view
                  display="flex"
                  items="center"
                  justify="center"
                  p={8}
                  bg={C.surface}
                  rounded={8}
                >
                  <button
                    p={12}
                    bg={C.accent}
                    cursor="pointer"
                    hover:bg={C.accentDim}
                  >
                    button text
                  </button>
                </view>
              </view>
            </view>

            <view display="flex" flexDir="row" gap={12}>
              <view
                display="flex"
                flexDir="col"
                gap={8}
                flex={1}
                p={12}
                bg={C.surface2}
                rounded={12}
                border={1}
                borderColor={C.border}
              >
                <view display="flex" flexDir="col" gap={2}>
                  <text fontSize={13} fontWeight={600} color={C.accentHi}>
                    Rounded
                  </text>
                  <text fontSize={11} color={C.textMuted}>
                    rounded: 8 | px: 12 | py: 6
                  </text>
                </view>
                <view
                  display="flex"
                  items="center"
                  justify="center"
                  p={8}
                  bg={C.surface}
                  rounded={8}
                >
                  <button
                    rounded={8}
                    px={12}
                    py={6}
                    bg={C.accent}
                    cursor="pointer"
                    hover:bg={C.accentDim}
                  >
                    button text
                  </button>
                </view>
              </view>

              <view
                display="flex"
                flexDir="col"
                gap={8}
                flex={1}
                p={12}
                bg={C.surface2}
                rounded={12}
                border={1}
                borderColor={C.border}
              >
                <view display="flex" flexDir="col" gap={2}>
                  <text fontSize={13} fontWeight={600} color={C.accentHi}>
                    Flex Centered
                  </text>
                  <text fontSize={11} color={C.textMuted}>
                    flex | px: 12 | py: 6 | rounded: 8
                  </text>
                </view>
                <view
                  display="flex"
                  items="center"
                  justify="center"
                  p={8}
                  bg={C.surface}
                  rounded={8}
                >
                  <button
                    display="flex"
                    flexDir="row"
                    justify="center"
                    rounded={8}
                    px={12}
                    py={6}
                    bg={C.accent}
                    cursor="pointer"
                    hover:bg={C.accentDim}
                  >
                    button text
                  </button>
                </view>
              </view>

              <view flex={1} />
            </view>
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={12}>
          <text fontSize={14} fontWeight={700} color={C.text}>
            Deep nesting (6 levels)
          </text>
          <view
            p={16}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
          >
            <view
              p={14}
              bg={C.surface3}
              rounded={8}
              border={1}
              borderColor={C.borderHi}
            >
              <view
                p={12}
                bg={C.surface4}
                rounded={8}
                border={1}
                borderColor={C.primaryDim}
              >
                <view
                  p={10}
                  bg={C.accentDark}
                  rounded={8}
                  border={1}
                  borderColor={C.accent}
                >
                  <view
                    p={8}
                    bg={C.accentDim}
                    rounded={8}
                    border={1}
                    borderColor={C.accentHi}
                  >
                    <view
                      p={6}
                      bg={C.accent}
                      rounded={4}
                      display="flex"
                      items="center"
                      justify="center"
                    >
                      <text fontSize={12} fontWeight={700} color="#fff">
                        6 levels
                      </text>
                    </view>
                  </view>
                </view>
              </view>
            </view>
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={10}>
          <view display="flex" flexDir="row" items="center" gap={8}>
            <text fontSize={14} fontWeight={700} color={C.text}>
              Cursor kinds
            </text>
          </view>
          <view display="flex" flexDir="row" gap={8}>
            {(
              [
                'default',
                'pointer',
                'text',
                'crosshair',
                'not-allowed',
                'grab',
              ] as const
            ).map((cur) => (
              <view
                key={cur}
                px={14}
                py={10}
                bg={C.surface2}
                hover:bg={C.surface3}
                active:bg={C.surface4}
                rounded={8}
                border={1}
                borderColor={C.border}
                hover:borderColor={C.accentHi}
                cursor={cur}
              >
                <text fontSize={12} color={C.textDim} hover:color={C.text}>
                  {cur}
                </text>
              </view>
            ))}
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={10}>
          <view display="flex" flexDir="row" items="center" gap={12}>
            <text fontSize={14} fontWeight={700} color={C.text}>
              display prop
            </text>
            <button
              onClick={() => setShowDisplay((s) => !s)}
              px={14}
              py={6}
              bg={showDisplay ? C.accentDim : C.surface3}
              hover:bg={showDisplay ? C.accent : C.surface4}
              rounded={8}
              border={1}
              borderColor={showDisplay ? C.accent : C.border}
              cursor="pointer"
            >
              <text
                fontSize={12}
                fontWeight={600}
                color={showDisplay ? C.accentHi : C.textMuted}
              >
                {showDisplay ? 'Hide it' : 'Show it'}
              </text>
            </button>
          </view>
          <view
            display={showDisplay ? 'flex' : 'none'}
            p={14}
            bg={C.primaryDark}
            rounded={8}
            border={1}
            borderColor={C.primary}
          >
            <text fontSize={14} color={C.primaryHi} fontWeight={600}>
              👁 Now you see me via display!
            </text>
          </view>
          <view
            display={'flex'}
            p={14}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
          >
            <text fontSize={14} color={C.textMuted}>
              Click the button to toggle with display.
            </text>
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={10}>
          <view display="flex" flexDir="row" items="center" gap={12}>
            <text fontSize={14} fontWeight={700} color={C.text}>
              visible prop
            </text>
            <button
              onClick={() => setShowVisibility((s) => !s)}
              px={14}
              py={6}
              bg={showVisibility ? C.accentDim : C.surface3}
              hover:bg={showVisibility ? C.accent : C.surface4}
              rounded={8}
              border={1}
              borderColor={showVisibility ? C.accent : C.border}
              cursor="pointer"
            >
              <text
                fontSize={12}
                fontWeight={600}
                color={showVisibility ? C.accentHi : C.textMuted}
              >
                {showVisibility ? 'Hide it' : 'Reveal it'}
              </text>
            </button>
          </view>
          <view
            visibility={showVisibility ? 'visible' : 'hidden'}
            p={14}
            bg={C.accentDark}
            rounded={8}
            border={1}
            borderColor={C.accent}
          >
            <text fontSize={14} color={C.accentHi} fontWeight={600}>
              👁 Now you see me! (visibility)
            </text>
          </view>
          <view
            p={14}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
          >
            <text fontSize={14} color={C.textMuted}>
              Click the button to toggle visibility.
            </text>
          </view>
        </view>

        <Divider />

        <ScrollDemo />

        <Divider />

        <AbsolutePositioningDemo />
      </view>
    </view>
  );
}

function ScrollDemo() {
  return (
    <view display="flex" flexDir="col" gap={12}>
      <view display="flex" flexDir="col" gap={4}>
        <text fontSize={14} fontWeight={700} color={C.text}>
          Scroll props
        </text>
        <text fontSize={12} color={C.textMuted}>
          `scroll` enables auto overflow on both axes, while `scrollX` and
          `scrollY` let you opt into each axis separately.
        </text>
      </view>

      <view display="flex" flexDir="col" gap={16}>
        <view display="flex" flexDir="col" gap={8}>
          <text fontSize={12} fontWeight={600} color={C.textMuted}>
            scroll
          </text>
          <view
            h={200}
            scroll
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
            p={12}
            display="flex"
            flexDir="col"
            gap={8}
          >
            {Array.from({ length: 12 }, (_, i) => (
              <view key={i} display="flex" flexDir="row" gap={10}>
                {Array.from({ length: 6 }, (_, j) => (
                  <view
                    key={`${i}-${j}`}
                    w={140}
                    flexShrink={0}
                    p={10}
                    bg={(i + j) % 2 === 0 ? C.surface3 : C.surface4}
                    rounded={6}
                    display="flex"
                    items="center"
                    justify="between"
                  >
                    <text fontSize={12} fontWeight={600} color={C.text}>
                      Row {i + 1}
                    </text>
                    <text fontSize={11} color={C.textMuted}>
                      Col {j + 1}
                    </text>
                  </view>
                ))}
              </view>
            ))}
          </view>
        </view>

        <view display="flex" flexDir="col" gap={8}>
          <text fontSize={12} fontWeight={600} color={C.textMuted}>
            scrollX
          </text>
          <view
            h={108}
            scrollX
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
            p={12}
            display="flex"
            flexDir="row"
            gap={10}
          >
            {Array.from({ length: 10 }, (_, i) => (
              <view
                key={i}
                w={140}
                h={80}
                bg={i % 2 === 0 ? C.primaryDim : C.accentDim}
                rounded={8}
                p={10}
                display="flex"
                flexDir="col"
                textWrap="nowrap"
                justify="between"
              >
                <text fontSize={12} fontWeight={700} color={C.text}>
                  Card #{i + 1}
                </text>
                <text fontSize={11} color={C.textMuted}>
                  Horizontal overflow only
                </text>
              </view>
            ))}
          </view>
        </view>

        <view display="flex" flexDir="col" gap={8}>
          <text fontSize={12} fontWeight={600} color={C.textMuted}>
            scrollY
          </text>
          <view
            h={220}
            scrollY
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
            p={12}
            display="flex"
            flexDir="col"
            gap={10}
          >
            {Array.from({ length: 14 }, (_, i) => (
              <view
                key={i}
                p={12}
                bg={i % 2 === 0 ? C.successDim : C.warningDim}
                rounded={8}
                display="flex"
                flexDir="col"
                gap={4}
              >
                <text fontSize={12} fontWeight={700} color={C.text}>
                  Log entry #{i + 1}
                </text>
                <text fontSize={10} color={C.textMuted}>
                  Vertical overflow only
                </text>
              </view>
            ))}
          </view>
        </view>
      </view>
    </view>
  );
}

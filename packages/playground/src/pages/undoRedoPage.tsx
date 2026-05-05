import { useState } from 'react';
import { C } from '../theme';
import { Divider, Badge } from '../components';

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <view display="flex" flexDir="col" gap={8}>
      <text fontSize={13} fontWeight={600} color={C.textSub}>
        {title}
      </text>
      {children}
    </view>
  );
}

export function UndoRedoPage() {
  const [controlled, setControlled] = useState('');
  const [log, setLog] = useState<string[]>([]);

  const pushLog = (msg: string) => setLog((prev) => [...prev.slice(-19), msg]);

  return (
    <view display="flex" flexDir="col" gap={0} h="full" scrollable>
      <view
        display="flex"
        flexDir="col"
        px={24}
        py={16}
        borderBottom={1}
        borderColor={C.border}
      >
        <text fontSize={20} fontWeight={800} color={C.text}>
          Undo / Redo
        </text>
        <text fontSize={12} color={C.textMuted}>
          Ctrl+Z to undo, Ctrl+Y or Ctrl+Shift+Z to redo
        </text>
      </view>

      <view display="flex" flexDir="col" gap={20} p={24}>
        <Section title="Uncontrolled Input">
          <text fontSize={11} color={C.textMuted}>
            Type freely, then undo/redo. No React state; history lives entirely
            in Rust.
          </text>
          <input
            placeholder="Type here and try Ctrl+Z / Ctrl+Y..."
            fontSize={14}
            color={C.text}
            bg={C.surface2}
            p={4}
            rounded={8}
            border={1}
            borderColor={C.border}
            w="full"
          />
        </Section>

        <Divider />

        <Section title="Controlled Input (set_value clears history)">
          <text fontSize={11} color={C.textMuted}>
            Controlled via React state. Each keystroke calls set_value, which
            clears the undo stack, so undo should have no effect here.
          </text>
          <input
            value={controlled}
            onChangeText={setControlled}
            placeholder="Controlled input, undo won't work"
            fontSize={14}
            color={C.text}
            bg={C.surface2}
            p={4}
            rounded={8}
            border={1}
            borderColor={controlled.length > 0 ? C.accent : C.border}
            w="full"
          />
          <view display="flex" flexDir="row" items="center" gap={8}>
            <Badge
              label={`${controlled.length} chars`}
              color={controlled.length > 0 ? C.accentHi : C.textMuted}
              bg={controlled.length > 0 ? C.accentDark : C.surface3}
            />
            <button
              display="flex"
              flexDir="col"
              justify="center"
              onClick={() => setControlled('')}
              px={12}
              h={24}
              bg={C.surface3}
              hover:bg={C.surface4}
              rounded={6}
              border={1}
              borderColor={C.border}
              cursor="pointer"
            >
              <text fontSize={11} color={C.textMuted}>
                Clear
              </text>
            </button>
          </view>
        </Section>

        <Divider />

        <Section title="Multiline Input">
          <text fontSize={11} color={C.textMuted}>
            Multiline uncontrolled input. Try typing across lines, then
            undo/redo.
          </text>
          <input
            multiline
            placeholder={'Line 1\nLine 2\nUndo should restore line by line...'}
            fontSize={14}
            color={C.text}
            bg={C.surface2}
            p={12}
            rounded={8}
            border={1}
            borderColor={C.border}
            w="full"
            h={110}
          />
        </Section>

        <Divider />

        <Section title="Group-Breaking Scenarios">
          <text fontSize={11} color={C.textMuted}>
            Type several chars, move the cursor (arrow keys), then type more.
            Each movement breaks the undo group so Ctrl+Z undoes in segments.
          </text>
          <input
            placeholder="Type, arrow around, type more, then undo..."
            fontSize={14}
            color={C.text}
            bg={C.surface2}
            p={4}
            rounded={8}
            border={1}
            borderColor={C.border}
            w="full"
          />
        </Section>

        <Divider />

        <Section title="Paste / Cut Isolation">
          <text fontSize={11} color={C.textMuted}>
            Paste text (Ctrl+V), then undo. The entire paste should revert in
            one step. Cut (Ctrl+X) with a selection, then undo to restore.
          </text>
          <input
            placeholder="Paste or cut here..."
            fontSize={14}
            color={C.text}
            bg={C.surface2}
            p={4}
            rounded={8}
            border={1}
            borderColor={C.border}
            w="full"
          />
        </Section>

        <Divider />

        <Section title="Event Log">
          <text fontSize={11} color={C.textMuted}>
            Logs inputType from onInput. Look for historyUndo / historyRedo
            events.
          </text>
          <input
            placeholder="Type, undo, redo, then watch the log below"
            fontSize={14}
            color={C.text}
            bg={C.surface2}
            p={4}
            rounded={8}
            border={1}
            borderColor={C.border}
            w="full"
            onInput={(e: any) => {
              const t = e.inputType ?? '(unknown)';
              pushLog(t);
            }}
          />
          <view
            bg={C.surface}
            rounded={8}
            border={1}
            borderColor={C.border}
            p={12}
            h={160}
            scrollable
            display="flex"
            flexDir="col"
            gap={2}
          >
            {log.length === 0 ? (
              <text fontSize={11} color={C.textMuted}>
                No events yet...
              </text>
            ) : (
              log.map((entry, i) => (
                <text
                  key={i}
                  fontSize={11}
                  color={
                    entry.includes('Undo') || entry.includes('Redo')
                      ? C.accentHi
                      : C.textDim
                  }
                >
                  {entry}
                </text>
              ))
            )}
          </view>
          <button
            display="flex"
            flexDir="col"
            justify="center"
            onClick={() => setLog([])}
            px={12}
            h={24}
            bg={C.surface3}
            hover:bg={C.surface4}
            rounded={6}
            border={1}
            borderColor={C.border}
            cursor="pointer"
          >
            <text fontSize={11} color={C.textMuted}>
              Clear Log
            </text>
          </button>
        </Section>
      </view>
    </view>
  );
}

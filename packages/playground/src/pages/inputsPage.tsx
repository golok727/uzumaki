import { useState } from 'react';
import { C } from '../theme';
import { Divider, Badge } from '../components';
import { Icon } from '../icon';

export function InputsPage() {
  const [username, setUsername] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [bio, setBio] = useState('');
  const [search, setSearch] = useState('');
  const [submitted, setSubmitted] = useState(false);

  const [noRounding, setNoRounding] = useState(false);
  const [rounded, setRounded] = useState(true);
  const [circle, setCircle] = useState(false);

  const inputPadding = 8;

  const pwMatch = password === confirm && confirm.length > 0;
  const pwMismatch = confirm.length > 0 && password !== confirm;

  return (
    <view display="flex" flexDir="col" gap={0} h="full" scroll>
      <view
        display="flex"
        flexDir="col"
        px={24}
        py={16}
        borderBottom={1}
        borderColor={C.border}
      >
        <text fontSize={20} fontWeight={800} color={C.text}>
          Input Lab
        </text>
      </view>

      <view display="flex" flexDir="col" gap={20} p={24}>
        <view display="flex" flexDir="col" gap={8}>
          <text fontSize={13} fontWeight={600} color={C.textSub}>
            Search Field
          </text>
          <input
            value={search}
            onValueChange={setSearch}
            placeholder="Search anything... (try IME input)"
            fontSize={15}
            color={C.text}
            bg={C.surface2}
            p={inputPadding}
            rounded={8}
            border={1}
            borderColor={search.length > 0 ? C.accent : C.border}
            w="full"
          />
          <view display="flex" flexDir="row" items="center" gap={8}>
            <Badge
              label={`${search.length} chars`}
              color={search.length > 0 ? C.accentHi : C.textMuted}
              bg={search.length > 0 ? C.accentDark : C.surface3}
            />
            {search.length > 20 && (
              <Badge
                label="LONG QUERY"
                color={C.warningHi}
                bg={C.warningDark}
              />
            )}
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="row" gap={16}>
          <view display="flex" flexDir="col" gap={8} flex={1}>
            <text fontSize={13} fontWeight={600} color={C.textSub}>
              Username
            </text>
            <input
              value={username}
              onValueChange={setUsername}
              placeholder="johndoe"
              fontSize={14}
              color={C.text}
              bg={C.surface2}
              p={inputPadding}
              rounded={8}
              border={1}
              borderColor={C.border}
              w="full"
            />
          </view>
          <view display="flex" flexDir="col" gap={8} flex={1}>
            <text fontSize={13} fontWeight={600} color={C.textSub}>
              Email
            </text>
            <input
              value={email}
              onValueChange={setEmail}
              placeholder="john@example.com"
              fontSize={14}
              color={C.text}
              bg={C.surface2}
              p={inputPadding}
              rounded={8}
              border={1}
              borderColor={C.border}
              w="full"
            />
          </view>
        </view>

        <view display="flex" flexDir="row" gap={16}>
          <view display="flex" flexDir="col" gap={8} flex={1}>
            <text fontSize={13} fontWeight={600} color={C.textSub}>
              Password (secure)
            </text>
            <input
              secure
              value={password}
              onValueChange={setPassword}
              placeholder="Enter password"
              fontSize={14}
              color={C.text}
              bg={C.surface2}
              p={inputPadding}
              rounded={8}
              border={1}
              borderColor={C.border}
              w="full"
            />
          </view>
          <view display="flex" flexDir="col" gap={8} flex={1}>
            <text fontSize={13} fontWeight={600} color={C.textSub}>
              Confirm Password
            </text>
            <input
              secure
              value={confirm}
              onValueChange={setConfirm}
              placeholder="Repeat password"
              fontSize={14}
              color={C.text}
              bg={C.surface2}
              p={inputPadding}
              rounded={8}
              border={1}
              borderColor={
                pwMismatch ? C.danger : (pwMatch ? C.success : C.border)
              }
              w="full"
            />
            {pwMismatch && (
              <text fontSize={11} color={C.danger}>
                Passwords don't match
              </text>
            )}
            {pwMatch && (
              <>
                <view gap={2}>
                  <text fontSize={11} color={C.success}>
                    Passwords match
                  </text>
                  <Icon name="check" color={C.success} size={12} />
                </view>
              </>
            )}
          </view>
        </view>

        <view display="flex" flexDir="col" gap={8}>
          <view display="flex" flexDir="row" items="center" justify="between">
            <text fontSize={13} fontWeight={600} color={C.textSub}>
              Bio (multiline)
            </text>
            <text
              fontSize={11}
              color={bio.length > 200 ? C.danger : C.textMuted}
            >
              {bio.length}/300
            </text>
          </view>
          <input
            multiline
            value={bio}
            onValueChange={setBio}
            placeholder="Tell us about yourself... (multiline input, try pasting long text)"
            fontSize={16}
            color={C.text}
            bg={C.surface2}
            p={12}
            rounded={8}
            border={1}
            borderColor={bio.length > 200 ? C.warning : C.border}
            w="full"
            h={110}
          />
        </view>

        <view display="flex" flexDir="row" gap={8}>
          <button
            display="flex"
            flexDir="col"
            justify="center"
            onClick={() => setSubmitted(true)}
            px={24}
            h={36}
            bg={C.accent}
            hover:bg={C.warning}
            active:bg={C.warningDark}
            rounded={8}
            cursor="pointer"
          >
            <text fontSize={14} fontWeight={700} color="#fff">
              Submit Form
            </text>
          </button>
          <button
            display="flex"
            flexDir="col"
            justify="center"
            onClick={() => {
              setUsername('');
              setEmail('');
              setPassword('');
              setConfirm('');
              setBio('');
              setSearch('');
              setSubmitted(false);
            }}
            px={24}
            h={36}
            bg={C.surface3}
            hover:bg={C.surface4}
            active:bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
            cursor="pointer"
          >
            <text fontSize={14} color={C.textMuted}>
              Reset
            </text>
          </button>
        </view>

        {submitted && (
          <view
            p={16}
            bg={C.successDim}
            rounded={8}
            border={1}
            borderColor={C.success}
            display="flex"
            flexDir="col"
            gap={4}
          >
            <view gap={2}>
              <text fontSize={14} fontWeight={700} color={C.successHi}>
                Form submitted
              </text>
              <Icon name="check" color={C.success} size={12} />
            </view>

            <text fontSize={12} color={C.success}>
              user={username || '(empty)'} · email={email || '(empty)'} · bio=
              {bio.length} chars
            </text>
          </view>
        )}

        <Divider />

        <view display="flex" flexDir="col" gap={12}>
          <text fontSize={14} fontWeight={700} color={C.text}>
            Checkboxes
          </text>
          <view
            display="flex"
            flexDir="col"
            p={16}
            gap={14}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
          >
            <view display="flex" items="center" gap={12}>
              <checkbox
                checked={noRounding}
                onValueChange={setNoRounding}
                bg={C.accent}
                borderColor={noRounding ? C.accent : C.border}
                color="#ffffff"
                w={20}
                h={20}
                hover:opacity={0.9}
              />
              <text fontSize={14} color={C.text}>
                Square checkbox{noRounding ? ' [selected]' : ''}
              </text>
            </view>
            <view display="flex" items="center" gap={12}>
              <checkbox
                checked={rounded}
                onValueChange={setRounded}
                bg={C.success}
                borderColor={rounded ? C.success : C.border}
                color="#08110a"
                rounded={4}
                w={20}
                h={20}
              />
              <text fontSize={14} color={C.text}>
                Rounded checkbox{rounded ? ' [selected]' : ''}
              </text>
            </view>
            <view display="flex" items="center" gap={12}>
              <checkbox
                checked={circle}
                onValueChange={setCircle}
                bg={C.warning}
                borderColor={circle ? C.warning : C.border}
                color="#1b1104"
                rounded={10}
                w={20}
                h={20}
              />
              <text fontSize={14} color={C.text}>
                Circular checkbox{circle ? ' [selected]' : ''}
              </text>
            </view>
          </view>
        </view>
        <Divider />

        <view display="flex" flexDir="col" gap={8}>
          <view display="flex" flexDir="row" items="center" gap={8}>
            <text fontSize={13} fontWeight={600} color={C.textSub}>
              Selectable Text Block
            </text>
          </view>
          <view
            selectable
            w="full"
            p={16}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.borderHi}
          >
            <text fontSize={13} color={C.textDim} w="100%">
              The quick brown fox jumps over the lazy dog. Pack my box with five
              dozen liquor jugs. How valiantly the strong and quick brown fox
              leaps over the sleeping lazy hound dog! Try selecting this text
              with your mouse.
            </text>
          </view>
        </view>
      </view>
    </view>
  );
}

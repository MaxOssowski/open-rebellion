# Message expiry, the Message Index's "clear" (read 2026-11-04)

The manual (p. 79): "Messages are eventually deleted whether or not you
read them, except for agent advice messages."

## The rule

`FUN_0048a2a0` walks the message manager's list (`thunk_FUN_005f5060`).
For each message it asks slot `+0x30`:
- **Nonzero.** The message is unlinked (`FUN_005f54d0`). It runs its slot
  `+0x34` unless `+0x24` bit 0 is set. Then it is destroyed.
- **Zero.** The message stays, and if it is unread (`+0x24` bit `0x10`)
  it adds its category `+0x34` to the unread mask `+0x50`.

After the walk the mask goes to the rail (`FUN_0041db40`, `FUN_0041db20`).
So an expired unread message rests its rail.

The walk does nothing while the manager's `+8` bit `0x10000000` is set.

## When the walk runs

- **On filing.** `FUN_0048a060` runs it after it files a message of kinds
  3, 4, 5 or 8. Kinds 1, 2, 6 and 7 return before it.
- **The Message Index.** `FUN_0048a610(1)` sets the bit, and
  `FUN_0048a610(0)` clears it and walks at once. The calls come through
  `FUN_0041d1f0` → `FUN_004367b0`:
  - `FUN_004665f0` calls it with 1, as the Message Index window is built;
  - `FUN_004664d0` and `FUN_00440f80` call it with 0.

## Slot `+0x30` by class

The message classes' vtables run from `0x65b0b0` to `0x65bc40`. Slot
`+0x30` is one of three things.

### The base rule

Most classes take `Class_0065c538_vt30` (`FUN_004c4fe0`):

```c
return msg[+0x38] + 300 < FUN_004fcee0();
```

The base constructor `FUN_004c4ce0` stamps `+0x38` with `FUN_004fcee0()`.
That is the scheduler's sub-tick, not the day (`agent-advice-list.md`, "The
clock"). A day has the speed's rate of sub-ticks: 600 Very Slow, 60 Slow, 12
Medium, 4 Fast. So a message lasts 300 sub-ticks:

| Speed | Days |
|---|---|
| Very Slow | half a day |
| Slow | 5 |
| Medium | 25 |
| Fast | 75 |

A paused game adds no sub-ticks. This fits the A0 VM's Alliance game
(2026-11-04), whose index then held a single recent message.

### Never expire

These classes take `FUN_006158b0`, which returns 0:

| Class | What it is |
|---|---|
| `0x65b188` | The agent's advice (`FUN_0048b2e0`, backgrounds `0x42f`/`0x430`) |
| `0x65b520` | Future Jedi Discovered and Leia Uses Force (`FUN_0048f1e0`) |
| `0x65b870` | The Assault Summary reports |
| `0x65ba20` | Category bit `0x20`, Chat (`FUN_00498e20` sets `+0x34` from `FUN_00526bd0`, `0x20`) |

### Facility Idle

Class `0x65b260` takes `Class_0065b260_vt30` (`FUN_0048c2b0`). It looks up
the manager the message names (`FUN_004f2f60(+0x20, +0x6c)`). It keeps the
message only while three things hold:
- the manager exists;
- its `+0x58` is 0, its remaining count, so it stands idle;
- its side bits match the message's side `+0x20`.

## Ported (2026-11-04)

- **The log.** `message_log::MessageLog` drops its invented cap of 500
  messages, which had evicted the oldest message, advice included.
- **Expiry.** `MessageLog::expire` is `FUN_0048a2a0`, through
  `GameMessage::expired`, and it waits while the index is open:
  - the Advice rail and Assault Summary reports never expire;
  - `MessageLifetime::Kept` never expires (Future Jedi Discovered);
  - `MessageLifetime::WhileIdle` stands for Facility Idle;
  - any other message expires 300 sub-ticks after it is posted.
- **When it runs.** `push` stamps the message with the log's sub-tick
  (`set_now`, from `GameClock::sub_tick`) and marks an expiry due for a
  message filed on a rail. `expire_if_due` runs it. `set_index_open` is
  `FUN_0048a610`.
- **`main.rs`.** It runs both once per frame, with the Message Index's
  state. Facility Idle stays while the system is still the player's, its
  area's queue is empty and a facility of the area stands there
  (`pictures::idle_yard`).
- **port:** a message that reaches no rail is a port log line. It expires
  on the base rule, but its filing makes no expiry due.

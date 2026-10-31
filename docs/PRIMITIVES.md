# KUI primitives contract

Behavior belongs to `registry/src/primitives`. Presentation belongs to `components`.

## Rules

1. A primitive has no default padding, fill, radius, or typography chrome.
2. Controlled values accept the current value and report the next value through callbacks.
3. Stable `ElementId` values are part of the public contract.
4. Accessibility roles and names belong on the interactive root.
5. Brand color belongs to KUI theme tokens, not primitives.

## MVP primitives

| Module | Responsibility |
| --- | --- |
| `button` | Activation + disabled/selected flags |
| `sizing` | Sm / Md / Lg |

## Install

Primitives usually arrive with a component (`kui add button`). Design-system apps may copy `primitives/` directly.
